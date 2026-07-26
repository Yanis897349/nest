use std::io::{self, Stdout};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, TableState, Wrap,
    },
};

use crate::app::{ExportFormat, InputMode, Screen, ViewState};

pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error.into());
        }
        let terminal = match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => terminal,
            Err(error) => {
                let mut stdout = io::stdout();
                let _ = execute!(stdout, LeaveAlternateScreen);
                let _ = disable_raw_mode();
                return Err(error.into());
            }
        };
        Ok(Self { terminal })
    }

    pub fn draw<F>(&mut self, render: F) -> Result<()>
    where
        F: FnOnce(&mut Frame<'_>),
    {
        self.terminal.draw(render)?;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

pub(crate) fn render(frame: &mut Frame<'_>, app: &ViewState) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(2),
    ])
    .areas(frame.area());
    render_header(frame, app, header);
    match app.screen {
        Screen::Loading => render_centered(frame, body, "Launching managed Chromium…"),
        Screen::Filters => render_filters(frame, app, body),
        Screen::Progress => render_progress(frame, app, body),
        Screen::Results => render_results(frame, app, body),
        Screen::Detail => render_detail(frame, app, body),
        Screen::Help => render_help(frame, body),
        Screen::Challenge => render_challenge(frame, body),
        Screen::Error => render_error(frame, app, body),
    }
    render_footer(frame, app, footer);
    if let Some(input) = &app.input {
        render_modal(frame, input, app.input_error.as_deref());
    }
}

fn render_progress(frame: &mut Frame<'_>, app: &ViewState, area: Rect) {
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

fn render_header(frame: &mut Frame<'_>, app: &ViewState, area: Rect) {
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

fn render_filters(frame: &mut Frame<'_>, app: &ViewState, area: Rect) {
    let [categories, options] =
        Layout::horizontal([Constraint::Percentage(32), Constraint::Percentage(68)]).areas(area);
    let groups = app.groups();
    let category_items = groups.iter().enumerate().map(|(index, group)| {
        let style = if index == app.category_index {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(group.title.clone()).style(style)
    });
    frame.render_widget(
        List::new(category_items).block(Block::default().title(" Filters ").borders(Borders::ALL)),
        categories,
    );
    let selected_group = groups.get(app.category_index);
    let option_items = selected_group
        .into_iter()
        .flat_map(|group| group.options.iter().enumerate())
        .map(|(index, option)| {
            let marker = if option.selected { "[x]" } else { "[ ]" };
            let mut style = if option.selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            if index == app.option_index {
                style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
            }
            ListItem::new(format!("{marker} {}", option.label)).style(style)
        });
    frame.render_widget(
        List::new(option_items).block(Block::default().title(" Options ").borders(Borders::ALL)),
        options,
    );
}

fn render_results(frame: &mut Frame<'_>, app: &ViewState, area: Rect) {
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

fn render_detail(frame: &mut Frame<'_>, app: &ViewState, area: Rect) {
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

fn render_help(frame: &mut Frame<'_>, area: Rect) {
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

fn render_challenge(frame: &mut Frame<'_>, area: Rect) {
    render_centered(
        frame,
        area,
        "Ziroom's browser challenge did not clear headlessly.\n\nPress Enter to retry in a visible temporary browser.\nResolve any challenge there, then return here.\n\nEsc or q cancels. CAPTCHAs are never automated.",
    );
}

fn render_error(frame: &mut Frame<'_>, app: &ViewState, area: Rect) {
    frame.render_widget(
        Paragraph::new(app.status.as_str())
            .style(Style::default().fg(Color::LightRed))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center)
            .block(Block::default().title(" Error ").borders(Borders::ALL)),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, app: &ViewState, area: Rect) {
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

fn render_modal(frame: &mut Frame<'_>, input: &InputMode, input_error: Option<&str>) {
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

fn render_centered(frame: &mut Frame<'_>, area: Rect, text: &str) {
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn centered_rect(width_percent: u16, height_percent: u16, area: Rect) -> Rect {
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

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use ratatui::{Terminal, backend::TestBackend};
    use url::Url;

    use super::*;
    use crate::{
        app::ViewState,
        model::{FilterCatalog, FilterOption, ListingDetail, ListingSummary, ScrapeReport},
    };

    fn rendered(state: &ViewState, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, state)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    fn state_with_results() -> ViewState {
        let mut state = ViewState::new();
        let source = Url::parse("https://www.ziroom.com/z/").unwrap();
        state.catalog = Some(FilterCatalog {
            districts: vec![FilterOption::new("海淀", Some(source.clone()))],
            selected_labels: vec!["海淀".into()],
            ..FilterCatalog::default()
        });
        state.report = Some(ScrapeReport {
            schema_version: 1,
            scraped_at: Utc::now(),
            source_url: source.clone(),
            selected_filters: vec!["海淀".into()],
            reported_total: Some(1),
            warnings: Vec::new(),
            listings: vec![ListingSummary {
                listing_id: "810907652".into(),
                title: "东馨园3居".into(),
                monthly_rent_cny: Some(8190),
                area_sqm: Some(81.9),
                floor: Some("4/6层".into()),
                orientation: Some("朝南".into()),
                location: Some("海淀".into()),
                transit: None,
                tags: vec!["独立阳台".into()],
                image_url: None,
                detail_url: Url::parse("https://www.ziroom.com/x/810907652.html").unwrap(),
                source_page_url: source,
                scraped_at: Utc::now(),
            }],
        });
        state.detail = Some(ListingDetail {
            listing_id: "810907652".into(),
            status: Some("可预订".into()),
            ..ListingDetail::default()
        });
        state.busy = false;
        state
    }

    #[test]
    fn loading_and_results_render_expected_content() {
        let loading = rendered(&ViewState::new(), 100, 30);
        assert!(loading.contains("Launching managed Chromium"));

        let mut state = state_with_results();
        state.screen = Screen::Results;
        let results = rendered(&state, 100, 30);
        assert!(results.contains('东'));
        assert!(results.contains('居'));
        assert!(results.contains("¥8190"));
    }

    #[test]
    fn modal_validation_message_is_visible() {
        let mut state = state_with_results();
        state.screen = Screen::Filters;
        state.input = Some(InputMode::Commute {
            address: String::new(),
            minutes: 30,
        });
        state.input_error = Some("Enter a company address".into());
        assert!(rendered(&state, 100, 30).contains("Enter a company address"));
    }

    #[test]
    fn every_screen_renders_in_a_constrained_terminal() {
        for screen in [
            Screen::Loading,
            Screen::Filters,
            Screen::Progress,
            Screen::Results,
            Screen::Detail,
            Screen::Help,
            Screen::Challenge,
            Screen::Error,
        ] {
            let mut state = state_with_results();
            state.screen = screen;
            let output = rendered(&state, 40, 12);
            assert!(!output.is_empty());
        }
    }
}
