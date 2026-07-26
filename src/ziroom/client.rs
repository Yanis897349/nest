use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
use chromiumoxide::{Browser, BrowserConfig, Page};
use chrono::Utc;
use futures::StreamExt;
use serde::de::DeserializeOwned;
use tempfile::TempDir;
use tokio::{task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::{
    ZIROOM_BASE_URL,
    model::{FilterCatalog, ListingDetail, LocationMode, RawResultPage, ScrapeReport},
    parser::normalize_listing,
};

use super::{
    BrowserError, path::resolve_browser_path, price::PriceDecoder, trusted_url::TrustedZiroomUrl,
};

#[derive(Debug, Clone)]
pub struct BrowserSettings {
    pub browser_path: Option<PathBuf>,
    pub headed: bool,
    pub navigation_timeout: Duration,
}

#[derive(Debug, Clone)]
pub enum BrowserProgress {
    Page {
        current: usize,
        total: Option<usize>,
        listings: usize,
    },
}

pub struct ZiroomClient {
    browser: Browser,
    page: Page,
    handler: JoinHandle<()>,
    _profile: Option<TempDir>,
    decoder: PriceDecoder,
    timeout: Duration,
    headed: bool,
}

#[derive(Debug, Default)]
struct ScrapeTracker {
    visited_pages: HashSet<Url>,
    seen_listings: HashSet<String>,
    pages: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageStart {
    Visit(usize),
    Loop,
    Limit,
}

impl ScrapeTracker {
    fn begin_page(&mut self, url: &Url) -> PageStart {
        if self.pages >= 100 {
            return PageStart::Limit;
        }
        if !self.visited_pages.insert(url.clone()) {
            return PageStart::Loop;
        }
        self.pages += 1;
        PageStart::Visit(self.pages)
    }

    fn accept_listing(&mut self, listing_id: &str) -> bool {
        self.seen_listings.insert(listing_id.to_owned())
    }
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

    pub async fn close(mut self) -> Result<()> {
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

    pub async fn discover(&self) -> Result<FilterCatalog> {
        self.eval(FILTER_CATALOG_SCRIPT).await
    }

    pub async fn navigate_filter(&self, url: Url) -> Result<FilterCatalog> {
        self.goto(url).await?;
        self.discover().await
    }

    pub async fn set_location_mode(&self, mode: LocationMode) -> Result<FilterCatalog> {
        let label = serde_json::to_string(mode.label())?;
        let script = format!(
            r#"() => {{
                const label = {label};
                const ownText = e => [...e.childNodes].filter(n => n.nodeType === 3)
                    .map(n => n.textContent).join('').trim();
                const candidates = [...document.querySelectorAll('a,div,span')]
                    .filter(e => ownText(e) === label || e.textContent.trim() === label);
                const target = candidates.find(e => e.tagName === 'A') || candidates[0];
                if (!target) return false;
                (target.closest('a') || target).click();
                return true;
            }}"#
        );
        let clicked: bool = self.eval(&script).await?;
        if !clicked {
            bail!("Ziroom location mode {label} was not found");
        }
        self.wait_for_catalog(|catalog| catalog.mode == mode).await
    }

    pub async fn set_custom_rent(&self, min: u32, max: u32) -> Result<FilterCatalog> {
        if min > max {
            bail!("minimum rent must not exceed maximum rent");
        }
        let previous_url = self.current_url().await?;
        let script = format!(
            r#"() => {{
                const low = document.querySelector('input[name="low"], input.low');
                const high = document.querySelector('input[name="high"], input.high');
                if (!low || !high) return false;
                const set = (el, value) => {{
                    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
                    setter.call(el, String(value));
                    el.dispatchEvent(new Event('input', {{bubbles: true}}));
                    el.dispatchEvent(new Event('change', {{bubbles: true}}));
                }};
                set(low, {min}); set(high, {max});
                const root = low.closest('.price, li, div');
                const button = root && root.querySelector('input.confirm, button, input[type="button"]');
                if (!button) return false;
                button.click(); return true;
            }}"#
        );
        let clicked: bool = self.eval(&script).await?;
        if !clicked {
            bail!("custom rent controls were not found");
        }
        self.wait_for_site().await?;
        self.wait_for_catalog(|catalog| catalog.current_url.as_str() != previous_url)
            .await
    }

    pub async fn set_commute(&self, address: &str, minutes: u8) -> Result<FilterCatalog> {
        if ![15, 30, 40, 50, 60].contains(&minutes) {
            bail!("commute time must be 15, 30, 40, 50, or 60 minutes");
        }
        self.set_location_mode(LocationMode::Commute).await?;
        let address_json = serde_json::to_string(address)?;
        let type_script = format!(
            r#"() => {{
                const address = {address_json};
                const input = [...document.querySelectorAll('input')]
                    .find(e => (e.placeholder || '').includes('公司地址'));
                if (!input) return false;
                const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
                setter.call(input, address);
                for (const name of ['input', 'change', 'keyup'])
                    input.dispatchEvent(new Event(name, {{bubbles: true}}));
                input.focus(); return true;
            }}"#
        );
        if !self.eval::<bool>(&type_script).await? {
            bail!("commute address input was not found");
        }
        self.wait_for_condition(
            "() => { const input = [...document.querySelectorAll('input')].find(e => (e.placeholder || '').includes('公司地址')); if (!input) return false; const visible = e => !!(e.offsetWidth || e.offsetHeight || e.getClientRects().length); let root = input.parentElement; for (let i = 0; i < 4 && root; i++, root = root.parentElement) { if ([...root.querySelectorAll('li,a')].some(e => visible(e) && e.textContent.trim() && e.textContent.trim() !== input.value)) return true; } return false; }",
            "commute suggestions",
        )
        .await?;
        let previous_url = self.current_url().await?;
        let choose_script = format!(
            r#"() => {{
                const input = [...document.querySelectorAll('input')]
                    .find(e => (e.placeholder || '').includes('公司地址'));
                if (!input) return false;
                const visible = e => !!(e.offsetWidth || e.offsetHeight || e.getClientRects().length);
                let root = input.parentElement;
                for (let i = 0; i < 4 && root; i++, root = root.parentElement) {{
                    const choices = [...root.querySelectorAll('li,a')]
                        .filter(e => visible(e) && e.textContent.trim() && e.textContent.trim() !== input.value);
                    if (choices.length) {{ choices[0].click(); break; }}
                }}
                const time = [...document.querySelectorAll('a')]
                    .find(e => e.textContent.trim() === '{minutes}分');
                if (!time) return false;
                time.click(); return true;
            }}"#
        );
        if !self.eval::<bool>(&choose_script).await? {
            bail!("Ziroom returned no commute suggestion or time control");
        }
        self.wait_for_site().await?;
        self.wait_for_catalog(|catalog| catalog.current_url.as_str() != previous_url)
            .await
    }

    pub async fn scrape_all(
        &mut self,
        cancel: &CancellationToken,
        progress: &tokio::sync::mpsc::Sender<BrowserProgress>,
    ) -> Result<ScrapeReport> {
        let catalog = self.discover().await?;
        let mut report = ScrapeReport::empty(catalog.current_url.clone());
        report.selected_filters = catalog.selected_labels;
        let mut tracker = ScrapeTracker::default();
        let mut next_url = Some(report.source_url.clone());

        while let Some(url) = next_url.take() {
            if cancel.is_cancelled() {
                bail!("scrape cancelled");
            }
            let page_number = match tracker.begin_page(&url) {
                PageStart::Visit(page_number) => page_number,
                PageStart::Loop => {
                    report
                        .warnings
                        .push(format!("pagination loop stopped at {url}"));
                    break;
                }
                PageStart::Limit => {
                    report
                        .warnings
                        .push("stopped at the 100-page safety limit".into());
                    break;
                }
            };
            if page_number > 1 {
                self.goto_with_retries(url.clone(), 2, cancel).await?;
            }
            let raw = self.result_page().await?;
            report.reported_total = report.reported_total.or(raw.reported_total);
            let total_pages = raw
                .reported_total
                .map(|count| count.div_ceil(raw.cards.len().max(1)));
            let source_page = raw.source_url.clone();
            next_url = raw.next_page_url.clone();
            for card in raw.cards {
                if !tracker.accept_listing(&card.listing_id) {
                    continue;
                }
                let price = match self.decoder.decode(&card.price_glyphs, cancel).await {
                    Ok(price) => Some(price),
                    Err(error) => {
                        report.warnings.push(format!(
                            "listing {} price unavailable: {error}",
                            card.listing_id
                        ));
                        None
                    }
                };
                report.listings.push(normalize_listing(
                    card,
                    price,
                    source_page.clone(),
                    Utc::now(),
                ));
            }
            if progress
                .send(BrowserProgress::Page {
                    current: page_number,
                    total: total_pages,
                    listings: report.listings.len(),
                })
                .await
                .is_err()
            {
                cancel.cancel();
                bail!("scrape progress receiver was closed");
            }
            if next_url.is_some() {
                cancelable_sleep(cancel, Duration::from_millis(1200)).await?;
            }
        }
        Ok(report)
    }

    pub async fn fetch_detail(&self, url: Url) -> Result<ListingDetail> {
        let trusted = TrustedZiroomUrl::parse(url)?;
        let page = self.browser.new_page("about:blank").await?;
        let result = async {
            self.goto_page(&page, trusted).await?;
            self.eval_on(&page, DETAIL_SCRIPT).await
        }
        .await;
        let close_result = page.close().await;
        match (result, close_result) {
            (Ok(detail), Ok(())) => Ok(detail),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error.into()),
        }
    }

    async fn result_page(&self) -> Result<RawResultPage> {
        self.eval(RESULT_PAGE_SCRIPT).await
    }

    async fn current_url(&self) -> Result<String> {
        self.eval("() => location.href").await
    }

    async fn wait_for_catalog<F>(&self, ready: F) -> Result<FilterCatalog>
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

    async fn wait_for_condition(&self, script: &str, description: &str) -> Result<()> {
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

    async fn goto_with_retries(
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

    async fn goto(&self, url: Url) -> Result<()> {
        let trusted = TrustedZiroomUrl::parse(url)?;
        self.goto_page(&self.page, trusted).await
    }

    async fn goto_page(&self, page: &Page, url: TrustedZiroomUrl) -> Result<()> {
        tokio::time::timeout(self.timeout, page.goto(url.as_url().as_str()))
            .await
            .context("navigation timed out")??;
        self.wait_for_site_on(page).await
    }

    async fn wait_for_site(&self) -> Result<()> {
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

    async fn eval<T: DeserializeOwned>(&self, script: &str) -> Result<T> {
        self.eval_on(&self.page, script).await
    }

    async fn eval_on<T: DeserializeOwned>(&self, page: &Page, script: &str) -> Result<T> {
        page.evaluate(script)
            .await?
            .into_value()
            .context("browser script returned an unexpected value")
    }
}

async fn cancelable_sleep(cancel: &CancellationToken, duration: Duration) -> Result<()> {
    tokio::select! {
        () = cancel.cancelled() => bail!("scrape cancelled"),
        () = sleep(duration) => Ok(()),
    }
}

const FILTER_CATALOG_SCRIPT: &str = include_str!("scripts/filter_catalog.js");
const RESULT_PAGE_SCRIPT: &str = include_str!("scripts/result_page.js");
const DETAIL_SCRIPT: &str = include_str!("scripts/detail.js");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrape_tracker_stops_loops_limits_pages_and_deduplicates_listings() {
        let first = Url::parse("https://www.ziroom.com/z/").unwrap();
        let mut tracker = ScrapeTracker::default();
        assert_eq!(tracker.begin_page(&first), PageStart::Visit(1));
        assert_eq!(tracker.begin_page(&first), PageStart::Loop);
        assert!(tracker.accept_listing("one"));
        assert!(!tracker.accept_listing("one"));

        let mut tracker = ScrapeTracker::default();
        for page in 1..=100 {
            let url = Url::parse(&format!("https://www.ziroom.com/z/p{page}/")).unwrap();
            assert_eq!(tracker.begin_page(&url), PageStart::Visit(page));
        }
        let extra = Url::parse("https://www.ziroom.com/z/p101/").unwrap();
        assert_eq!(tracker.begin_page(&extra), PageStart::Limit);
    }

    #[tokio::test]
    #[ignore = "opt-in live Ziroom smoke test"]
    async fn discovers_live_beijing_filters() {
        assert_eq!(std::env::var("NEST_LIVE_TESTS").as_deref(), Ok("1"));
        let launch = ZiroomClient::launch(BrowserSettings {
            browser_path: std::env::var_os("NEST_BROWSER").map(Into::into),
            headed: std::env::var_os("NEST_LIVE_HEADED").is_some(),
            navigation_timeout: Duration::from_secs(60),
        })
        .await;
        let client = match launch {
            Ok(client) => client,
            Err(error)
                if error
                    .downcast_ref::<BrowserError>()
                    .is_some_and(|value| matches!(value, BrowserError::ChallengeRequired)) =>
            {
                eprintln!("Ziroom challenge detected; the TUI will retry visibly");
                return;
            }
            Err(error) => panic!("launch browser and load Ziroom: {error:#}"),
        };
        let catalog = client.discover().await.expect("discover filter catalog");
        assert!(catalog.districts.iter().any(|value| value.label == "海淀"));
        assert!(
            catalog
                .rental_types
                .iter()
                .any(|value| value.label == "整租")
        );
        assert!(catalog.bedrooms.iter().any(|value| value.label == "3居+"));
        client.close().await.expect("close browser");
    }
}
