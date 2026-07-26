use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::chrome::render_centered;
use crate::app::AppState;

pub(super) fn render_progress(frame: &mut Frame<'_>, app: &AppState, area: Rect) {
    let count = app
        .report
        .as_ref()
        .map_or(0, |report| report.listings.len());
    render_centered(
        frame,
        area,
        &format!(
            "Scraping Ziroom sequentially…\n\n{}\n\n{count} listings collected\n\nPress c to cancel",
            app.status
        ),
    );
}

pub(super) fn render_help(frame: &mut Frame<'_>, area: Rect) {
    let help = Text::from(vec![
        Line::from(Span::styled(
            "Keyboard",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("←/→ or Tab     change filter group"),
        Line::from("↑/↓             move within a group/result list"),
        Line::from("Space/Enter     apply the focused site filter"),
        Line::from("p               enter a custom rent range"),
        Line::from("a               enter commute address and duration"),
        Line::from("s / c           scrape all pages / cancel scrape"),
        Line::from("d / o           lazy detail / open verified Ziroom URL"),
        Line::from("e               export JSON or CSV"),
        Line::from("r               return/reset filters"),
        Line::from("? / q           help / quit"),
    ]);
    frame.render_widget(
        Paragraph::new(help)
            .wrap(Wrap { trim: true })
            .block(Block::default().title(" Help ").borders(Borders::ALL)),
        area,
    );
}

pub(super) fn render_challenge(frame: &mut Frame<'_>, area: Rect) {
    render_centered(
        frame,
        area,
        "Ziroom's browser challenge did not clear headlessly.\n\nPress Enter to retry in a visible temporary browser.\nResolve any challenge there, then return here.\n\nEsc or q cancels. CAPTCHAs are never automated.",
    );
}

pub(super) fn render_error(frame: &mut Frame<'_>, app: &AppState, area: Rect) {
    frame.render_widget(
        Paragraph::new(app.status.as_str())
            .style(Style::default().fg(Color::LightRed))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center)
            .block(Block::default().title(" Error ").borders(Borders::ALL)),
        area,
    );
}
