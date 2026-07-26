mod protocol;
mod runtime;
mod state;
mod worker;

use std::{path::PathBuf, time::Duration};

pub use runtime::run;

pub(crate) use crate::export::ExportFormat;
pub(crate) use protocol::{InputMode, Screen};
pub(crate) use state::AppState;

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub browser_path: Option<PathBuf>,
    pub headed: bool,
    pub output_dir: PathBuf,
    pub navigation_timeout: Duration,
}
