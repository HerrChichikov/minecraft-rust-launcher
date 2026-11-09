use super::*;
use crate::{
    profiles::SettingsPolicy,
    storage::{AppPaths, WorkspaceService},
};
use std::{ffi::OsString, path::Path};

const FIXTURE_TOKEN: &str = "phase-five-fixture-secret";

#[test]
fn game_process_child() -> std::io::Result<()> {
    if !Path::new("launcher-process-test-marker").exists() {
        return Ok(());
    }
    println!("stdout fixture {FIXTURE_TOKEN}");
    eprintln!("stderr fixture");
    if Path::new("wait-for-stop").exists() {
        std::thread::sleep(Duration::from_secs(15));
    }
    std::process::exit(23);
}

#[test]
fn reaps_owned_child_and_persists_only_bounded_filtered_output() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().canonicalize()?;
    let workspace = WorkspaceService::open(
        AppPaths {
            data: root.join("data"),
            configuration: root.join("config"),
            cache: root.join("cache"),
            logs: root.join("logs"),
        },
        SettingsPolicy::new(4096)?,
    )?;
    let snapshot = workspace.snapshot()?;
    let profile = snapshot.active_profile_id;
    let cwd = workspace.profile_game_dir(profile)?;
    std::fs::write(cwd.join("launcher-process-test-marker"), b"")?;
    let executable = crate::runtime::execution_path(&std::env::current_exe()?)?;
    let spawn = || {
        GameProcess::spawn(
            LaunchSpec {
                executable: executable.clone(),
                arguments: [
                    "--exact",
                    "launcher::process::tests::game_process_child",
                    "--nocapture",
                ]
                .into_iter()
                .map(OsString::from)
                .collect(),
                working_directory: cwd.clone(),
            },
            SecretString::new(FIXTURE_TOKEN.into()),
            workspace.installation_store(snapshot.root_id, profile)?,
            profile,
            None,
        )
    };
    let process = spawn()?;
    assert!(workspace.lease_profile(profile).is_err());
    assert!(matches!(
        verify_no_pending_run(&process.store, profile),
        Err(Error::GameRecoveryRequired)
    ));
    let completed = process.wait(&Cancellation::default())?;
    assert_eq!(completed.exit.code, Some(23));
    assert!(!completed.exit.success && !completed.exit.stopped && completed.log_saved);
    assert!(!completed.recovery_required);
    let log = std::fs::read_to_string(snapshot.data_root.join(format!(
        "profiles/{}/logs/launcher-last-run.json",
        profile.0
    )))?;
    assert!(log.contains("stderr fixture"));
    assert!(log.contains("строка с токеном скрыта"));
    assert!(!log.contains(FIXTURE_TOKEN));

    std::fs::write(cwd.join("wait-for-stop"), b"")?;
    let process = spawn()?;
    let cancel = Cancellation::default();
    cancel.cancel();
    let completed = process.wait(&cancel)?;
    assert!(completed.exit.stopped && !completed.recovery_required);
    workspace.lease_profile(profile)?;
    Ok(())
}
