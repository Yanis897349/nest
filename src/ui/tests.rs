use chrono::Utc;
use ratatui::{Terminal, backend::TestBackend};
use url::Url;

use super::render;
use crate::{
    app::{AppState, InputMode, Screen},
    model::{FilterCatalog, FilterOption, ListingDetail, ListingSummary, ScrapeReport},
};

fn rendered(state: &AppState, width: u16, height: u16) -> String {
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

fn state_with_results() -> AppState {
    let mut state = AppState::new();
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
    let loading = rendered(&AppState::new(), 100, 30);
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
