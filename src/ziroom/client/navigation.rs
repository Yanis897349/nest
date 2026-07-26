use std::time::Duration;

use anyhow::{Context, Result, bail};
use chromiumoxide::Page;
use serde::de::DeserializeOwned;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use url::Url;

use super::ZiroomClient;
use crate::{
    model::FilterCatalog,
    ziroom::{BrowserError, trusted_url::TrustedZiroomUrl},
};

impl ZiroomClient {
    pub(super) async fn current_url(&self) -> Result<String> {
        self.eval("() => location.href").await
    }

    pub(super) async fn wait_for_catalog<F>(&self, ready: F) -> Result<FilterCatalog>
    where
        F: Fn(&FilterCatalog) -> bool,
    {
        let started = tokio::time::Instant::now();
        loop {
            if let Ok(catalog) = self.discover().await
                && ready(&catalog)
            {
                return Ok(catalog);
            }
            if started.elapsed() >= self.timeout {
                bail!("Ziroom filter state did not update before timeout");
            }
            sleep(Duration::from_millis(100)).await;
        }
    }

    pub(super) async fn wait_for_condition(&self, script: &str, description: &str) -> Result<()> {
        let started = tokio::time::Instant::now();
        loop {
            if self.eval::<bool>(script).await.unwrap_or(false) {
                return Ok(());
            }
            if started.elapsed() >= self.timeout {
                bail!("Ziroom did not expose {description} before timeout");
            }
            sleep(Duration::from_millis(100)).await;
        }
    }

    pub(super) async fn goto_with_retries(
        &self,
        url: Url,
        retries: usize,
        cancel: &CancellationToken,
    ) -> Result<()> {
        let mut last_error = None;
        for attempt in 0..=retries {
            match self.goto(url.clone()).await {
                Ok(()) => return Ok(()),
                Err(error) => {
                    last_error = Some(error);
                    if attempt < retries {
                        cancelable_sleep(cancel, Duration::from_secs(1 << attempt)).await?;
                    }
                }
            }
        }
        Err(last_error.expect("at least one navigation attempt"))
    }

    pub(super) async fn goto(&self, url: Url) -> Result<()> {
        let trusted = TrustedZiroomUrl::parse(url)?;
        self.goto_page(&self.page, trusted).await
    }

    pub(super) async fn goto_page(&self, page: &Page, url: TrustedZiroomUrl) -> Result<()> {
        tokio::time::timeout(self.timeout, page.goto(url.as_url().as_str()))
            .await
            .context("navigation timed out")??;
        self.wait_for_site_on(page).await
    }

    pub(super) async fn wait_for_site(&self) -> Result<()> {
        self.wait_for_site_on(&self.page).await
    }

    async fn wait_for_site_on(&self, page: &Page) -> Result<()> {
        let started = tokio::time::Instant::now();
        let mut execution_context_resets = 0_u32;
        let mut challenge_seen = false;
        loop {
            let state: serde_json::Value = match self
                .eval_on(page, "() => ({ready: document.readyState, text: document.body?.innerText || '', html: document.documentElement?.innerHTML || ''})")
                .await
            {
                Ok(state) => state,
                Err(error) if started.elapsed() < self.timeout => {
                    execution_context_resets += 1;
                    tracing::debug!(%error, "waiting for Ziroom execution context");
                    if execution_context_resets >= 8
                        && started.elapsed() >= Duration::from_secs(3)
                    {
                        return Err(BrowserError::ChallengeRequired.into());
                    }
                    sleep(Duration::from_millis(250)).await;
                    continue;
                }
                Err(error) => return Err(error),
            };
            let text = state["text"].as_str().unwrap_or_default();
            let html = state["html"].as_str().unwrap_or_default();
            challenge_seen |= html.contains("EO_Bot_Ssid") || html.contains("__tst_status");
            if text.contains("找房方式") || text.contains("房源") {
                return Ok(());
            }
            if !self.headed && started.elapsed() >= Duration::from_secs(10) {
                return Err(BrowserError::ChallengeRequired.into());
            }
            if challenge_seen && started.elapsed() >= Duration::from_secs(5) {
                return Err(BrowserError::ChallengeRequired.into());
            }
            if started.elapsed() >= self.timeout {
                if challenge_seen || execution_context_resets >= 3 {
                    return Err(BrowserError::ChallengeRequired.into());
                }
                bail!("Ziroom page did not expose its search content before timeout");
            }
            sleep(Duration::from_millis(250)).await;
        }
    }

    pub(super) async fn eval<T: DeserializeOwned>(&self, script: &str) -> Result<T> {
        self.eval_on(&self.page, script).await
    }

    pub(super) async fn eval_on<T: DeserializeOwned>(
        &self,
        page: &Page,
        script: &str,
    ) -> Result<T> {
        page.evaluate(script)
            .await?
            .into_value()
            .context("browser script returned an unexpected value")
    }
}

pub(super) async fn cancelable_sleep(cancel: &CancellationToken, duration: Duration) -> Result<()> {
    tokio::select! {
        () = cancel.cancelled() => bail!("scrape cancelled"),
        () = sleep(duration) => Ok(()),
    }
}
