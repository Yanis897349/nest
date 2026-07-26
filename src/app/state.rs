use std::{collections::HashMap, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use url::Url;

use crate::ziroom::BrowserProgress;
use crate::{
    export::ExportFormat,
    model::{FilterCatalog, FilterOption, ListingDetail, LocationMode, ScrapeReport},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Screen {
    Loading,
    Filters,
    Progress,
    Results,
    Detail,
    Help,
    Challenge,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InputMode {
    CustomRent {
        min: String,
        max: String,
        editing_max: bool,
    },
    Commute {
        address: String,
        minutes: u8,
    },
    Export {
        format: ExportFormat,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupKind {
    LocationMode,
    District,
    BusinessArea,
    SubwayLine,
    Station,
    Commute,
    RentalType,
    Rent,
    Bedroom,
    Sort,
}

#[derive(Debug, Clone)]
pub(crate) struct FilterGroup {
    pub title: String,
    pub kind: GroupKind,
    pub options: Vec<FilterOption>,
}

#[derive(Debug)]
pub(crate) enum BackendRequest {
    Initialize,
    RetryHeaded,
    Reset,
    Navigate(Url),
    SetMode(LocationMode),
    SetCustomRent(u32, u32),
    SetCommute(String, u8),
    Scrape,
    Detail(Url),
}

#[derive(Debug)]
pub(crate) enum BackendEvent {
    Status(String),
    Catalog(FilterCatalog),
    Progress(BrowserProgress),
    Report(ScrapeReport),
    Detail(ListingDetail),
    Cancelled,
    Challenge,
    Error(String),
}

#[derive(Debug)]
pub(crate) enum Action {
    Key(KeyEvent),
    Backend(Box<BackendEvent>),
    Exported(Result<PathBuf, String>),
}

#[derive(Debug)]
pub(crate) enum Effect {
    Backend(BackendRequest),
    CancelScrape,
    OpenUrl(Url),
    Export(ExportFormat),
    Quit,
}

pub(crate) struct AppState {
    pub screen: Screen,
    pub previous_screen: Screen,
    pub catalog: Option<FilterCatalog>,
    pub report: Option<ScrapeReport>,
    pub detail: Option<ListingDetail>,
    pub category_index: usize,
    pub option_index: usize,
    pub result_index: usize,
    pub status: String,
    pub input: Option<InputMode>,
    pub input_error: Option<String>,
    pub busy: bool,
    pub quit: bool,
    scraping: bool,
    detail_cache: HashMap<String, ListingDetail>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub(crate) fn new() -> Self {
        Self {
            screen: Screen::Loading,
            previous_screen: Screen::Filters,
            catalog: None,
            report: None,
            detail: None,
            category_index: 0,
            option_index: 0,
            result_index: 0,
            status: "Starting browser…".into(),
            input: None,
            input_error: None,
            busy: true,
            quit: false,
            scraping: false,
            detail_cache: HashMap::new(),
        }
    }

    pub(crate) fn update(&mut self, action: Action) -> Vec<Effect> {
        match action {
            Action::Key(key) => self.handle_key(key),
            Action::Backend(event) => {
                self.handle_backend_event(*event);
                Vec::new()
            }
            Action::Exported(result) => {
                self.busy = false;
                match result {
                    Ok(path) => self.status = format!("Exported {}", path.display()),
                    Err(error) => self.show_error(error),
                }
                Vec::new()
            }
        }
    }

    pub(crate) fn groups(&self) -> Vec<FilterGroup> {
        let Some(catalog) = &self.catalog else {
            return Vec::new();
        };
        let modes = LocationMode::ALL
            .into_iter()
            .map(|mode| FilterOption {
                label: mode.label().into(),
                url: None,
                selected: mode == catalog.mode,
            })
            .collect();
        let mut groups = vec![FilterGroup {
            title: "Location mode".into(),
            kind: GroupKind::LocationMode,
            options: modes,
        }];
        match catalog.mode {
            LocationMode::Region => {
                groups.push(FilterGroup {
                    title: "District / 区域".into(),
                    kind: GroupKind::District,
                    options: catalog.districts.clone(),
                });
                if !catalog.business_areas.is_empty() {
                    groups.push(FilterGroup {
                        title: "Business areas / 商圈".into(),
                        kind: GroupKind::BusinessArea,
                        options: catalog.business_areas.clone(),
                    });
                }
            }
            LocationMode::Subway => {
                groups.push(FilterGroup {
                    title: "Subway line / 地铁".into(),
                    kind: GroupKind::SubwayLine,
                    options: catalog.subway_lines.clone(),
                });
                if !catalog.stations.is_empty() {
                    groups.push(FilterGroup {
                        title: "Stations / 站点".into(),
                        kind: GroupKind::Station,
                        options: catalog.stations.clone(),
                    });
                }
            }
            LocationMode::Commute => groups.push(FilterGroup {
                title: "Commute / 通勤".into(),
                kind: GroupKind::Commute,
                options: vec![FilterOption::new("Press a to enter company address", None)],
            }),
        }
        groups.extend([
            FilterGroup {
                title: "Rental type / 类型".into(),
                kind: GroupKind::RentalType,
                options: catalog.rental_types.clone(),
            },
            FilterGroup {
                title: "Rent / 租金".into(),
                kind: GroupKind::Rent,
                options: catalog.rent_presets.clone(),
            },
            FilterGroup {
                title: "Bedrooms / 户型".into(),
                kind: GroupKind::Bedroom,
                options: catalog.bedrooms.clone(),
            },
            FilterGroup {
                title: "Sort".into(),
                kind: GroupKind::Sort,
                options: catalog.sort_orders.clone(),
            },
        ]);
        groups.retain(|group| !group.options.is_empty());
        groups
    }

    fn handle_backend_event(&mut self, event: BackendEvent) {
        match event {
            BackendEvent::Status(status) => self.status = status,
            BackendEvent::Catalog(catalog) => {
                self.catalog = Some(catalog);
                self.category_index = self
                    .category_index
                    .min(self.groups().len().saturating_sub(1));
                self.option_index = 0;
                self.busy = false;
                if matches!(
                    self.screen,
                    Screen::Loading | Screen::Challenge | Screen::Error
                ) {
                    self.screen = Screen::Filters;
                }
                self.status = "Filter catalog loaded".into();
            }
            BackendEvent::Progress(BrowserProgress::Page {
                current,
                total,
                listings,
            }) => {
                self.status = format!(
                    "Scraping page {current}{} — {listings} unique listings",
                    total.map(|value| format!("/{value}")).unwrap_or_default()
                );
            }
            BackendEvent::Report(report) => {
                self.report = Some(report);
                self.result_index = 0;
                self.scraping = false;
                self.busy = false;
                self.screen = Screen::Results;
                self.status = "Scrape complete".into();
            }
            BackendEvent::Detail(detail) => {
                self.detail_cache
                    .insert(detail.listing_id.clone(), detail.clone());
                self.detail = Some(detail);
                self.busy = false;
                self.screen = Screen::Detail;
                self.status = "Listing detail loaded".into();
            }
            BackendEvent::Cancelled => {
                self.scraping = false;
                self.busy = false;
                self.screen = Screen::Filters;
                self.status = "Scrape cancelled".into();
            }
            BackendEvent::Challenge => {
                self.scraping = false;
                self.busy = false;
                self.screen = Screen::Challenge;
                self.status = "Browser challenge requires a visible retry".into();
            }
            BackendEvent::Error(error) => {
                self.scraping = false;
                self.busy = false;
                self.show_error(error);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return vec![Effect::CancelScrape, Effect::Quit];
        }
        if self.input.is_some() {
            return self.handle_input_key(key);
        }
        match self.screen {
            Screen::Loading => match key.code {
                KeyCode::Char('q') => self.request_quit(),
                _ => Vec::new(),
            },
            Screen::Challenge => match key.code {
                KeyCode::Enter => {
                    self.busy = true;
                    self.screen = Screen::Loading;
                    self.status = "Relaunching a visible browser…".into();
                    vec![Effect::Backend(BackendRequest::RetryHeaded)]
                }
                KeyCode::Esc | KeyCode::Char('q') => self.request_quit(),
                _ => Vec::new(),
            },
            Screen::Help => match key.code {
                KeyCode::Esc | KeyCode::Char('?') => {
                    self.screen = self.previous_screen;
                    Vec::new()
                }
                _ => Vec::new(),
            },
            Screen::Error => match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.screen = self.previous_screen;
                    Vec::new()
                }
                KeyCode::Char('q') => self.request_quit(),
                _ => Vec::new(),
            },
            Screen::Filters => self.handle_filter_key(key),
            Screen::Progress => match key.code {
                KeyCode::Char('c') if self.scraping => {
                    self.status = "Cancelling…".into();
                    vec![Effect::CancelScrape]
                }
                KeyCode::Char('?') => {
                    self.show_help();
                    Vec::new()
                }
                KeyCode::Char('q') => self.request_quit(),
                _ => Vec::new(),
            },
            Screen::Results => self.handle_result_key(key),
            Screen::Detail => match key.code {
                KeyCode::Esc | KeyCode::Char('d') => {
                    self.screen = Screen::Results;
                    Vec::new()
                }
                KeyCode::Char('?') => {
                    self.show_help();
                    Vec::new()
                }
                KeyCode::Char('q') => self.request_quit(),
                _ => Vec::new(),
            },
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        let groups = self.groups();
        if groups.is_empty() {
            return Vec::new();
        }
        match key.code {
            KeyCode::Left | KeyCode::BackTab => {
                self.category_index = self.category_index.saturating_sub(1);
                self.option_index = 0;
            }
            KeyCode::Right | KeyCode::Tab => {
                self.category_index = (self.category_index + 1).min(groups.len() - 1);
                self.option_index = 0;
            }
            KeyCode::Up => self.option_index = self.option_index.saturating_sub(1),
            KeyCode::Down => {
                self.option_index = (self.option_index + 1)
                    .min(groups[self.category_index].options.len().saturating_sub(1));
            }
            KeyCode::Char(' ') | KeyCode::Enter if !self.busy => {
                let group = groups[self.category_index].clone();
                let option = group.options.get(self.option_index).cloned();
                match group.kind {
                    GroupKind::LocationMode => {
                        let mode = LocationMode::ALL[self.option_index.min(2)];
                        self.busy = true;
                        return vec![Effect::Backend(BackendRequest::SetMode(mode))];
                    }
                    GroupKind::Commute => self.open_commute_input(),
                    _ => {
                        if let Some(url) = option.and_then(|value| value.url) {
                            self.busy = true;
                            self.status = "Applying filter…".into();
                            return vec![Effect::Backend(BackendRequest::Navigate(url))];
                        }
                    }
                }
            }
            KeyCode::Char('p') if !self.busy => {
                self.input = Some(InputMode::CustomRent {
                    min: String::new(),
                    max: String::new(),
                    editing_max: false,
                });
                self.input_error = None;
            }
            KeyCode::Char('a') if !self.busy => self.open_commute_input(),
            KeyCode::Char('s') if !self.busy => {
                self.scraping = true;
                self.busy = true;
                self.screen = Screen::Progress;
                self.status = "Starting full scrape…".into();
                return vec![Effect::Backend(BackendRequest::Scrape)];
            }
            KeyCode::Char('r') if !self.busy => {
                self.busy = true;
                self.screen = Screen::Loading;
                self.status = "Resetting filters…".into();
                return vec![Effect::Backend(BackendRequest::Reset)];
            }
            KeyCode::Char('?') => self.show_help(),
            KeyCode::Char('q') => return self.request_quit(),
            _ => {}
        }
        Vec::new()
    }

    fn handle_result_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        let count = self
            .report
            .as_ref()
            .map_or(0, |report| report.listings.len());
        match key.code {
            KeyCode::Up => self.result_index = self.result_index.saturating_sub(1),
            KeyCode::Down => {
                self.result_index = (self.result_index + 1).min(count.saturating_sub(1));
            }
            KeyCode::Char('d') | KeyCode::Enter if !self.busy => {
                if let Some((listing_id, url)) = self.selected_listing_identity() {
                    if let Some(detail) = self.detail_cache.get(&listing_id).cloned() {
                        self.detail = Some(detail);
                        self.screen = Screen::Detail;
                        self.status = "Listing detail loaded from memory".into();
                    } else {
                        self.busy = true;
                        self.status = "Loading listing detail…".into();
                        return vec![Effect::Backend(BackendRequest::Detail(url))];
                    }
                }
            }
            KeyCode::Char('o') => {
                if let Some(url) = self.selected_listing_url() {
                    return vec![Effect::OpenUrl(url)];
                }
            }
            KeyCode::Char('e') if self.report.is_some() => {
                self.input = Some(InputMode::Export {
                    format: ExportFormat::Json,
                });
                self.input_error = None;
            }
            KeyCode::Char('r') => self.screen = Screen::Filters,
            KeyCode::Char('?') => self.show_help(),
            KeyCode::Char('q') => return self.request_quit(),
            _ => {}
        }
        Vec::new()
    }

    fn handle_input_key(&mut self, key: KeyEvent) -> Vec<Effect> {
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

    fn open_commute_input(&mut self) {
        self.input = Some(InputMode::Commute {
            address: String::new(),
            minutes: 30,
        });
        self.input_error = None;
    }

    fn selected_listing_url(&self) -> Option<Url> {
        self.report
            .as_ref()?
            .listings
            .get(self.result_index)
            .map(|listing| listing.detail_url.clone())
    }

    fn selected_listing_identity(&self) -> Option<(String, Url)> {
        let listing = self.report.as_ref()?.listings.get(self.result_index)?;
        Some((listing.listing_id.clone(), listing.detail_url.clone()))
    }

    fn show_help(&mut self) {
        self.previous_screen = self.screen;
        self.screen = Screen::Help;
    }

    fn show_error(&mut self, error: String) {
        if self.screen != Screen::Error {
            self.previous_screen = self.screen;
        }
        self.input = None;
        self.input_error = None;
        self.status = error;
        self.screen = Screen::Error;
    }

    fn request_quit(&mut self) -> Vec<Effect> {
        self.quit = true;
        vec![Effect::CancelScrape, Effect::Quit]
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
    use chrono::Utc;

    use super::*;
    use crate::model::ListingSummary;

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
    fn commute_steps_stay_in_supported_range() {
        assert_eq!(next_commute(15, false), 15);
        assert_eq!(next_commute(15, true), 30);
        assert_eq!(next_commute(50, true), 60);
        assert_eq!(next_commute(60, true), 60);
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
        assert!(state.status.contains("2/4"));
        let effects = state.update(key(KeyCode::Char('c')));
        assert!(matches!(effects.as_slice(), [Effect::CancelScrape]));
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
        assert!(state.status.contains("memory"));
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
        assert!(state.status.contains("out.json"));
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
}
