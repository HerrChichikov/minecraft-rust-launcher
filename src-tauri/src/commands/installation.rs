use super::{snapshot::ApiError, workspace::execute};
use launcher_core::{
    core::{EventSink, LauncherError, OperationEvent, OperationStage, Platform, Result},
    installer::{Cancellation, InstallEngine, metadata::RuleContext},
    profiles::ProfileId,
    storage::RootId,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, Runtime};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationStatus {
    schema_version: u32,
    operation_id: String,
    root_id: RootId,
    profile_id: ProfileId,
    profile_name: String,
    stage: OperationStage,
    sequence: u64,
    completed_bytes: u64,
    total_bytes: Option<u64>,
    downloaded_bytes: u64,
    elapsed_ms: u64,
    error_code: Option<&'static str>,
    terminal: bool,
}
#[derive(Debug)]
struct Job {
    status: InstallationStatus,
    cancel: Cancellation,
}
#[derive(Debug, Default)]
pub(crate) struct InstallationState {
    job: Arc<Mutex<Option<Job>>>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartRequest {
    root_id: RootId,
    profile_id: ProfileId,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelRequest {
    operation_id: String,
}
struct Sink<R: Runtime> {
    app: AppHandle<R>,
    job: Arc<Mutex<Option<Job>>>,
}
impl<R: Runtime> EventSink for Sink<R> {
    fn publish(&self, event: OperationEvent) {
        if matches!(
            event.stage,
            OperationStage::Ready | OperationStage::Error | OperationStage::Cancelled
        ) {
            return;
        }
        if let Ok(mut job) = self.job.lock()
            && let Some(job) = job.as_mut()
            && !job.status.terminal
            && job.status.stage != OperationStage::Cancelling
        {
            let status = &mut job.status;
            status.sequence += 1;
            status.stage = event.stage;
            status.completed_bytes = event.completed_bytes;
            status.total_bytes = event.total_bytes;
            status.downloaded_bytes = event.downloaded_bytes;
            status.elapsed_ms = event.elapsed_ms;
            let _ = self
                .app
                .emit_to("main", "launcher://installation", status.clone());
        }
    }
}
fn error_code(error: &LauncherError) -> &'static str {
    match error {
        LauncherError::Cancelled => "cancelled",
        LauncherError::Network => "network",
        LauncherError::Integrity => "integrity",
        LauncherError::Metadata => "metadata",
        LauncherError::RuntimeIncompatible => "runtime",
        _ => "storage",
    }
}
#[tauri::command]
pub async fn start_install<R: Runtime>(
    app: AppHandle<R>,
    request: StartRequest,
) -> std::result::Result<InstallationStatus, ApiError> {
    let root_id = request.root_id;
    let (engine, profile, platform) = execute(app.clone(), move |service| {
        let snapshot = service.snapshot()?;
        if snapshot.root_id != request.root_id {
            return Err(LauncherError::RevisionConflict);
        }
        let profile = snapshot
            .profiles
            .into_iter()
            .find(|p| p.id == request.profile_id)
            .ok_or(LauncherError::InvalidInput { field: "profile" })?;
        let platform = Platform::current_host()?;
        let engine = InstallEngine::new(
            service.installation_store(request.root_id, request.profile_id)?,
            request.profile_id,
            Cancellation::default(),
        )?;
        Ok((engine, profile, platform))
    })
    .await?;
    let cancel = engine.cancellation();
    let state = app.state::<InstallationState>();
    let shared = Arc::clone(&state.job);
    let mut status = InstallationStatus {
        schema_version: 1,
        operation_id: engine.operation_id().0.to_string(),
        root_id,
        profile_id: profile.id,
        profile_name: profile.name.clone(),
        stage: OperationStage::Initializing,
        sequence: 0,
        completed_bytes: 0,
        total_bytes: None,
        downloaded_bytes: 0,
        elapsed_ms: 0,
        error_code: None,
        terminal: false,
    };
    {
        let mut job = shared
            .lock()
            .map_err(|_| ApiError::from(LauncherError::StorageBusy))?;
        if job.as_ref().is_some_and(|j| !j.status.terminal) {
            return Err(ApiError::from(LauncherError::StorageBusy));
        }
        status.sequence = job.as_ref().map_or(1, |j| j.status.sequence + 1);
        *job = Some(Job {
            status: status.clone(),
            cancel,
        });
        let _ = app.emit_to("main", "launcher://installation", status.clone());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let sink = Sink {
            app: app.clone(),
            job: Arc::clone(&shared),
        };
        let context = RuleContext::new(platform, sysinfo::System::os_version().unwrap_or_default());
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            engine
                .plan(&profile, &context)
                .and_then(|plan| engine.prepare(&plan, platform, &sink, &profile.settings.runtime))
        }));
        let result: Result<()> = match outcome {
            Ok(value) => value.map(|_| ()),
            Err(_) => Err(LauncherError::StorageBusy),
        };
        // Release filesystem leases before advertising a terminal state.
        drop(engine);
        if let Ok(mut job) = shared.lock()
            && let Some(job) = job.as_mut()
        {
            job.status.terminal = true;
            job.status.sequence += 1;
            match result {
                Ok(()) => job.status.stage = OperationStage::Ready,
                Err(error) => {
                    job.status.stage = if matches!(error, LauncherError::Cancelled) {
                        OperationStage::Cancelled
                    } else {
                        OperationStage::Error
                    };
                    job.status.error_code = Some(error_code(&error));
                }
            }
            let _ = app.emit_to("main", "launcher://installation", job.status.clone());
        }
    });
    Ok(status)
}
#[tauri::command]
pub fn get_installation<R: Runtime>(
    app: AppHandle<R>,
) -> std::result::Result<Option<InstallationStatus>, ApiError> {
    let state = app.state::<InstallationState>();
    let job = state
        .job
        .lock()
        .map_err(|_| ApiError::from(LauncherError::StorageBusy))?;
    Ok(job.as_ref().map(|j| j.status.clone()))
}
#[tauri::command]
pub fn cancel_install<R: Runtime>(
    app: AppHandle<R>,
    request: CancelRequest,
) -> std::result::Result<(), ApiError> {
    let state = app.state::<InstallationState>();
    let mut job = state
        .job
        .lock()
        .map_err(|_| ApiError::from(LauncherError::StorageBusy))?;
    let job = job
        .as_mut()
        .filter(|j| j.status.operation_id == request.operation_id)
        .ok_or_else(|| {
            ApiError::from(LauncherError::InvalidInput {
                field: "operation ID",
            })
        })?;
    if !job.status.terminal {
        job.cancel.cancel();
        job.status.stage = OperationStage::Cancelling;
        job.status.sequence += 1;
        let _ = app.emit_to("main", "launcher://installation", job.status.clone());
    }
    Ok(())
}
