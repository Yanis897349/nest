use std::io::Write;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::model::{ListingSummary, ScrapeReport};

pub(super) fn write_json(report: &ScrapeReport, writer: impl Write) -> Result<()> {
    serde_json::to_writer_pretty(writer, report).context("failed to serialize JSON export")
}

#[derive(Serialize)]
struct CsvListing<'a> {
    listing_id: &'a str,
    title: &'a str,
    monthly_rent_cny: Option<u32>,
    area_sqm: Option<f32>,
    floor: &'a str,
    orientation: &'a str,
    location: &'a str,
    transit: &'a str,
    tags: String,
    image_url: String,
    detail_url: String,
    source_page_url: String,
    scraped_at: String,
}

impl<'a> From<&'a ListingSummary> for CsvListing<'a> {
    fn from(listing: &'a ListingSummary) -> Self {
        Self {
            listing_id: &listing.listing_id,
            title: &listing.title,
            monthly_rent_cny: listing.monthly_rent_cny,
            area_sqm: listing.area_sqm,
            floor: listing.floor.as_deref().unwrap_or_default(),
            orientation: listing.orientation.as_deref().unwrap_or_default(),
            location: listing.location.as_deref().unwrap_or_default(),
            transit: listing.transit.as_deref().unwrap_or_default(),
            tags: listing.tags.join("|"),
            image_url: listing
                .image_url
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            detail_url: listing.detail_url.to_string(),
            source_page_url: listing.source_page_url.to_string(),
            scraped_at: listing.scraped_at.to_rfc3339(),
        }
    }
}

pub(super) fn write_csv(report: &ScrapeReport, writer: impl Write) -> Result<()> {
    let mut writer = csv::Writer::from_writer(writer);
    for listing in &report.listings {
        writer
            .serialize(CsvListing::from(listing))
            .with_context(|| format!("failed to serialize listing {}", listing.listing_id))?;
    }
    writer.flush().context("failed to flush CSV export")
}
