use crossterm::event::{KeyCode, KeyEvent};
use url::Url;

use super::AppState;
use crate::{
    app::protocol::{BackendRequest, Effect, InputMode, Screen},
    export::ExportFormat,
};

impl AppState {
    pub(super) fn handle_result_key(&mut self, key: KeyEvent) -> Vec<Effect> {
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
}
