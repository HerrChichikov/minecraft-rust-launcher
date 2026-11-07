//! Explicit upstream smoke: downloads a pinned agent, but does not execute it or Minecraft.
use launcher_core::{
    auth::agent,
    core::LauncherError,
    installer::Cancellation,
    profiles::SettingsPolicy,
    storage::{AppPaths, WorkspaceService},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = std::path::PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: agent_smoke ABSOLUTE_ISOLATED_TEST_ROOT")?,
    );
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
    let store = service.installation_store(snapshot.root_id, snapshot.active_profile_id)?;
    let cancel = Cancellation::default();
    let prepared = agent::ensure(store.clone(), &cancel)?;
    let args = prepared.jvm_arguments(&cancel)?;
    assert_eq!(args[0], "-Dauthlibinjector.noLogFile");
    let text = args[1].to_str().ok_or("invalid argument")?;
    let path = text
        .strip_prefix("-javaagent:")
        .and_then(|s| s.rsplit_once('=').map(|(p, _)| p))
        .ok_or("invalid agent argument")?;
    let mut bytes = std::fs::read(path)?;
    let size = bytes.len();
    bytes[0] ^= 1;
    std::fs::write(path, &bytes)?;
    assert!(matches!(
        prepared.jvm_arguments(&cancel),
        Err(LauncherError::Integrity)
    ));
    let repaired = agent::ensure(store.clone(), &cancel)?;
    repaired.jvm_arguments(&cancel)?;
    let reused = agent::ensure(store, &cancel)?;
    reused.jvm_arguments(&cancel)?;
    println!(
        "{}",
        serde_json::json!({"build":agent::BUILD,"version":agent::VERSION,"sha256":agent::SHA256,"sourceCommit":agent::SOURCE_COMMIT,"bytes":size,"downloadVerification":"pass","sameSizeCorruptionRejected":true,"repair":"pass","reuse":"pass","execution":"not attempted"})
    );
    Ok(())
}
