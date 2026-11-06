use tauri::{
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder},
    webview::InvokeRequest,
};

fn request(command: &str, origin: &str) -> Result<InvokeRequest, Box<dyn std::error::Error>> {
    Ok(InvokeRequest {
        cmd: command.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: origin.parse()?,
        body: InvokeBody::default(),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.into(),
    })
}

struct TestApp {
    app: tauri::App<MockRuntime>,
    _temp: tempfile::TempDir,
}
impl std::ops::Deref for TestApp {
    type Target = tauri::App<MockRuntime>;
    fn deref(&self) -> &Self::Target {
        &self.app
    }
}
fn app() -> Result<TestApp, Box<dyn std::error::Error>> {
    let temp = tempfile::TempDir::new()?;
    let paths = launcher_core::storage::AppPaths {
        data: temp.path().join("data"),
        configuration: temp.path().join("config"),
        cache: temp.path().join("cache"),
        logs: temp.path().join("logs"),
    };
    let app = super::configure_with_storage(
        mock_builder(),
        super::commands::workspace::WorkspaceState::isolated(paths),
    )
    .build(super::context())?;
    Ok(TestApp { app, _temp: temp })
}

fn bundled_origin() -> &'static str {
    if cfg!(windows) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    }
}

#[test]
fn main_window_can_read_the_real_application_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let app = app()?;
    let main = tauri::WebviewWindowBuilder::new(&app.app, "main", Default::default()).build()?;
    let response = get_ipc_response(&main, request("get_snapshot", bundled_origin())?)
        .map_err(|value| value.to_string())?;
    let snapshot: serde_json::Value = response.deserialize()?;
    assert_eq!(snapshot["phase"], "foundation");
    assert_eq!(snapshot["features"]["gameLaunch"], false);
    assert_eq!(snapshot["developmentBuild"], cfg!(debug_assertions));
    Ok(())
}

#[test]
fn other_windows_and_remote_origins_cannot_read_application_state()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app()?;
    let secondary =
        tauri::WebviewWindowBuilder::new(&app.app, "untrusted", Default::default()).build()?;
    assert!(get_ipc_response(&secondary, request("get_snapshot", bundled_origin())?).is_err());

    let main = tauri::WebviewWindowBuilder::new(&app.app, "main", Default::default()).build()?;
    assert!(get_ipc_response(&main, request("get_snapshot", "https://example.com")?).is_err());
    Ok(())
}

#[test]
fn auth_launch_and_filesystem_commands_are_unavailable() -> Result<(), Box<dyn std::error::Error>> {
    let app = app()?;
    let main = tauri::WebviewWindowBuilder::new(&app.app, "main", Default::default()).build()?;
    for command in [
        "login",
        "mock_login",
        "play",
        "plugin:fs|read_text_file",
        "plugin:shell|execute",
    ] {
        assert!(get_ipc_response(&main, request(command, bundled_origin())?).is_err());
    }
    Ok(())
}

fn call(
    main: &tauri::WebviewWindow<MockRuntime>,
    command: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut req = request(command, bundled_origin())?;
    req.body = InvokeBody::Json(payload);
    Ok(get_ipc_response(main, req)
        .map_err(|value| value.to_string())?
        .deserialize()?)
}

#[test]
fn storage_commands_create_select_save_and_persist_through_real_ipc()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app()?;
    let main = tauri::WebviewWindowBuilder::new(&app.app, "main", Default::default()).build()?;
    let initial = call(&main, "get_workspace", serde_json::json!({}))?;
    assert_eq!(
        initial["profiles"]
            .as_array()
            .ok_or("profiles missing")?
            .len(),
        1
    );
    let created = call(
        &main,
        "create_profile",
        serde_json::json!({"request":{"rootId":initial["rootId"],"name":"IPC profile"}}),
    )?;
    let second = created["profiles"]
        .as_array()
        .ok_or("profiles missing")?
        .iter()
        .find(|p| p["id"] != initial["activeProfileId"])
        .ok_or("second profile missing")?;
    let selected = call(
        &main,
        "select_profile",
        serde_json::json!({"request":{"rootId":initial["rootId"],"profileId":second["id"],"expectedRevision":created["revision"]}}),
    )?;
    assert_eq!(selected["activeProfileId"], second["id"]);
    let mut settings = second["settings"].clone();
    settings["maxRamMib"] = 2048.into();
    let saved = call(
        &main,
        "save_profile",
        serde_json::json!({"request":{"rootId":initial["rootId"],"profileId":second["id"],"expectedRevision":1,"name":"Saved over IPC","settings":settings}}),
    )?;
    let profile = saved["profiles"]
        .as_array()
        .ok_or("profiles missing")?
        .iter()
        .find(|p| p["id"] == second["id"])
        .ok_or("saved missing")?;
    assert_eq!(profile["revision"], 2);
    let file = std::path::PathBuf::from(initial["dataRoot"].as_str().ok_or("path missing")?)
        .join("profiles")
        .join(second["id"].as_str().ok_or("id missing")?)
        .join("config.json");
    let disk: serde_json::Value = serde_json::from_slice(&std::fs::read(file)?)?;
    assert_eq!(disk["name"], "Saved over IPC");
    let mut invalid = request("save_profile", bundled_origin())?;
    invalid.body = InvokeBody::Json(
        serde_json::json!({"request":{"rootId":initial["rootId"],"profileId":second["id"],"expectedRevision":1,"name":"Stale","settings":settings}}),
    );
    assert_eq!(
        get_ipc_response(&main, invalid)
            .err()
            .ok_or("stale save accepted")?["code"],
        "revision_conflict"
    );
    Ok(())
}

#[test]
fn storage_acl_rejects_untrusted_windows_and_origins_before_creating_files()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app()?;
    let secondary =
        tauri::WebviewWindowBuilder::new(&app.app, "untrusted", Default::default()).build()?;
    let main = tauri::WebviewWindowBuilder::new(&app.app, "main", Default::default()).build()?;
    for command in [
        "get_workspace",
        "create_profile",
        "select_profile",
        "save_profile",
        "switch_data_root",
    ] {
        assert!(get_ipc_response(&secondary, request(command, bundled_origin())?).is_err());
        assert!(get_ipc_response(&main, request(command, "https://example.com")?).is_err());
    }
    assert!(!app._temp.path().join("config").exists());
    assert!(!app._temp.path().join("data").exists());
    Ok(())
}

#[test]
fn installer_state_and_cancel_are_scoped_and_unknown_operations_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app()?;
    let main = tauri::WebviewWindowBuilder::new(&app.app, "main", Default::default()).build()?;
    assert!(call(&main, "get_installation", serde_json::json!({}))?.is_null());
    let mut cancel = request("cancel_install", bundled_origin())?;
    cancel.body = InvokeBody::Json(serde_json::json!({"request":{"operationId":"unknown"}}));
    assert!(get_ipc_response(&main, cancel).is_err());
    let secondary =
        tauri::WebviewWindowBuilder::new(&app.app, "untrusted", Default::default()).build()?;
    for command in ["start_install", "get_installation", "cancel_install"] {
        assert!(get_ipc_response(&secondary, request(command, bundled_origin())?).is_err());
        assert!(get_ipc_response(&main, request(command, "https://example.com")?).is_err());
    }
    assert!(!app._temp.path().join("data").exists());
    Ok(())
}
