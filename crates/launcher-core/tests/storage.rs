use launcher_core::{
    core::LauncherError as Error,
    profiles::{ProfileId, SettingsPolicy},
    runtime::RuntimeSelection,
    storage::{AppPaths, IndexStatus, VerificationRecord, WorkspaceService},
};
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;
use uuid::Uuid;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn paths(base: &Path) -> AppPaths {
    AppPaths {
        data: base.join("данные игры"),
        configuration: base.join("config"),
        cache: base.join("cache"),
        logs: base.join("logs"),
    }
}
fn open(base: &Path) -> launcher_core::core::Result<WorkspaceService> {
    WorkspaceService::open(paths(base), SettingsPolicy::new(8192)?)
}
fn json(path: &Path) -> std::result::Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

#[test]
fn profiles_and_settings_survive_restart_with_stable_ids() -> Result {
    let temp = TempDir::new()?;
    let first;
    let second;
    {
        let service = open(temp.path())?;
        let initial = service.snapshot()?;
        first = initial.active_profile_id;
        assert_eq!(initial.profiles.len(), 1);
        let created = service.create_profile(initial.root_id, "  Второй мир  ")?;
        second = created
            .profiles
            .iter()
            .find(|p| p.id != first)
            .ok_or("second profile missing")?
            .id;
        let mut settings = initial.profiles[0].settings.clone();
        settings.min_ram_mib = 512;
        settings.max_ram_mib = 2048;
        settings.extra_jvm_args = vec!["-XX:+UseG1GC".into(), "-XX:MaxGCPauseMillis=100".into()];
        let saved = service.save_profile(initial.root_id, first, 1, "Мой мир", settings)?;
        assert_eq!(
            saved
                .profiles
                .iter()
                .find(|p| p.id == first)
                .ok_or("first missing")?
                .revision,
            2
        );
        service.select_profile(initial.root_id, second, saved.revision)?;
    }
    let service = open(temp.path())?;
    let snapshot = service.snapshot()?;
    assert_eq!(snapshot.active_profile_id, second);
    let profile = snapshot
        .profiles
        .iter()
        .find(|p| p.id == first)
        .ok_or("first missing")?;
    assert_eq!(profile.name, "Мой мир");
    assert_eq!(profile.settings.max_ram_mib, 2048);
    assert_eq!(profile.settings.extra_jvm_args.len(), 2);
    assert_eq!(snapshot.profiles.len(), 2);
    Ok(())
}

#[test]
fn profile_names_are_labels_and_worlds_are_isolated() -> Result {
    let temp = TempDir::new()?;
    let service = open(temp.path())?;
    let a = service.snapshot()?;
    let b = service.create_profile(a.root_id, "../../outside")?;
    let second = b
        .profiles
        .iter()
        .find(|p| p.id != a.active_profile_id)
        .ok_or("second missing")?
        .id;
    let first_dir = service.profile_game_dir(a.active_profile_id)?;
    let second_dir = service.profile_game_dir(second)?;
    fs::create_dir(first_dir.join("saves"))?;
    fs::write(first_dir.join("saves/world.dat"), b"world data")?;
    assert_ne!(first_dir, second_dir);
    assert!(!second_dir.join("saves").exists());
    assert!(!temp.path().join("outside").exists());
    let mut settings = a.profiles[0].settings.clone();
    settings.max_ram_mib = 3072;
    service.save_profile(a.root_id, a.active_profile_id, 1, "Renamed", settings)?;
    assert_eq!(fs::read(first_dir.join("saves/world.dat"))?, b"world data");
    Ok(())
}

#[test]
fn root_switch_preserves_old_data_and_restores_per_root_selection() -> Result {
    let temp = TempDir::new()?;
    let mut service = open(temp.path())?;
    let a = service.snapshot()?;
    fs::write(
        service
            .profile_game_dir(a.active_profile_id)?
            .join("options.txt"),
        "user settings",
    )?;
    let b = service.switch_root(
        a.root_id,
        a.configuration_revision,
        &temp.path().join("other root"),
    )?;
    assert_ne!(a.root_id, b.root_id);
    assert_ne!(a.active_profile_id, b.active_profile_id);
    assert!(matches!(
        service.create_profile(a.root_id, "stale"),
        Err(Error::RevisionConflict)
    ));
    let restored = service.switch_root(b.root_id, b.configuration_revision, &a.data_root)?;
    assert_eq!(restored.root_id, a.root_id);
    assert_eq!(restored.active_profile_id, a.active_profile_id);
    assert_eq!(
        fs::read_to_string(
            service
                .profile_game_dir(a.active_profile_id)?
                .join("options.txt")
        )?,
        "user settings"
    );
    drop(service);
    assert_eq!(open(temp.path())?.snapshot()?.root_id, a.root_id);
    Ok(())
}

