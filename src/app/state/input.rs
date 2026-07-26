use crossterm::event::{KeyCode, KeyEvent};

use super::AppState;
use crate::{
    app::protocol::{BackendRequest, Effect, InputMode},
    export::ExportFormat,
};

impl AppState {
    pub(super) fn handle_input_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        if key.code == KeyCode::Esc {
            self.input = None;
            self.input_error = None;
            return Vec::new();
        }
        self.input_error = None;
        let Some(input) = self.input.as_mut() else {
            return Vec::new();
        };
        match input {
            InputMode::CustomRent {
                min,
                max,
                editing_max,
            } => match key.code {
                KeyCode::Tab | KeyCode::Right | KeyCode::Left => *editing_max = !*editing_max,
                KeyCode::Backspace => {
                    if *editing_max { max } else { min }.pop();
                }
                KeyCode::Char(value) if value.is_ascii_digit() => {
                    let target = if *editing_max { max } else { min };
                    if target.len() < 10 {
                        target.push(value);
                    }
                }
                KeyCode::Enter if !*editing_max => *editing_max = true,
                KeyCode::Enter => {
                    let min_value = match min.parse::<u32>() {
                        Ok(value) => value,
                        Err(_) => {
                            self.input_error = Some("Enter a valid minimum rent".into());
                            return Vec::new();
                        }
                    };
                    let max_value = match max.parse::<u32>() {
                        Ok(value) => value,
                        Err(_) => {
                            self.input_error = Some("Enter a valid maximum rent".into());
                            return Vec::new();
                        }
                    };
                    if min_value > max_value {
                        self.input_error = Some("Minimum rent must not exceed maximum rent".into());
                        return Vec::new();
                    }
                    self.input = None;
                    self.busy = true;
                    return vec![Effect::Backend(BackendRequest::SetCustomRent(
                        min_value, max_value,
                    ))];
                }
                _ => {}
            },
            InputMode::Commute { address, minutes } => match key.code {
                KeyCode::Backspace => {
                    address.pop();
                }
                KeyCode::Char(value) if !value.is_control() && address.chars().count() < 200 => {
                    address.push(value);
                }
                KeyCode::Up | KeyCode::Right => *minutes = next_commute(*minutes, true),
                KeyCode::Down | KeyCode::Left => *minutes = next_commute(*minutes, false),
                KeyCode::Enter => {
                    let trimmed = address.trim();
                    if trimmed.is_empty() {
                        self.input_error = Some("Enter a company address".into());
                        return Vec::new();
                    }
                    let address = trimmed.to_owned();
                    let minutes = *minutes;
                    self.input = None;
                    self.busy = true;
                    return vec![Effect::Backend(BackendRequest::SetCommute(
                        address, minutes,
                    ))];
                }
                _ => {}
            },
            InputMode::Export { format } => match key.code {
                KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                    *format = match *format {
                        ExportFormat::Json => ExportFormat::Csv,
                        ExportFormat::Csv => ExportFormat::Json,
                    };
                }
                KeyCode::Enter => {
                    let format = *format;
                    self.input = None;
                    self.busy = true;
                    self.status = "Writing export…".into();
                    return vec![Effect::Export(format)];
                }
                _ => {}
            },
        }
        Vec::new()
    }

    pub(super) fn open_commute_input(&mut self) {
        self.input = Some(InputMode::Commute {
            address: String::new(),
            minutes: 30,
        });
        self.input_error = None;
    }
}

fn next_commute(current: u8, forward: bool) -> u8 {
    let values = [15, 30, 40, 50, 60];
    let index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(1);
    if forward {
        values[(index + 1).min(values.len() - 1)]
    } else {
        values[index.saturating_sub(1)]
    }
}

#[cfg(test)]
mod tests {
    use super::next_commute;

    #[test]
    fn commute_steps_stay_in_supported_range() {
        assert_eq!(next_commute(15, false), 15);
        assert_eq!(next_commute(15, true), 30);
        assert_eq!(next_commute(50, true), 60);
        assert_eq!(next_commute(60, true), 60);
    }
}
