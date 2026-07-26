use std::{collections::HashSet, time::Duration};

use anyhow::{Result, bail};
use chrono::Utc;
use tokio_util::sync::CancellationToken;
use url::Url;

use super::{BrowserProgress, ZiroomClient, navigation::cancelable_sleep};
use crate::model::{RawResultPage, ScrapeReport, normalize_listing};

#[derive(Debug, Default)]
pub(super) struct ScrapeTracker {
    visited_pages: HashSet<Url>,
    seen_listings: HashSet<String>,
    pages: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PageStart {
    Visit(usize),
    Loop,
    Limit,
}

impl ScrapeTracker {
    pub(super) fn begin_page(&mut self, url: &Url) -> PageStart {
        if self.pages >= 100 {
            return PageStart::Limit;
        }
        if !self.visited_pages.insert(url.clone()) {
            return PageStart::Loop;
        }
        self.pages += 1;
        PageStart::Visit(self.pages)
    }

    pub(super) fn accept_listing(&mut self, listing_id: &str) -> bool {
        self.seen_listings.insert(listing_id.to_owned())
    }
}

impl ZiroomClient {
    pub(crate) async fn scrape_all(
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

    async fn result_page(&self) -> Result<RawResultPage> {
        self.eval(RESULT_PAGE_SCRIPT).await
    }
}

const RESULT_PAGE_SCRIPT: &str = include_str!("../scripts/result_page.js");
