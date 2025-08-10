mod app;
mod audio;
mod auth;
mod config;
mod error;
mod secrets;
mod signaling;

use crate::app::state::{AppState, AppStateInner};
use crate::error::FrontendError;
use tauri::{Emitter, Manager, RunEvent};
use tokio::sync::Mutex;

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Warn)
                .level_for("vacs_client_lib", log::LevelFilter::Trace)
                .level_for("vacs_audio", log::LevelFilter::Trace)
                .level_for("vacs_signaling", log::LevelFilter::Trace)
                .level_for("vacs_vatsim", log::LevelFilter::Trace)
                .level_for("vacs_webrtc", log::LevelFilter::Trace)
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|app, argv, _| {
            if let Some(url) = argv.get(1) {
                let app = app.clone();
                let url = url.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = auth::handle_auth_callback(&app, &url).await {
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

            app.manage(Mutex::new(AppStateInner::new()?));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::app_frontend_ready,
            audio::commands::audio_get_devices,
            auth::commands::auth_check_session,
            auth::commands::auth_logout,
            auth::commands::auth_open_oauth_url,
            signaling::commands::signaling_accept_call,
            signaling::commands::signaling_connect,
            signaling::commands::signaling_disconnect,
            signaling::commands::signaling_end_call,
            signaling::commands::signaling_reject_call,
            signaling::commands::signaling_start_call,
            signaling::commands::signaling_terminate,
        ])
        .build(tauri::generate_context!())
        .expect("Failed to build tauri application")
        .run(move |app_handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
                let app_handle = app_handle.clone();
                tauri::async_runtime::block_on(async move {
                    app_handle
                        .state::<AppState>()
                        .lock()
                        .await
                        .persist()
                        .expect("Failed to persist app state");
                });
            }
        });
}
