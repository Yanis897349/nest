use chrono::{DateTime, Utc};

use crate::model::{ListingSummary, RawListingCard};

pub fn normalize_listing(
    raw: RawListingCard,
    monthly_rent_cny: Option<u32>,
    source_page_url: url::Url,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RawListingCard;

    #[test]
    fn parses_card_metadata_without_losing_chinese_labels() {
        let detail = url::Url::parse("https://www.ziroom.com/x/810907652.html").unwrap();
        let source = url::Url::parse("https://www.ziroom.com/z/").unwrap();
        let listing = normalize_listing(
            RawListingCard {
                listing_id: "810907652".into(),
                title: "东馨园3居".into(),
                detail_url: detail,
                area_floor_orientation: "约81.9㎡ | 4/6层 | 朝南".into(),
                location_or_transit: "距18号线上地软件园站步行约646米".into(),
                tags: vec!["独立阳台".into()],
                image_url: None,
                price_glyphs: vec![],
            },
            Some(8190),
            source,
            Utc::now(),
        );
        assert_eq!(listing.area_sqm, Some(81.9));
        assert_eq!(listing.floor.as_deref(), Some("4/6层"));
        assert_eq!(listing.orientation.as_deref(), Some("朝南"));
        assert!(listing.location.is_none());
        assert!(listing.transit.unwrap().contains("18号线"));
    }
}
