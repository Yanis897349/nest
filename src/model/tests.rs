use chrono::Utc;
use url::Url;

use super::listings::RawListingCard;
use super::{FilterCatalog, ListingDetail, LocationMode, RawResultPage, normalize_listing};

#[test]
fn current_result_fixture_deserializes_with_stable_unicode_fields() {
    let page: RawResultPage =
        serde_json::from_str(include_str!("../../tests/fixtures/raw_page.json"))
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
        serde_json::from_str(include_str!("../../tests/fixtures/catalog.json")).unwrap();
    let detail: ListingDetail =
        serde_json::from_str(include_str!("../../tests/fixtures/detail.json")).unwrap();
    assert_eq!(catalog.mode, LocationMode::Region);
    assert_eq!(catalog.selected_labels, ["海淀"]);
    assert_eq!(detail.status.as_deref(), Some("可预订"));
    assert_eq!(detail.facts.get("面积").map(String::as_str), Some("81.9㎡"));
}

#[test]
fn normalizes_card_metadata_without_losing_chinese_labels() {
    let detail = Url::parse("https://www.ziroom.com/x/810907652.html").unwrap();
    let source = Url::parse("https://www.ziroom.com/z/").unwrap();
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
