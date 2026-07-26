use std::fs;

use chrono::{Local, TimeZone, Utc};
use tempfile::tempdir;
use url::Url;

use super::{ExportFormat, export_report_at, serialization};
use crate::model::{ListingSummary, ScrapeReport};

fn report() -> ScrapeReport {
    let source = Url::parse("https://www.ziroom.com/z/").unwrap();
    ScrapeReport {
        schema_version: 1,
        scraped_at: Utc::now(),
        source_url: source.clone(),
        selected_filters: vec!["海淀".into()],
        reported_total: Some(1),
        warnings: vec![],
        listings: vec![ListingSummary {
            listing_id: "810907652".into(),
            title: "东馨园3居, 朝南".into(),
            monthly_rent_cny: Some(8190),
            area_sqm: Some(81.9),
            floor: Some("4/6层".into()),
            orientation: Some("朝南".into()),
            location: Some("海淀".into()),
            transit: Some("距18号线约646米".into()),
            tags: vec!["独立阳台".into(), "南北通透".into()],
            image_url: None,
            detail_url: Url::parse("https://www.ziroom.com/x/810907652.html").unwrap(),
            source_page_url: source,
            scraped_at: Utc::now(),
        }],
    }
}

#[test]
fn serializers_preserve_unicode_and_stable_headers() {
    let mut json = Vec::new();
    let mut csv = Vec::new();
    serialization::write_json(&report(), &mut json).unwrap();
    serialization::write_csv(&report(), &mut csv).unwrap();
    let json = String::from_utf8(json).unwrap();
    let csv = String::from_utf8(csv).unwrap();
    assert!(json.contains("\"schema_version\": 1"));
    assert!(json.contains("东馨园3居"));
    assert!(csv.starts_with("listing_id,title,monthly_rent_cny,area_sqm"));
    assert!(csv.contains("\"东馨园3居, 朝南\""));
    assert!(csv.contains("独立阳台|南北通透"));
}

#[test]
fn writes_json_and_csv_with_expected_names() {
    let dir = tempdir().unwrap();
    let now = Local.with_ymd_and_hms(2026, 7, 26, 12, 0, 0).unwrap();
    let json = export_report_at(&report(), dir.path(), ExportFormat::Json, now).unwrap();
    let csv = export_report_at(&report(), dir.path(), ExportFormat::Csv, now).unwrap();
    assert_eq!(
        json.file_name().unwrap(),
        "ziroom-beijing-20260726-120000.json"
    );
    assert_eq!(
        csv.file_name().unwrap(),
        "ziroom-beijing-20260726-120000.csv"
    );
}

#[test]
fn collisions_get_a_suffix_without_overwriting() {
    let dir = tempdir().unwrap();
    let now = Local.with_ymd_and_hms(2026, 7, 26, 12, 0, 0).unwrap();
    let first = export_report_at(&report(), dir.path(), ExportFormat::Json, now).unwrap();
    let first_contents = fs::read(&first).unwrap();
    let second = export_report_at(&report(), dir.path(), ExportFormat::Json, now).unwrap();
    assert_ne!(first, second);
    assert!(
        second
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("-1.json")
    );
    assert_eq!(fs::read(first).unwrap(), first_contents);
}
