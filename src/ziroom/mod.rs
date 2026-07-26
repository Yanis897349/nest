mod client;
mod path;
mod price;
mod trusted_url;

pub(crate) use client::{BrowserProgress, BrowserSettings, ZiroomClient};
use thiserror::Error;
pub(crate) use trusted_url::validate_ziroom_url;

#[derive(Debug, Error)]
pub(crate) enum BrowserError {
    #[error("Ziroom presented a browser challenge that did not clear automatically")]
    ChallengeRequired,
    #[error("no supported Chromium-family browser was found; pass --browser PATH or NEST_BROWSER")]
    BrowserNotFound,
    #[error("navigation to an untrusted URL was rejected: {0}")]
    UnsafeNavigation(String),
}
