use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use super::chrome::centered_rect;
use crate::app::{ExportFormat, InputMode};

pub(super) fn render(frame: &mut Frame<'_>, input: &InputMode, input_error: Option<&str>) {
    let area = centered_rect(60, 36, frame.area());
    frame.render_widget(Clear, area);
    let text = match input {
        InputMode::CustomRent {
            min,
            max,
            editing_max,
        } => Text::from(vec![
            Line::from("Custom monthly rent (CNY)"),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!("Min: {min:<10}"),
                    if !editing_max {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                ),
                Span::raw("   "),
                Span::styled(
                    format!("Max: {max:<10}"),
                    if *editing_max {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                ),
            ]),
            Line::from("Tab switches fields; Enter applies; Esc cancels"),
        ]),
        InputMode::Commute { address, minutes } => Text::from(vec![
            Line::from("Company address / 公司地址"),
            Line::from(Span::styled(address, Style::default().fg(Color::Yellow))),
            Line::from(format!("Bus commute limit: {minutes} minutes")),
            Line::from("Arrows change time; Enter chooses Ziroom's first suggestion"),
        ]),
        InputMode::Export { format } => Text::from(vec![
            Line::from("Export all card-level results"),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    " JSON ",
                    if *format == ExportFormat::Json {
                        Style::default().fg(Color::Black).bg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                ),
                Span::raw("   "),
                Span::styled(
                    " CSV ",
                    if *format == ExportFormat::Csv {
                        Style::default().fg(Color::Black).bg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                ),
            ]),
            Line::from("Left/right switches format; Enter writes the file"),
        ]),
    };
    let mut text = text;
    if let Some(error) = input_error {
        text.lines.push(Line::from(""));
        text.lines.push(Line::from(Span::styled(
            error,
            Style::default().fg(Color::LightRed),
        )));
    }
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(" nest ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            ),
        area,
    );
}
