mod serialization;

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Local};

use self::serialization::{write_csv, write_json};
use crate::model::ScrapeReport;

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

#[cfg(test)]
mod tests;
