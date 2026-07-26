use std::path::PathBuf;

use crossterm::event::KeyEvent;
use url::Url;

use crate::{
    export::ExportFormat,
    model::{FilterCatalog, ListingDetail, LocationMode, ScrapeReport},
    ziroom::BrowserProgress,
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
