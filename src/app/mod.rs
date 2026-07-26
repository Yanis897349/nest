mod state;
mod worker;

use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result, anyhow};
use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;
use tokio_util::sync::CancellationToken;

use self::{
    state::{Action, AppState, BackendEvent, BackendRequest, Effect},
    worker::WorkerHandle,
};
use crate::{
    export, ui,
    ziroom::{BrowserSettings, validate_ziroom_url},
};

pub(crate) use crate::export::ExportFormat;
pub(crate) use state::AppState as ViewState;
pub(crate) use state::{InputMode, Screen};

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub browser_path: Option<PathBuf>,
    pub headed: bool,
    pub output_dir: PathBuf,
    pub navigation_timeout: Duration,
}

pub async fn run(config: RunConfig) -> Result<()> {
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        headed = config.headed,
        "starting nest"
    );
    let mut terminal = ui::TerminalGuard::new()?;
    let mut terminal_events = EventStream::new();
    let mut state = AppState::new();
    let mut worker = WorkerHandle::spawn(BrowserSettings {
        browser_path: config.browser_path.clone(),
        headed: config.headed,
        navigation_timeout: config.navigation_timeout,
    });
    let mut active_scrape: Option<CancellationToken> = None;
    let mut worker_open = true;

    worker
        .send(BackendRequest::Initialize, None)
        .await
        .context("failed to initialize browser worker")?;

    let loop_result: Result<()> = async {
        while !state.quit {
            terminal.draw(|frame| ui::render(frame, &state))?;
            tokio::select! {
                terminal_event = terminal_events.next() => {
                    match terminal_event {
                        Some(Ok(Event::Key(key)))
                            if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                        {
                            let effects = state.update(Action::Key(key));
                            execute_effects(
                                effects,
                                &mut state,
                                &worker,
                                &config.output_dir,
                                &mut active_scrape,
                            ).await;
                        }
                        Some(Ok(Event::Resize(_, _))) => {}
                        Some(Ok(_)) => {}
                        Some(Err(error)) => return Err(error.into()),
                        None => return Err(anyhow!("terminal event stream closed")),
                    }
                }
                backend_event = worker.recv(), if worker_open => {
                    match backend_event {
                        Some(event) => {
                            if matches!(
                                event,
                                BackendEvent::Report(_)
                                    | BackendEvent::Cancelled
                                    | BackendEvent::Challenge
                                    | BackendEvent::Error(_)
                            ) {
                                active_scrape = None;
                            }
                            state.update(Action::Backend(Box::new(event)));
                        }
                        None => {
                            worker_open = false;
                            state.update(Action::Backend(Box::new(BackendEvent::Error(
                                "browser worker stopped unexpectedly".into(),
                            ))));
                        }
                    }
                }
            }
        }
        Ok(())
    }
    .await;

    if let Some(cancel) = active_scrape.take() {
        cancel.cancel();
    }
    worker.shutdown().await;
    tracing::info!("nest stopped");
    loop_result
}

async fn execute_effects(
    effects: Vec<Effect>,
    state: &mut AppState,
    worker: &WorkerHandle,
    output_dir: &std::path::Path,
    active_scrape: &mut Option<CancellationToken>,
) {
    for effect in effects {
        let result = match effect {
            Effect::Backend(request) => {
                let cancel = if matches!(request, BackendRequest::Scrape) {
                    let token = CancellationToken::new();
                    *active_scrape = Some(token.clone());
                    Some(token)
                } else {
                    None
                };
                worker.send(request, cancel).await
            }
            Effect::CancelScrape => {
                if let Some(cancel) = active_scrape.as_ref() {
                    cancel.cancel();
                }
                Ok(())
            }
            Effect::OpenUrl(url) => validate_ziroom_url(&url)
                .and_then(|()| open::that(url.as_str()).map_err(Into::into))
                .map(|()| {
                    state.status = "Opened listing in the system browser".into();
                }),
            Effect::Export(format) => {
                let Some(report) = state.report.clone() else {
                    state.update(Action::Exported(Err("nothing to export".into())));
                    continue;
                };
                let output_dir = output_dir.to_path_buf();
                let result = tokio::task::spawn_blocking(move || {
                    export::export_report(&report, &output_dir, format)
                })
                .await
                .map_err(|error| anyhow!("export task failed: {error}"))
                .and_then(|result| result);
                state.update(Action::Exported(
                    result.map_err(|error| format!("{error:#}")),
                ));
                Ok(())
            }
            Effect::Quit => Ok(()),
        };
        if let Err(error) = result {
            tracing::error!(error = %error, "application effect failed");
            state.update(Action::Backend(Box::new(BackendEvent::Error(format!(
                "{error:#}"
            )))));
        }
    }
}
