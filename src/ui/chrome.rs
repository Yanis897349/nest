use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::app::AppState;

pub(super) fn render_header(frame: &mut Frame<'_>, app: &AppState, area: Rect) {
    let selected = app
        .catalog
        .as_ref()
        .map(|catalog| catalog.selected_labels.join(" · "))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Beijing / 北京".into());
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " nest ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  Ziroom search  "),
            Span::styled(selected, Style::default().fg(Color::Yellow)),
        ]))
        .block(Block::default().borders(Borders::ALL)),
        area,
    );
}

pub(super) fn render_footer(frame: &mut Frame<'_>, app: &AppState, area: Rect) {
    let status = if app.busy {
        format!("⏳ {}", app.status)
    } else {
        app.status.clone()
    };
    frame.render_widget(
        Paragraph::new(status)
            .style(Style::default().fg(if app.busy { Color::Yellow } else { Color::Gray }))
            .alignment(Alignment::Left),
        area,
    );
}

pub(super) fn render_centered(frame: &mut Frame<'_>, area: Rect, text: &str) {
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL)),
        area,
    );
}

pub(super) fn centered_rect(width_percent: u16, height_percent: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}
