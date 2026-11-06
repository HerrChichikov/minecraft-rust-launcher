//! Explicit manual installer smoke; never executed by ordinary unit tests.
use launcher_core::{
    core::{EventSink, OperationEvent, Platform},
    installer::{Cancellation, InstallEngine, metadata::RuleContext},
    profiles::SettingsPolicy,
    storage::{AppPaths, WorkspaceService},
};
use std::sync::Mutex;
#[derive(Debug)]
struct Progress(Mutex<std::time::Instant>);
impl EventSink for Progress {
    fn publish(&self, event: OperationEvent) {
        if let Ok(mut last) = self.0.lock()
            && (last.elapsed().as_secs() >= 2
                || matches!(
                    event.stage,
                    launcher_core::core::OperationStage::Ready
                        | launcher_core::core::OperationStage::Error
                ))
        {
            println!(
                "{:?}: {}/{}",
                event.stage,
                event.completed_bytes,
                event.total_bytes.unwrap_or(0)
            );
            *last = std::time::Instant::now();
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::args_os()
        .nth(1)
        .ok_or("usage: install_smoke ABSOLUTE_TEST_ROOT")?;
    let base = std::path::PathBuf::from(base);
    let service = WorkspaceService::open(
        AppPaths {
            data: base.join("data"),
            configuration: base.join("config"),
            cache: base.join("cache"),
            logs: base.join("logs"),
        },
        SettingsPolicy::new(8192)?,
    )?;
    let snapshot = service.snapshot()?;
    let profile = snapshot
        .profiles
        .iter()
        .find(|p| p.id == snapshot.active_profile_id)
        .ok_or("profile missing")?;
    let store = service.installation_store(snapshot.root_id, profile.id)?;
    let engine = InstallEngine::new(store, profile.id, Cancellation::default())?;
    let platform = Platform::current_host()?;
    let plan = engine.plan(profile, &RuleContext::new(platform, String::new()))?;
    let result = engine.prepare(
        &plan,
        platform,
        &Progress(Mutex::new(std::time::Instant::now())),
        &profile.settings.runtime,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
