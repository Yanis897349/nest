use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use tracing::Level;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, Parser)]
#[command(
    name = "nest",
    version,
    about = "Browse and export Beijing Ziroom listings"
)]
struct Cli {
    #[arg(
        long,
        env = "NEST_BROWSER",
        help = "Chromium-family browser executable or macOS .app bundle"
    )]
    browser: Option<PathBuf>,

    #[arg(long, help = "Always show the managed browser window")]
    headed: bool,

    #[arg(
        long,
        default_value = ".",
        help = "Directory used for JSON and CSV exports"
    )]
    output_dir: PathBuf,

    #[arg(
        long,
        default_value_t = 45,
        value_parser = clap::value_parser!(u64).range(5..=300),
        help = "Per-navigation timeout in seconds"
    )]
    navigation_timeout: u64,

    #[arg(
        long,
        env = "NEST_LOG_FILE",
        help = "Append newline-delimited JSON diagnostics to this file"
    )]
    log_file: Option<PathBuf>,

    #[arg(
        long,
        env = "NEST_LOG_LEVEL",
        value_enum,
        default_value_t = LogLevel::Info,
        help = "Diagnostic verbosity used when --log-file is set"
    )]
    log_level: LogLevel,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    const fn tracing_level(self) -> Level {
        match self {
            Self::Error => Level::ERROR,
            Self::Warn => Level::WARN,
            Self::Info => Level::INFO,
            Self::Debug => Level::DEBUG,
            Self::Trace => Level::TRACE,
        }
    }
}

pub(crate) async fn run() -> Result<()> {
    let cli = Cli::parse();
    init_logging(cli.log_file.as_deref(), cli.log_level)?;
    nest::run(nest::RunConfig {
        browser_path: cli.browser,
        headed: cli.headed,
        output_dir: cli.output_dir,
        navigation_timeout: Duration::from_secs(cli.navigation_timeout),
    })
    .await
}

fn init_logging(path: Option<&Path>, level: LogLevel) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log directory {}", parent.display()))?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open log file {}", path.display()))?;
    tracing_subscriber::fmt()
        .json()
        .with_ansi(false)
        .with_max_level(level.tracing_level())
        .with_writer(Mutex::new(file))
        .finish()
        .try_init()
        .context("failed to initialize diagnostic logging")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_preserves_defaults() {
        let cli = Cli::try_parse_from(["nest"]).unwrap();
        assert!(!cli.headed);
        assert_eq!(cli.output_dir, PathBuf::from("."));
        assert_eq!(cli.navigation_timeout, 45);
        assert!(cli.log_file.is_none());
        assert!(matches!(cli.log_level, LogLevel::Info));
    }

    #[test]
    fn cli_parses_diagnostics_and_runtime_options() {
        let cli = Cli::try_parse_from([
            "nest",
            "--headed",
            "--navigation-timeout",
            "60",
            "--output-dir",
            "exports",
            "--log-file",
            "logs/nest.jsonl",
            "--log-level",
            "debug",
        ])
        .unwrap();
        assert!(cli.headed);
        assert_eq!(cli.navigation_timeout, 60);
        assert_eq!(cli.output_dir, PathBuf::from("exports"));
        assert_eq!(cli.log_file, Some(PathBuf::from("logs/nest.jsonl")));
        assert!(matches!(cli.log_level, LogLevel::Debug));
    }

    #[test]
    fn cli_rejects_navigation_timeout_outside_bounds() {
        assert!(Cli::try_parse_from(["nest", "--navigation-timeout", "4"]).is_err());
        assert!(Cli::try_parse_from(["nest", "--navigation-timeout", "301"]).is_err());
    }
}
