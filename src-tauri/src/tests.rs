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

#[test]
fn authentication_registry_acl_and_release_mock_gate() -> Result<(), Box<dyn std::error::Error>> {
    let app = app()?;
    let main = tauri::WebviewWindowBuilder::new(&app.app, "main", Default::default()).build()?;
    let secondary =
        tauri::WebviewWindowBuilder::new(&app.app, "untrusted", Default::default()).build()?;
    for command in ["get_auth", "login", "logout"] {
        assert!(get_ipc_response(&secondary, request(command, bundled_origin())?).is_err());
        assert!(get_ipc_response(&main, request(command, "https://example.com")?).is_err());
    }
    let state = call(&main, "get_auth", serde_json::json!({}))?;
    assert!(state["account"].is_null());
    let payload = serde_json::json!({"request":{"expectedRevision":0,"providerId":"development-mock","nickname":"Notch"}});
    #[cfg(all(debug_assertions, feature = "dev-auth"))]
    {
        let logged = call(&main, "login", payload.clone())?;
        assert_eq!(
            logged["account"]["playerId"],
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
        assert_eq!(logged["account"]["development"], true);
        assert!(!logged.to_string().contains("development-only-"));
        assert_eq!(call(&main, "get_auth", serde_json::json!({}))?, logged);
        assert!(call(&main, "login", payload).is_err());
        assert!(
            call(
                &main,
                "logout",
                serde_json::json!({"request":{"expectedRevision":1}})
            )?["account"]
                .is_null()
        );
    }
    #[cfg(not(all(debug_assertions, feature = "dev-auth")))]
    {
        assert_eq!(
            state["providers"]
                .as_array()
                .ok_or("providers missing")?
                .len(),
            1
        );
        for id in ["development-mock", "mock", "offline", "ely-by"] {
            let mut req = request("login", bundled_origin())?;
            req.body = InvokeBody::Json(
                serde_json::json!({"request":{"expectedRevision":0,"providerId":id,"nickname":"Notch"}}),
            );
            assert_eq!(
                get_ipc_response(&main, req)
                    .err()
                    .ok_or("forged provider accepted")?["code"],
                "auth_unavailable"
            );
        }
        let _ = payload;
    }
    assert!(call(&main,"login",serde_json::json!({"request":{"expectedRevision":0,"providerId":"development-mock","nickname":"Notch","session":{"accessToken":"forged-token"}}})).is_err());
    assert!(!app._temp.path().join("data").exists());
    Ok(())
}
