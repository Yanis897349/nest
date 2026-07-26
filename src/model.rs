use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilterOption {
    pub label: String,
    pub url: Option<Url>,
    #[serde(default)]
    pub selected: bool,
}

impl FilterOption {
    pub fn new(label: impl Into<String>, url: Option<Url>) -> Self {
        Self {
            label: label.into(),
            url,
            selected: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocationMode {
    #[default]
    Region,
    Subway,
    Commute,
}

impl LocationMode {
    pub const ALL: [Self; 3] = [Self::Region, Self::Subway, Self::Commute];

    pub fn label(self) -> &'static str {
        match self {
            Self::Region => "区域",
            Self::Subway => "地铁",
            Self::Commute => "通勤",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilterCatalog {
    pub mode: LocationMode,
    pub districts: Vec<FilterOption>,
    pub business_areas: Vec<FilterOption>,
    pub subway_lines: Vec<FilterOption>,
    pub stations: Vec<FilterOption>,
    pub rental_types: Vec<FilterOption>,
    pub rent_presets: Vec<FilterOption>,
    pub bedrooms: Vec<FilterOption>,
    pub sort_orders: Vec<FilterOption>,
    pub selected_labels: Vec<String>,
    pub current_url: Url,
}

impl Default for FilterCatalog {
    fn default() -> Self {
        Self {
            mode: LocationMode::Region,
            districts: Vec::new(),
            business_areas: Vec::new(),
            subway_lines: Vec::new(),
            stations: Vec::new(),
            rental_types: Vec::new(),
            rent_presets: Vec::new(),
            bedrooms: Vec::new(),
            sort_orders: Vec::new(),
            selected_labels: Vec::new(),
            current_url: Url::parse(crate::ZIROOM_BASE_URL).expect("valid Ziroom base URL"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriceGlyph {
    pub sprite_url: Url,
    pub background_x_px: f32,
    pub background_size_px: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawListingCard {
    pub listing_id: String,
    pub title: String,
    pub detail_url: Url,
    pub area_floor_orientation: String,
    pub location_or_transit: String,
    pub tags: Vec<String>,
    pub image_url: Option<Url>,
    pub price_glyphs: Vec<PriceGlyph>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawResultPage {
    pub source_url: Url,
    pub reported_total: Option<usize>,
    pub cards: Vec<RawListingCard>,
    pub next_page_url: Option<Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListingSummary {
    pub listing_id: String,
    pub title: String,
    pub monthly_rent_cny: Option<u32>,
    pub area_sqm: Option<f32>,
    pub floor: Option<String>,
    pub orientation: Option<String>,
    pub location: Option<String>,
    pub transit: Option<String>,
    pub tags: Vec<String>,
    pub image_url: Option<Url>,
    pub detail_url: Url,
    pub source_page_url: Url,
    pub scraped_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListingDetail {
    pub listing_id: String,
    pub status: Option<String>,
    pub description: Option<String>,
    pub facts: BTreeMap<String, String>,
    pub amenities: Vec<String>,
    pub image_urls: Vec<Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScrapeReport {
    pub schema_version: u8,
    pub scraped_at: DateTime<Utc>,
    pub source_url: Url,
    pub selected_filters: Vec<String>,
    pub reported_total: Option<usize>,
    pub warnings: Vec<String>,
    pub listings: Vec<ListingSummary>,
}

impl ScrapeReport {
    pub fn empty(source_url: Url) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_result_fixture_deserializes_with_stable_unicode_fields() {
        let page: RawResultPage =
            serde_json::from_str(include_str!("../tests/fixtures/raw_page.json"))
                .expect("valid result fixture");
        assert_eq!(page.reported_total, Some(36));
        assert_eq!(page.cards[0].title, "东馨园3居");
        assert_eq!(page.cards[0].price_glyphs.len(), 4);
        assert!(page.next_page_url.unwrap().path().contains("p2"));
    }

    #[test]
    fn malformed_extraction_payload_is_rejected() {
        let error = serde_json::from_str::<RawResultPage>(
            r#"{"source_url":"not a URL","cards":[],"next_page_url":null}"#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("relative URL"));
    }

    #[test]
    fn catalog_and_detail_fixtures_preserve_extraction_contracts() {
        let catalog: FilterCatalog =
            serde_json::from_str(include_str!("../tests/fixtures/catalog.json")).unwrap();
        let detail: ListingDetail =
            serde_json::from_str(include_str!("../tests/fixtures/detail.json")).unwrap();
        assert_eq!(catalog.mode, LocationMode::Region);
        assert_eq!(catalog.selected_labels, ["海淀"]);
        assert_eq!(detail.status.as_deref(), Some("可预订"));
        assert_eq!(detail.facts.get("面积").map(String::as_str), Some("81.9㎡"));
    }
}