#[test]
fn switching_rejects_nonempty_unmanaged_nested_and_config_roots() -> Result {
    let temp = TempDir::new()?;
    let mut service = open(temp.path())?;
    let a = service.snapshot()?;
    let foreign = temp.path().join(".minecraft");
    fs::create_dir(&foreign)?;
    fs::write(foreign.join("world.dat"), "untouched")?;
    for path in [
        &foreign,
        &a.data_root.join("nested"),
        temp.path(),
        &paths(temp.path()).configuration,
    ] {
        assert!(
            service
                .switch_root(a.root_id, a.configuration_revision, path)
                .is_err()
        );
    }
    assert!(!foreign.join("root.json").exists());
    assert!(!foreign.join(".launcher-root.lock").exists());
    assert_eq!(fs::read_to_string(foreign.join("world.dat"))?, "untouched");
    assert_eq!(service.snapshot()?.root_id, a.root_id);
    Ok(())
}

#[test]
fn leases_prevent_writes_and_switches_and_survive_store_drop() -> Result {
    let temp = TempDir::new()?;
    let mut service = open(temp.path())?;
    let a = service.snapshot()?;
    let lease = service.lease_profile(a.active_profile_id)?;
    assert!(matches!(
        service.save_profile(
            a.root_id,
            a.active_profile_id,
            1,
            "Busy",
            a.profiles[0].settings.clone()
        ),
        Err(Error::ProfileBusy)
    ));
    assert!(matches!(
        service.switch_root(
            a.root_id,
            a.configuration_revision,
            &temp.path().join("other")
        ),
        Err(Error::ProfileBusy)
    ));
    drop(service);
    assert!(matches!(open(temp.path()), Err(Error::StorageBusy)));
    drop(lease);
    let reopened = open(temp.path());
    assert!(
        reopened.is_ok(),
        "reopen after dropping lease: {:?}",
        reopened.err()
    );
    Ok(())
}

#[test]
fn revisions_prevent_lost_updates_and_invalid_settings_preserve_bytes() -> Result {
    let temp = TempDir::new()?;
    let service = open(temp.path())?;
    let a = service.snapshot()?;
    let path = a
        .data_root
        .join("profiles")
        .join(a.active_profile_id.0.to_string())
        .join("config.json");
    service.save_profile(
        a.root_id,
        a.active_profile_id,
        1,
        "Updated",
        a.profiles[0].settings.clone(),
    )?;
    let before = fs::read(&path)?;
    assert!(matches!(
        service.save_profile(
            a.root_id,
            a.active_profile_id,
            1,
            "Stale",
            a.profiles[0].settings.clone()
        ),
        Err(Error::RevisionConflict)
    ));
    for argument in [
        "-Xmx99G",
        "-javaagent:evil.jar",
        "@args.txt",
        "-cp",
        "-Djava.library.path=/tmp",
        "-XX:OnError=bad",
        "-XX:MaxGCPauseMillis=0",
        "\n",
    ] {
        let mut settings = a.profiles[0].settings.clone();
        settings.extra_jvm_args = vec![argument.into()];
        assert!(
            service
                .save_profile(a.root_id, a.active_profile_id, 2, "Updated", settings)
                .is_err()
        );
    }
    for (min, max) in [(0, 1024), (2048, 1024), (1024, 8193), (1024, u32::MAX)] {
        let mut settings = a.profiles[0].settings.clone();
        settings.min_ram_mib = min;
        settings.max_ram_mib = max;
        assert!(
            service
                .save_profile(a.root_id, a.active_profile_id, 2, "Updated", settings)
                .is_err()
        );
    }
    assert_eq!(fs::read(&path)?, before);
    Ok(())
}

