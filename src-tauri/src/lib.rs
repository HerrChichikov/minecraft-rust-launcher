//! Tauri adapters only. Domain services live in launcher-core.

mod commands;
mod credentials;

use launcher_core::application::ApplicationService;
use tauri::Runtime;

fn configure<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    configure_with_storage(builder, commands::workspace::WorkspaceState::default())
}

fn configure_with_storage<R: Runtime>(
    builder: tauri::Builder<R>,
    storage: commands::workspace::WorkspaceState,
) -> tauri::Builder<R> {
    builder
        .manage(ApplicationService)
        .manage(storage)
        .manage(commands::installation::InstallationState::default())
        .manage(commands::auth::AuthState::default())
        .manage(commands::game::GameState::default())
        .invoke_handler(tauri::generate_handler![
            commands::snapshot::get_snapshot,
            commands::workspace::get_workspace,
            commands::workspace::create_profile,
            commands::workspace::select_profile,
            commands::workspace::save_profile,
            commands::workspace::switch_data_root,
            commands::installation::start_install,
            commands::installation::get_installation,
            commands::installation::cancel_install,
            commands::auth::get_auth,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::continue_login,
            commands::auth::cancel_login,
            commands::auth::refresh_auth,
            commands::auth::restore_auth,
            commands::game::get_game,
            commands::game::start_game,
            commands::game::stop_game
        ])
}

pub fn run() -> tauri::Result<()> {
    use tauri::Manager;
    configure(tauri::Builder::default())
        .build(context())?
        .run(|app, event| {
            let game = app.state::<commands::game::GameState>();
            match event {
                tauri::RunEvent::ExitRequested { api, .. } => {
                    if !game.can_exit() {
                        api.prevent_exit();
                    }
                }
                tauri::RunEvent::WindowEvent {
                    label,
                    event: tauri::WindowEvent::CloseRequested { api, .. },
                    ..
                } if label == "main" && game.active() => api.prevent_close(),
                _ => {}
            }
        });
    Ok(())
}

fn context<R: Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

#[cfg(test)]
mod tests;
