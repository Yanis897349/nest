use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct PriceGlyph {
    pub(crate) sprite_url: Url,
    pub(crate) background_x_px: f32,
    pub(crate) background_size_px: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct RawListingCard {
    pub(crate) listing_id: String,
    pub(crate) title: String,
    pub(crate) detail_url: Url,
    pub(crate) area_floor_orientation: String,
    pub(crate) location_or_transit: String,
    pub(crate) tags: Vec<String>,
    pub(crate) image_url: Option<Url>,
    pub(crate) price_glyphs: Vec<PriceGlyph>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct RawResultPage {
    pub(crate) source_url: Url,
    pub(crate) reported_total: Option<usize>,
    pub(crate) cards: Vec<RawListingCard>,
    pub(crate) next_page_url: Option<Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ListingSummary {
    pub(crate) listing_id: String,
    pub(crate) title: String,
    pub(crate) monthly_rent_cny: Option<u32>,
    pub(crate) area_sqm: Option<f32>,
    pub(crate) floor: Option<String>,
    pub(crate) orientation: Option<String>,
    pub(crate) location: Option<String>,
    pub(crate) transit: Option<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) image_url: Option<Url>,
    pub(crate) detail_url: Url,
    pub(crate) source_page_url: Url,
    pub(crate) scraped_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ListingDetail {
    pub(crate) listing_id: String,
    pub(crate) status: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) facts: BTreeMap<String, String>,
    pub(crate) amenities: Vec<String>,
    pub(crate) image_urls: Vec<Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ScrapeReport {
    pub(crate) schema_version: u8,
    pub(crate) scraped_at: DateTime<Utc>,
    pub(crate) source_url: Url,
    pub(crate) selected_filters: Vec<String>,
    pub(crate) reported_total: Option<usize>,
    pub(crate) warnings: Vec<String>,
    pub(crate) listings: Vec<ListingSummary>,
}

impl ScrapeReport {
    pub(crate) fn empty(source_url: Url) -> Self {
        Self {
            schema_version: 1,
            scraped_at: Utc::now(),
            source_url,
            selected_filters: Vec::new(),
            reported_total: None,
            warnings: Vec::new(),
            listings: Vec::new(),
        }
    }
}

pub(crate) fn normalize_listing(
    raw: RawListingCard,
    monthly_rent_cny: Option<u32>,
    source_page_url: Url,
    scraped_at: DateTime<Utc>,
) -> ListingSummary {
    let parts: Vec<_> = raw
        .area_floor_orientation
        .split('|')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    let area_sqm = parts.first().and_then(|value| parse_area(value));
    let floor = parts.get(1).map(|value| (*value).to_owned());
    let orientation = parts.get(2).map(|value| (*value).to_owned());

    let location_text = raw.location_or_transit.trim();
    let is_transit = location_text.starts_with('距')
        || location_text.contains("站步行")
        || location_text.contains("地铁");

    ListingSummary {
        listing_id: raw.listing_id,
        title: raw.title,
        monthly_rent_cny,
        area_sqm,
        floor,
        orientation,
        location: (!location_text.is_empty() && !is_transit).then(|| location_text.to_owned()),
        transit: (!location_text.is_empty() && is_transit).then(|| location_text.to_owned()),
        tags: raw.tags,
        image_url: raw.image_url,
        detail_url: raw.detail_url,
        source_page_url,
        scraped_at,
    }
}

fn parse_area(value: &str) -> Option<f32> {
    let cleaned = value
        .trim()
        .trim_start_matches('约')
        .trim_end_matches("㎡")
        .trim();
    cleaned.parse().ok()
}
