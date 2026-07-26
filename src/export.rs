use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Local};
use serde::Serialize;

use crate::model::{ListingSummary, ScrapeReport};

const MAX_FILENAME_ATTEMPTS: usize = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExportFormat {
    Json,
    Csv,
}

impl ExportFormat {
    pub(crate) const fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Csv => "csv",
        }
    }
}

pub(crate) fn export_report(
    report: &ScrapeReport,
    output_dir: &Path,
    format: ExportFormat,
) -> Result<PathBuf> {
    export_report_at(report, output_dir, format, Local::now())
}

fn export_report_at(
    report: &ScrapeReport,
    output_dir: &Path,
    format: ExportFormat,
    now: DateTime<Local>,
) -> Result<PathBuf> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create export directory {}", output_dir.display()))?;
    let timestamp = now.format("%Y%m%d-%H%M%S");
    let mut temporary = tempfile::Builder::new()
        .prefix(".nest-export-")
        .tempfile_in(output_dir)
        .with_context(|| {
            format!(
                "failed to create temporary export in {}",
                output_dir.display()
            )
        })?;

    match format {
        ExportFormat::Json => write_json(report, temporary.as_file_mut())?,
        ExportFormat::Csv => write_csv(report, temporary.as_file_mut())?,
    }
    temporary
        .as_file_mut()
        .flush()
        .context("failed to flush temporary export")?;
    temporary
        .as_file()
        .sync_all()
        .context("failed to sync temporary export")?;

    for attempt in 0..MAX_FILENAME_ATTEMPTS {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("-{attempt}")
        };
        let path = output_dir.join(format!(
            "ziroom-beijing-{timestamp}{suffix}.{}",
            format.extension()
        ));
        match temporary.persist_noclobber(&path) {
            Ok(_) => return Ok(path),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                temporary = error.file;
            }
            Err(error) => {
                return Err(error.error)
                    .with_context(|| format!("failed to persist export as {}", path.display()));
            }
        }
    }
    Err(anyhow!(
        "could not choose an unused export filename after {MAX_FILENAME_ATTEMPTS} attempts"
    ))
}

fn write_json(report: &ScrapeReport, writer: impl Write) -> Result<()> {
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

fn write_csv(report: &ScrapeReport, writer: impl Write) -> Result<()> {
    let mut writer = csv::Writer::from_writer(writer);
    for listing in &report.listings {
        writer
            .serialize(CsvListing::from(listing))
            .with_context(|| format!("failed to serialize listing {}", listing.listing_id))?;
    }
    writer.flush().context("failed to flush CSV export")
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone, Utc};
    use tempfile::tempdir;
    use url::Url;

    use super::*;

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
    fn writes_json_and_csv_with_unicode_and_stable_headers() {
        let dir = tempdir().unwrap();
        let now = Local.with_ymd_and_hms(2026, 7, 26, 12, 0, 0).unwrap();
        let json = export_report_at(&report(), dir.path(), ExportFormat::Json, now).unwrap();
        let csv = export_report_at(&report(), dir.path(), ExportFormat::Csv, now).unwrap();

        let json_text = fs::read_to_string(json).unwrap();
        let csv_text = fs::read_to_string(csv).unwrap();
        assert!(json_text.contains("\"schema_version\": 1"));
        assert!(json_text.contains("东馨园3居"));
        assert!(csv_text.starts_with("listing_id,title,monthly_rent_cny,area_sqm"));
        assert!(csv_text.contains("\"东馨园3居, 朝南\""));
        assert!(csv_text.contains("独立阳台|南北通透"));
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
}
