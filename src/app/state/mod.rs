mod filters;
mod input;
mod navigation;

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::model::{FilterCatalog, ListingDetail, ScrapeReport};

use super::protocol::{Action, BackendEvent, BackendRequest, Effect, InputMode, Screen};

pub(crate) struct AppState {
    pub(crate) screen: Screen,
    pub(crate) previous_screen: Screen,
    pub(crate) catalog: Option<FilterCatalog>,
    pub(crate) report: Option<ScrapeReport>,
    pub(crate) detail: Option<ListingDetail>,
    pub(crate) category_index: usize,
    pub(crate) option_index: usize,
    pub(crate) result_index: usize,
    pub(crate) status: String,
    pub(crate) input: Option<InputMode>,
    pub(crate) input_error: Option<String>,
    pub(crate) busy: bool,
    pub(crate) quit: bool,
    pub(super) scraping: bool,
    pub(super) detail_cache: HashMap<String, ListingDetail>,
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
            BackendEvent::Progress(crate::ziroom::BrowserProgress::Page {
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

    pub(super) fn show_help(&mut self) {
        self.previous_screen = self.screen;
        self.screen = Screen::Help;
    }

    pub(super) fn show_error(&mut self, error: String) {
        if self.screen != Screen::Error {
            self.previous_screen = self.screen;
        }
        self.input = None;
        self.input_error = None;
        self.status = error;
        self.screen = Screen::Error;
    }

    pub(super) fn request_quit(&mut self) -> Vec<Effect> {
        self.quit = true;
        vec![Effect::CancelScrape, Effect::Quit]
    }
}

#[cfg(test)]
mod tests;
