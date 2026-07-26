use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use tempfile::TempDir;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use url::Url;

use super::state::{BackendEvent, BackendRequest};
use crate::{
    ZIROOM_BASE_URL,
    ziroom::{BrowserError, BrowserSettings, ZiroomClient},
};

const COMMAND_CAPACITY: usize = 8;
const EVENT_CAPACITY: usize = 32;
const PROGRESS_CAPACITY: usize = 16;

enum WorkerCommand {
    Request(BackendRequest, Option<CancellationToken>),
    Shutdown,
}

pub(crate) struct WorkerHandle {
    commands: mpsc::Sender<WorkerCommand>,
    events: mpsc::Receiver<BackendEvent>,
    join: Option<JoinHandle<()>>,
}

impl WorkerHandle {
    pub(crate) fn spawn(settings: BrowserSettings) -> Self {
        let (command_tx, command_rx) = mpsc::channel(COMMAND_CAPACITY);
        let (event_tx, event_rx) = mpsc::channel(EVENT_CAPACITY);
        let join = tokio::spawn(worker_loop(settings, command_rx, event_tx));
        Self {
            commands: command_tx,
            events: event_rx,
            join: Some(join),
        }
    }

    pub(crate) async fn send(
        &self,
        request: BackendRequest,
        cancel: Option<CancellationToken>,
    ) -> Result<()> {
        self.commands
            .send(WorkerCommand::Request(request, cancel))
            .await
            .map_err(|_| anyhow!("browser worker is no longer running"))
    }

    pub(crate) async fn recv(&mut self) -> Option<BackendEvent> {
        self.events.recv().await
    }

    pub(crate) async fn shutdown(mut self) {
        let _ = tokio::time::timeout(
            Duration::from_secs(1),
            self.commands.send(WorkerCommand::Shutdown),
        )
        .await;
        drop(self.commands);
        if let Some(mut join) = self.join.take()
            && tokio::time::timeout(Duration::from_secs(10), &mut join)
                .await
                .is_err()
        {
            tracing::warn!("browser worker did not stop within the shutdown deadline");
            join.abort();
            let _ = join.await;
        }
    }
}

async fn worker_loop(
    mut settings: BrowserSettings,
    mut commands: mpsc::Receiver<WorkerCommand>,
    events: mpsc::Sender<BackendEvent>,
) {
    let profile = match tempfile::Builder::new().prefix("nest-browser-").tempdir() {
        Ok(profile) => profile,
        Err(error) => {
            let _ = events
                .send(BackendEvent::Error(format!(
                    "could not create a temporary browser profile: {error}"
                )))
                .await;
            return;
        }
    };
    let mut client: Option<ZiroomClient> = None;

    while let Some(command) = commands.recv().await {
        match command {
            WorkerCommand::Shutdown => break,
            WorkerCommand::Request(request, cancel) => {
                let result = handle_request(
                    request,
                    cancel,
                    &mut settings,
                    &profile,
                    &mut client,
                    &events,
                )
                .await;
                if let Err(error) = result {
                    let event = if error
                        .downcast_ref::<BrowserError>()
                        .is_some_and(|value| matches!(value, BrowserError::ChallengeRequired))
                    {
                        BackendEvent::Challenge
                    } else {
                        BackendEvent::Error(format!("{error:#}"))
                    };
                    if events.send(event).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    if let Some(active) = client.take()
        && let Err(error) = active.close().await
    {
        tracing::warn!(%error, "failed to close managed browser cleanly");
    }
}

async fn handle_request(
    request: BackendRequest,
    cancel: Option<CancellationToken>,
    settings: &mut BrowserSettings,
    profile: &TempDir,
    client: &mut Option<ZiroomClient>,
    events: &mpsc::Sender<BackendEvent>,
) -> Result<()> {
    match request {
        BackendRequest::Initialize => {
            launch(settings, profile, client, events).await?;
        }
        BackendRequest::RetryHeaded => {
            settings.headed = true;
            if let Some(active) = client.take() {
                active.close().await?;
            }
            launch(settings, profile, client, events).await?;
        }
        BackendRequest::Reset => {
            let active = ready(client)?;
            let catalog = active.navigate_filter(Url::parse(ZIROOM_BASE_URL)?).await?;
            send(events, BackendEvent::Catalog(catalog)).await?;
        }
        BackendRequest::Navigate(url) => {
            let catalog = ready(client)?.navigate_filter(url).await?;
            send(events, BackendEvent::Catalog(catalog)).await?;
        }
        BackendRequest::SetMode(mode) => {
            let catalog = ready(client)?.set_location_mode(mode).await?;
            send(events, BackendEvent::Catalog(catalog)).await?;
        }
        BackendRequest::SetCustomRent(min, max) => {
            let catalog = ready(client)?.set_custom_rent(min, max).await?;
            send(events, BackendEvent::Catalog(catalog)).await?;
        }
        BackendRequest::SetCommute(address, minutes) => {
            let catalog = ready(client)?.set_commute(&address, minutes).await?;
            send(events, BackendEvent::Catalog(catalog)).await?;
        }
        BackendRequest::Scrape => {
            let cancel = cancel.context("scrape request did not include cancellation state")?;
            let (progress_tx, mut progress_rx) = mpsc::channel(PROGRESS_CAPACITY);
            let forward = events.clone();
            let forwarder = tokio::spawn(async move {
                while let Some(progress) = progress_rx.recv().await {
                    if forward
                        .send(BackendEvent::Progress(progress))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            });
            let scrape_result = ready_mut(client)?.scrape_all(&cancel, &progress_tx).await;
            drop(progress_tx);
            let _ = forwarder.await;
            if cancel.is_cancelled() {
                send(events, BackendEvent::Cancelled).await?;
            } else {
                send(events, BackendEvent::Report(scrape_result?)).await?;
            }
        }
        BackendRequest::Detail(url) => {
            let detail = ready(client)?.fetch_detail(url).await?;
            send(events, BackendEvent::Detail(detail)).await?;
        }
    }
    Ok(())
}

async fn launch(
    settings: &BrowserSettings,
    profile: &TempDir,
    client: &mut Option<ZiroomClient>,
    events: &mpsc::Sender<BackendEvent>,
) -> Result<()> {
    send(
        events,
        BackendEvent::Status(format!(
            "Launching {} browser…",
            if settings.headed {
                "visible"
            } else {
                "headless"
            }
        )),
    )
    .await?;
    let new_client =
        ZiroomClient::launch_with_profile(settings.clone(), Some(profile.path())).await?;
    match new_client.discover().await {
        Ok(catalog) => {
            *client = Some(new_client);
            send(events, BackendEvent::Catalog(catalog)).await
        }
        Err(error) => {
            let _ = new_client.close().await;
            Err(error)
        }
    }
}

fn ready(client: &Option<ZiroomClient>) -> Result<&ZiroomClient> {
    client
        .as_ref()
        .ok_or_else(|| anyhow!("browser is not ready"))
}

fn ready_mut(client: &mut Option<ZiroomClient>) -> Result<&mut ZiroomClient> {
    client
        .as_mut()
        .ok_or_else(|| anyhow!("browser is not ready"))
}

async fn send(events: &mpsc::Sender<BackendEvent>, event: BackendEvent) -> Result<()> {
    events
        .send(event)
        .await
        .map_err(|_| anyhow!("application event receiver was closed"))
}
