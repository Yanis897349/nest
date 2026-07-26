use std::path::PathBuf;

use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use url::Url;

use super::AppState;
use crate::{
    app::protocol::{Action, BackendEvent, BackendRequest, Effect, InputMode, Screen},
    model::{FilterCatalog, FilterOption, ListingDetail, ListingSummary, ScrapeReport},
    ziroom::BrowserProgress,
};

fn key(code: KeyCode) -> Action {
    Action::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn url(value: &str) -> Url {
    Url::parse(value).unwrap()
}

fn catalog() -> FilterCatalog {
    FilterCatalog {
        districts: vec![FilterOption::new(
            "海淀",
            Some(url("https://www.ziroom.com/z/d23008618/")),
        )],
        rental_types: vec![FilterOption::new(
            "整租",
            Some(url("https://www.ziroom.com/z/z2/")),
        )],
        rent_presets: vec![FilterOption::new(
            "3000-4000元",
            Some(url("https://www.ziroom.com/z/?cp=3000TO4000")),
        )],
        bedrooms: vec![FilterOption::new(
            "3居+",
            Some(url("https://www.ziroom.com/z/u15/")),
        )],
        sort_orders: vec![FilterOption::new(
            "默认排序",
            Some(url("https://www.ziroom.com/z/")),
        )],
        ..FilterCatalog::default()
    }
}

fn report() -> ScrapeReport {
    let source = url("https://www.ziroom.com/z/");
    ScrapeReport {
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
            detail_url: url("https://www.ziroom.com/x/810907652.html"),
            source_page_url: source,
            scraped_at: Utc::now(),
        }],
    }
}

#[test]
fn invalid_custom_rent_stays_in_the_modal() {
    let mut state = AppState::new();
    state.screen = Screen::Filters;
    state.busy = false;
    state.catalog = Some(FilterCatalog::default());
    state.update(key(KeyCode::Char('p')));
    state.update(key(KeyCode::Tab));
    state.update(key(KeyCode::Enter));
    assert!(matches!(state.input, Some(InputMode::CustomRent { .. })));
    assert_eq!(state.screen, Screen::Filters);
    assert!(state.input_error.is_some());
}

#[test]
fn control_c_requests_cancellation_and_quit() {
    let mut state = AppState::new();
    let effects = state.update(Action::Key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    )));
    assert!(state.quit);
    assert!(matches!(
        effects.as_slice(),
        [Effect::CancelScrape, Effect::Quit]
    ));
}

#[test]
fn catalog_event_enters_filters_and_clamps_selection() {
    let mut state = AppState::new();
    state.category_index = usize::MAX;
    state.option_index = usize::MAX;
    state.update(Action::Backend(Box::new(BackendEvent::Catalog(catalog()))));
    assert_eq!(state.screen, Screen::Filters);
    assert!(!state.busy);
    assert!(state.category_index < state.groups().len());
    assert_eq!(state.option_index, 0);
}

#[test]
fn filter_navigation_stays_within_available_options() {
    let mut state = AppState::new();
    state.update(Action::Backend(Box::new(BackendEvent::Catalog(catalog()))));
    for _ in 0..100 {
        state.update(key(KeyCode::Right));
        state.update(key(KeyCode::Down));
    }
    assert_eq!(state.category_index, state.groups().len() - 1);
    assert_eq!(state.option_index, 0);
}

#[test]
fn filter_projection_changes_with_location_mode() {
    let mut state = AppState::new();
    let mut subway = catalog();
    subway.mode = crate::model::LocationMode::Subway;
    subway.subway_lines = vec![FilterOption::new("1号线", None)];
    state.catalog = Some(subway);
    let titles: Vec<_> = state
        .groups()
        .into_iter()
        .map(|group| group.title)
        .collect();
    assert!(titles.iter().any(|title| title == "Subway line / 地铁"));
    assert!(!titles.iter().any(|title| title == "District / 区域"));
}

#[test]
fn scrape_progress_and_cancellation_have_consistent_state() {
    let mut state = AppState::new();
    state.update(Action::Backend(Box::new(BackendEvent::Catalog(catalog()))));
    let effects = state.update(key(KeyCode::Char('s')));
    assert!(matches!(
        effects.as_slice(),
        [Effect::Backend(BackendRequest::Scrape)]
    ));
    assert_eq!(state.screen, Screen::Progress);
    state.update(Action::Backend(Box::new(BackendEvent::Progress(
        BrowserProgress::Page {
            current: 2,
            total: Some(4),
            listings: 18,
        },
    ))));
    assert_eq!(state.status, "Scraping page 2/4 — 18 unique listings");
    let effects = state.update(key(KeyCode::Char('c')));
    assert!(matches!(effects.as_slice(), [Effect::CancelScrape]));
    assert_eq!(state.status, "Cancelling…");
    state.update(Action::Backend(Box::new(BackendEvent::Cancelled)));
    assert_eq!(state.screen, Screen::Filters);
    assert!(!state.busy);
}

#[test]
fn detail_is_requested_once_and_then_served_from_cache() {
    let mut state = AppState::new();
    state.update(Action::Backend(Box::new(BackendEvent::Report(report()))));
    let effects = state.update(key(KeyCode::Char('d')));
    assert!(matches!(
        effects.as_slice(),
        [Effect::Backend(BackendRequest::Detail(_))]
    ));
    let detail = ListingDetail {
        listing_id: "810907652".into(),
        status: Some("可预订".into()),
        ..ListingDetail::default()
    };
    state.update(Action::Backend(Box::new(BackendEvent::Detail(detail))));
    state.update(key(KeyCode::Esc));
    assert!(state.update(key(KeyCode::Char('d'))).is_empty());
    assert_eq!(state.screen, Screen::Detail);
    assert_eq!(state.status, "Listing detail loaded from memory");
}

#[test]
fn challenge_can_retry_headed_and_errors_return_to_previous_screen() {
    let mut state = AppState::new();
    state.update(Action::Backend(Box::new(BackendEvent::Challenge)));
    assert_eq!(state.screen, Screen::Challenge);
    let effects = state.update(key(KeyCode::Enter));
    assert!(matches!(
        effects.as_slice(),
        [Effect::Backend(BackendRequest::RetryHeaded)]
    ));
    state.update(Action::Backend(Box::new(BackendEvent::Error(
        "launch failed".into(),
    ))));
    assert_eq!(state.screen, Screen::Error);
    state.update(key(KeyCode::Esc));
    assert_eq!(state.screen, Screen::Loading);
}

#[test]
fn export_result_reports_success_or_a_recoverable_error() {
    let mut state = AppState::new();
    state.screen = Screen::Results;
    state.busy = true;
    state.update(Action::Exported(Ok(PathBuf::from("out.json"))));
    assert!(!state.busy);
    assert_eq!(state.status, "Exported out.json");
    state.update(Action::Exported(Err("disk full".into())));
    assert_eq!(state.screen, Screen::Error);
    assert_eq!(state.status, "disk full");
}

#[test]
fn empty_commute_input_stays_open_with_feedback() {
    let mut state = AppState::new();
    state.input = Some(InputMode::Commute {
        address: "   ".into(),
        minutes: 30,
    });
    assert!(state.update(key(KeyCode::Enter)).is_empty());
    assert!(matches!(state.input, Some(InputMode::Commute { .. })));
    assert_eq!(
        state.input_error.as_deref(),
        Some("Enter a company address")
    );
}
