mod detail;
mod filters;
mod navigation;
mod scrape;

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use tempfile::TempDir;
use tokio::task::JoinHandle;
use url::Url;

use super::{path::resolve_browser_path, price::PriceDecoder};
use crate::ZIROOM_BASE_URL;

#[derive(Debug, Clone)]
pub(crate) struct BrowserSettings {
    pub(crate) browser_path: Option<PathBuf>,
    pub(crate) headed: bool,
    pub(crate) navigation_timeout: Duration,
}

#[derive(Debug, Clone)]
pub(crate) enum BrowserProgress {
    Page {
        current: usize,
        total: Option<usize>,
        listings: usize,
    },
}

pub(crate) struct ZiroomClient {
    browser: Browser,
    page: Page,
    handler: JoinHandle<()>,
    _profile: Option<TempDir>,
    decoder: PriceDecoder,
    timeout: Duration,
    headed: bool,
}

impl ZiroomClient {
    #[cfg(test)]
    async fn launch(settings: BrowserSettings) -> Result<Self> {
        Self::launch_with_profile(settings, None).await
    }

    pub(crate) async fn launch_with_profile(
        settings: BrowserSettings,
        shared_profile: Option<&Path>,
    ) -> Result<Self> {
        let executable = resolve_browser_path(settings.browser_path.as_deref())?;
        let owned_profile = if shared_profile.is_none() {
            Some(tempfile::Builder::new().prefix("nest-browser-").tempdir()?)
        } else {
            None
        };
        let profile_path = shared_profile
            .or_else(|| owned_profile.as_ref().map(TempDir::path))
            .expect("a browser profile is always available");
        let mut builder = BrowserConfig::builder()
            .chrome_executable(&executable)
            .user_data_dir(profile_path)
            .window_size(1280, 900)
            .launch_timeout(settings.navigation_timeout)
            .request_timeout(settings.navigation_timeout);
        builder = if settings.headed {
            builder.with_head()
        } else {
            builder.new_headless_mode()
        };
        let config = builder
            .build()
            .map_err(|error| anyhow!("invalid browser configuration: {error}"))?;
        let (browser, mut handler_stream) = Browser::launch(config)
            .await
            .with_context(|| format!("failed to launch browser {}", executable.display()))?;
        let handler = tokio::spawn(async move {
            while let Some(result) = handler_stream.next().await {
                if let Err(error) = result {
                    tracing::warn!(%error, "browser handler stopped");
                    break;
                }
            }
        });
        let page = browser.new_page("about:blank").await?;
        let client = Self {
            browser,
            page,
            handler,
            _profile: owned_profile,
            decoder: PriceDecoder::new()?,
            timeout: settings.navigation_timeout,
            headed: settings.headed,
        };
        client.goto(Url::parse(ZIROOM_BASE_URL)?).await?;
        Ok(client)
    }

    pub(crate) async fn close(mut self) -> Result<()> {
        let close_result = self.browser.close().await;
        if tokio::time::timeout(Duration::from_secs(5), &mut self.handler)
            .await
            .is_err()
        {
            self.handler.abort();
            let _ = self.handler.await;
        }
        close_result.map(|_| ()).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests;
