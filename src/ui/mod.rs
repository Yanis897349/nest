mod chrome;
mod filters;
mod listings;
mod modal;
mod overlays;
mod terminal;

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
};

use crate::app::{AppState, Screen};

pub use terminal::TerminalGuard;

pub(crate) fn render(frame: &mut Frame<'_>, app: &AppState) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(2),
    ])
    .areas(frame.area());
    chrome::render_header(frame, app, header);
    match app.screen {
        Screen::Loading => chrome::render_centered(frame, body, "Launching managed Chromium…"),
        Screen::Filters => filters::render(frame, app, body),
        Screen::Progress => overlays::render_progress(frame, app, body),
        Screen::Results => listings::render_results(frame, app, body),
        Screen::Detail => listings::render_detail(frame, app, body),
        Screen::Help => overlays::render_help(frame, body),
        Screen::Challenge => overlays::render_challenge(frame, body),
        Screen::Error => overlays::render_error(frame, app, body),
    }
    chrome::render_footer(frame, app, footer);
    if let Some(input) = &app.input {
        modal::render(frame, input, app.input_error.as_deref());
    }
}

#[cfg(test)]
mod tests;
