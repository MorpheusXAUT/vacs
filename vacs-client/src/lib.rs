mod app;
mod audio;
mod auth;
mod build;
mod config;
mod error;
mod secrets;
mod signaling;

use crate::app::open_fatal_error_dialog;
use crate::app::state::http::HttpState;
use crate::app::state::{AppState, AppStateInner};
use crate::build::VersionInfo;
use crate::error::{FrontendError, StartupError, StartupErrorExt};
use anyhow::Context;
use tauri::{App, Emitter, Manager, RunEvent};
use tokio::sync::Mutex;

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .max_file_size(1_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            log::info!("{:?}", VersionInfo::gather());

            fn setup(app: &mut App) -> Result<(), StartupError> {
                use tauri_plugin_deep_link::DeepLinkExt;
                app.deep_link()
                    .register_all()
                    .context("Failed to register deep link")
                    .map_startup_err(StartupError::Other)?;

                let state = AppStateInner::new(app.handle())?;

                if state.config.client.always_on_top {
                    let main_window = app
                        .get_webview_window("main")
                        .context("Failed to get main window")
                        .map_startup_err(StartupError::Other)?;
                    if let Err(err) = main_window.set_always_on_top(true) {
                        log::warn!("Failed to set main window to be always on top: {err}");
                    } else {
                        log::debug!("Set main window to be always on top");
                    }
                }

                app.manage(Mutex::new(state));

                let http_state = HttpState::new(app.handle())?;
                app.manage(http_state);

                Ok(())
            }

            if let Err(err) = setup(app) {
                log::error!("Startup failed. Err: {err:?}");

                open_fatal_error_dialog(app.handle(), &err.to_string());

                return Err(anyhow::anyhow!("{err}").into());
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::app_check_for_update,
            app::commands::app_frontend_ready,
            app::commands::app_open_logs_folder,
            app::commands::app_set_always_on_top,
            app::commands::app_update,
            audio::commands::audio_get_devices,
            audio::commands::audio_get_hosts,
            audio::commands::audio_get_volumes,
            audio::commands::audio_play_ui_click,
            audio::commands::audio_set_device,
            audio::commands::audio_set_host,
            audio::commands::audio_set_input_muted,
            audio::commands::audio_set_volume,
            audio::commands::audio_start_input_level_meter,
            audio::commands::audio_stop_input_level_meter,
            auth::commands::auth_check_session,
            auth::commands::auth_logout,
            auth::commands::auth_open_oauth_url,
            signaling::commands::signaling_accept_call,
            signaling::commands::signaling_connect,
            signaling::commands::signaling_disconnect,
            signaling::commands::signaling_end_call,
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
                        .state::<HttpState>()
                        .persist()
                        .expect("Failed to persist http state");

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