#[test]
fn custom_java_is_stored_only_after_file_validation_without_running_it() -> Result {
    let temp = TempDir::new()?;
    let service = open(temp.path())?;
    let a = service.snapshot()?;
    let java = temp
        .path()
        .join(if cfg!(windows) { "java.exe" } else { "java" });
    fs::write(&java, b"not executed")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&java, fs::Permissions::from_mode(0o700))?;
    }
    let mut settings = a.profiles[0].settings.clone();
    settings.runtime = RuntimeSelection::Custom {
        executable: java.clone(),
    };
    service.save_profile(a.root_id, a.active_profile_id, 1, "Custom", settings)?;
    fs::remove_file(&java)?;
    drop(service);
    // A missing previously selected JVM must not prevent opening settings to fix it.
    assert!(matches!(
        open(temp.path())?.snapshot()?.profiles[0].settings.runtime,
        RuntimeSelection::Custom { .. }
    ));
    Ok(())
}

#[test]
fn corrupt_and_future_configuration_are_never_reset() -> Result {
    for document in ["{broken", "{\"schemaVersion\":99,\"futureField\":true}"] {
        let temp = TempDir::new()?;
        fs::create_dir_all(paths(temp.path()).configuration.clone())?;
        let path = paths(temp.path()).configuration.join("launcher.json");
        fs::write(&path, document)?;
        assert!(open(temp.path()).is_err());
        assert_eq!(fs::read_to_string(&path)?, document);
        assert!(!paths(temp.path()).data.exists());
    }
    let temp = TempDir::new()?;
    let service = open(temp.path())?;
    let a = service.snapshot()?;
    let path = a
        .data_root
        .join("profiles")
        .join(a.active_profile_id.0.to_string())
        .join("config.json");
    fs::write(&path, "{\"schemaVersion\":2}")?;
    assert!(matches!(service.snapshot(), Err(Error::UnsupportedSchema)));
    assert!(
        service
            .save_profile(
                a.root_id,
                a.active_profile_id,
                1,
                "No reset",
                a.profiles[0].settings.clone()
            )
            .is_err()
    );
    assert_eq!(fs::read_to_string(&path)?, "{\"schemaVersion\":2}");
    Ok(())
}

#[test]
fn partial_writes_and_profile_staging_are_ignored_after_restart() -> Result {
    let temp = TempDir::new()?;
    let service = open(temp.path())?;
    let a = service.snapshot()?;
    let parent = a.data_root.join("profiles");
    fs::create_dir(parent.join(format!(".creating-{}", Uuid::new_v4())))?;
    fs::write(
        paths(temp.path())
            .configuration
            .join(".write-interrupted.tmp"),
        "{partial",
    )?;
    let config = parent
        .join(a.active_profile_id.0.to_string())
        .join("config.json");
    let before = fs::read(&config)?;
    fs::write(config.with_file_name(".write-interrupted.tmp"), "partial")?;
    drop(service);
    let b = open(temp.path())?.snapshot()?;
    assert_eq!(b.active_profile_id, a.active_profile_id);
    assert_eq!(b.profiles.len(), 1);
    assert_eq!(fs::read(&config)?, before);
    Ok(())
}

#[test]
fn corrupt_index_is_quarantined_future_index_is_preserved_and_worlds_survive() -> Result {
    let temp = TempDir::new()?;
    let service = open(temp.path())?;
    let a = service.snapshot()?;
    let world = service
        .profile_game_dir(a.active_profile_id)?
        .join("world.dat");
    fs::write(&world, "world")?;
    let mut index = service.load_verification_index()?;
    assert_eq!(index.status, IndexStatus::Created);
    index.index.records.insert(
        "shared/libraries/example.jar".into(),
        VerificationRecord {
            expected_sha1: "a".repeat(40),
            expected_size: 42,
            observed_size: 42,
            modified_unix_ns: 1234567890123456789,
            verified_unix_ms: 1,
            file_identity: None,
        },
    );
    service.save_verification_index(&index.index)?;
    assert_eq!(service.load_verification_index()?.index.records.len(), 1);
    index.index.records.insert(
        "../escape".into(),
        index
            .index
            .records
            .values()
            .next()
            .ok_or("record missing")?
            .clone(),
    );
    assert!(service.save_verification_index(&index.index).is_err());
    let path = a.data_root.join("state/verification-index.json");
    fs::write(&path, "{broken")?;
    let recovered = service.load_verification_index()?;
    assert_eq!(recovered.status, IndexStatus::Rebuilt);
    assert!(recovered.index.records.is_empty());
    assert!(
        fs::read_dir(path.parent().ok_or("no parent")?)?
            .filter_map(|e| e.ok())
            .any(|e| e
                .file_name()
                .to_string_lossy()
                .starts_with("verification-index.invalid-"))
    );
    fs::write(&path, "{\"schemaVersion\":99}")?;
    assert!(matches!(
        service.load_verification_index(),
        Err(Error::UnsupportedSchema)
    ));
    assert!(service.save_verification_index(&recovered.index).is_err());
    assert_eq!(fs::read_to_string(&path)?, "{\"schemaVersion\":99}");
    assert_eq!(fs::read_to_string(&world)?, "world");
    Ok(())
}

