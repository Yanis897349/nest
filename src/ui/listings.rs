use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};

use super::chrome::render_centered;
use crate::app::AppState;

pub(super) fn render_results(frame: &mut Frame<'_>, app: &AppState, area: Rect) {
    let [table_area, preview_area] =
        Layout::vertical([Constraint::Percentage(67), Constraint::Percentage(33)]).areas(area);
    let Some(report) = &app.report else {
        render_centered(frame, area, "No scrape results");
        return;
    };
    let rows = report.listings.iter().map(|listing| {
        Row::new(vec![
            Cell::from(
                listing
                    .monthly_rent_cny
                    .map(|value| format!("¥{value}"))
                    .unwrap_or_else(|| "—".into()),
            ),
            Cell::from(listing.title.clone()),
            Cell::from(
                listing
                    .area_sqm
                    .map(|value| format!("{value:.1}㎡"))
                    .unwrap_or_default(),
            ),
            Cell::from(listing.orientation.clone().unwrap_or_default()),
            Cell::from(
                listing
                    .location
                    .clone()
                    .or_else(|| listing.transit.clone())
                    .unwrap_or_default(),
            ),
        ])
    });
    let mut state = TableState::default().with_selected(Some(app.result_index));
    frame.render_stateful_widget(
        Table::new(
            rows,
            [
                Constraint::Length(9),
                Constraint::Percentage(30),
                Constraint::Length(10),
                Constraint::Length(9),
                Constraint::Min(20),
            ],
        )
        .header(
            Row::new(["Rent", "Title", "Area", "Facing", "Location / transit"]).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .title(format!(" Results — {} listings ", report.listings.len()))
                .borders(Borders::ALL),
        ),
        table_area,
        &mut state,
    );
    if let Some(listing) = report.listings.get(app.result_index) {
        let text = vec![
            Line::from(Span::styled(
                &listing.title,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(format!(
                "ID: {}   Floor: {}",
                listing.listing_id,
                listing.floor.as_deref().unwrap_or("—")
            )),
            Line::from(format!("Tags: {}", listing.tags.join(" · "))),
            Line::from(format!("URL: {}", listing.detail_url)),
        ];
        frame.render_widget(
            Paragraph::new(text).wrap(Wrap { trim: true }).block(
                Block::default()
                    .title(" Selected listing ")
                    .borders(Borders::ALL),
            ),
            preview_area,
        );
    }
}

pub(super) fn render_detail(frame: &mut Frame<'_>, app: &AppState, area: Rect) {
    let Some(detail) = &app.detail else {
        render_centered(frame, area, "No detail loaded");
        return;
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("Listing {}", detail.listing_id),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "Status: {}",
            detail.status.as_deref().unwrap_or("—")
        )),
    ];
    if let Some(description) = &detail.description {
        lines.push(Line::from(format!("Description: {description}")));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Ziroom facts",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    for (key, value) in &detail.facts {
        lines.push(Line::from(format!("{key}: {value}")));
    }
    if !detail.amenities.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(format!(
            "Amenities: {}",
            detail.amenities.join(" · ")
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Ziroom image URLs",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    if detail.image_urls.is_empty() {
        lines.push(Line::from("—"));
    } else {
        lines.extend(
            detail
                .image_urls
                .iter()
                .map(|url| Line::from(url.as_str().to_owned())),
        );
    }
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::default()
                .title(" Lazy listing detail ")
                .borders(Borders::ALL),
        ),
        area,
    );
}
