mod auth;
mod config;
mod error;
mod secrets;
mod signaling;
mod app;

use crate::app::state::AppState;
use tauri::{Emitter, Manager, RunEvent};
use crate::error::FrontendError;

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .level_for("vacs_client_lib", log::LevelFilter::Trace)
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|app, argv, _| {
            if let Some(url) = argv.get(1) {
                let app = app.clone();
                let url = url.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = auth::handle_auth_callback(&app, &url)
                        .await {
                        app.emit::<FrontendError>("error", err.into()).ok();
                    }
                });
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            use tauri_plugin_deep_link::DeepLinkExt;
            app.deep_link().register_all()?;

            app.manage(AppState::new()?);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::app_frontend_ready,
            auth::commands::auth_open_oauth_url,
            auth::commands::auth_check_session,
            auth::commands::auth_logout,
        ])
        .build(tauri::generate_context!())
        .expect("Failed to build tauri application")
        .run(move |app_handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
                app_handle
                    .state::<AppState>()
                    .persist()
                    .expect("Failed to persist app state");
            }
        });
}