#[test]
fn configuration_and_root_locks_work_across_processes_and_release_on_exit() -> Result {
    let temp = TempDir::new()?;
    let service = open(temp.path())?;
    assert!(matches!(open(temp.path()), Err(Error::StorageBusy)));
    let output = Command::new(std::env::current_exe()?)
        .args(["--exact", "child_lock_probe", "--nocapture"])
        .env("LAUNCHER_LOCK_PROBE", temp.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let mut other_paths = paths(temp.path());
    other_paths.configuration = temp.path().join("another config");
    assert!(matches!(
        WorkspaceService::open(other_paths, SettingsPolicy::new(8192)?),
        Err(Error::StorageBusy)
    ));
    drop(service);
    let output = Command::new(std::env::current_exe()?)
        .args(["--exact", "child_lock_probe", "--nocapture"])
        .env("LAUNCHER_LOCK_PROBE", temp.path())
        .env("LAUNCHER_LOCK_EXPECT_FREE", "1")
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(open(temp.path()).is_ok());
    Ok(())
}
#[test]
fn child_lock_probe() -> Result {
    let Some(base) = std::env::var_os("LAUNCHER_LOCK_PROBE") else {
        return Ok(());
    };
    if std::env::var_os("LAUNCHER_LOCK_EXPECT_FREE").is_some() {
        let _service = open(Path::new(&base))?;
    } else {
        assert!(matches!(open(Path::new(&base)), Err(Error::StorageBusy)));
    }
    Ok(())
}

#[cfg(unix)]
fn symlink_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(from, to)
}
#[cfg(windows)]
fn symlink_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(from, to)
}
#[cfg(unix)]
fn symlink_file(from: &Path, to: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(from, to)
}
#[cfg(windows)]
fn symlink_file(from: &Path, to: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(from, to)
}

#[test]
fn managed_symlinks_cannot_redirect_reads_writes_or_game_paths() -> Result {
    let temp = TempDir::new()?;
    let service = open(temp.path())?;
    let a = service.snapshot()?;
    let external = temp.path().join("external");
    fs::create_dir(&external)?;
    let game = service.profile_game_dir(a.active_profile_id)?;
    fs::remove_dir(&game)?;
    symlink_dir(&external, &game)?;
    assert!(service.profile_game_dir(a.active_profile_id).is_err());
    assert!(
        service
            .save_profile(
                a.root_id,
                a.active_profile_id,
                1,
                "No",
                a.profiles[0].settings.clone()
            )
            .is_err()
    );
    assert_eq!(fs::read_dir(&external)?.count(), 0);
    #[cfg(unix)]
    fs::remove_file(&game)?;
    #[cfg(windows)]
    fs::remove_dir(&game)?;
    fs::create_dir(&game)?;
    let config = game.with_file_name("config.json");
    let before = fs::read(&config)?;
    let outside_file = external.join("config.json");
    fs::write(&outside_file, &before)?;
    fs::remove_file(&config)?;
    symlink_file(&outside_file, &config)?;
    assert!(service.snapshot().is_err());
    assert!(
        service
            .save_profile(
                a.root_id,
                a.active_profile_id,
                1,
                "No",
                a.profiles[0].settings.clone()
            )
            .is_err()
    );
    assert_eq!(fs::read(&outside_file)?, before);
    let mut service = service;
    let linked_root = temp.path().join("root-link");
    symlink_dir(&external, &linked_root)?;
    assert!(
        service
            .switch_root(a.root_id, a.configuration_revision, &linked_root)
            .is_err()
    );
    Ok(())
}

#[test]
fn arbitrary_and_mismatched_profile_ids_are_rejected() -> Result {
    let temp = TempDir::new()?;
    let service = open(temp.path())?;
    let a = service.snapshot()?;
    assert!(service.profile_game_dir(ProfileId(Uuid::nil())).is_err());
    assert!(service.profile_game_dir(ProfileId(Uuid::new_v4())).is_err());
    let path = a
        .data_root
        .join("profiles")
        .join(a.active_profile_id.0.to_string())
        .join("config.json");
    let mut document = json(&path)?;
    document["id"] = Uuid::new_v4().to_string().into();
    fs::write(&path, serde_json::to_vec(&document)?)?;
    assert!(matches!(service.snapshot(), Err(Error::CorruptDocument)));
    Ok(())
}

#[test]
fn future_root_marker_and_selection_are_preserved() -> Result {
    for target in ["root.json", "state/root-settings.json"] {
        let temp = TempDir::new()?;
        let service = open(temp.path())?;
        let a = service.snapshot()?;
        let path = a.data_root.join(target);
        drop(service);
        let before = "{\"schemaVersion\":42,\"future\":true}";
        fs::write(&path, before)?;
        assert!(matches!(open(temp.path()), Err(Error::UnsupportedSchema)));
        assert_eq!(fs::read_to_string(&path)?, before);
    }
    Ok(())
}

#[test]
fn killed_process_releases_os_locks_without_deleting_lock_files() -> Result {
    let temp = TempDir::new()?;
    let mut child = Command::new(std::env::current_exe()?)
        .args(["--exact", "child_crash_probe", "--nocapture"])
        .env("LAUNCHER_CRASH_PROBE", temp.path())
        .stdout(std::process::Stdio::null())
        .spawn()?;
    let ready = temp.path().join("ready");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !ready.exists() && std::time::Instant::now() < deadline {
        if child.try_wait()?.is_some() {
            return Err("lock holder exited early".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let acquired = ready.exists();
    let busy = matches!(open(temp.path()), Err(Error::StorageBusy));
    child.kill()?;
    child.wait()?;
    assert!(acquired, "lock holder did not become ready");
    assert!(busy);
    assert!(
        paths(temp.path())
            .configuration
            .join(".launcher.lock")
            .exists()
    );
    assert!(open(temp.path()).is_ok());
    Ok(())
}
#[test]
fn child_crash_probe() -> Result {
    let Some(base) = std::env::var_os("LAUNCHER_CRASH_PROBE") else {
        return Ok(());
    };
    let service = open(Path::new(&base))?;
    let lease = service.lease_profile(service.snapshot()?.active_profile_id)?;
    fs::write(Path::new(&base).join("ready"), "ready")?;
    // Parent terminates this process; no destructor/unlock runs.
    std::thread::sleep(std::time::Duration::from_secs(30));
    drop(lease);
    drop(service);
    Err("parent failed to terminate the lock holder".into())
}

#[test]
fn switching_to_corrupt_profile_does_not_commit_the_new_root() -> Result {
    let temp = TempDir::new()?;
    let mut current = open(temp.path())?;
    let a = current.snapshot()?;
    let other_base = temp.path().join("other launcher");
    let other = open(&other_base)?;
    let b = other.snapshot()?;
    let profile_path = b
        .data_root
        .join("profiles")
        .join(b.active_profile_id.0.to_string())
        .join("config.json");
    drop(other);
    let bytes = b"{\"schemaVersion\":99}";
    fs::write(&profile_path, bytes)?;
    let bootstrap_path = paths(temp.path()).configuration.join("launcher.json");
    let before = fs::read(&bootstrap_path)?;
    assert!(matches!(
        current.switch_root(a.root_id, a.configuration_revision, &b.data_root),
        Err(Error::UnsupportedSchema)
    ));
    assert_eq!(current.snapshot()?.root_id, a.root_id);
    assert_eq!(fs::read(bootstrap_path)?, before);
    assert_eq!(fs::read(profile_path)?, bytes);
    Ok(())
}
