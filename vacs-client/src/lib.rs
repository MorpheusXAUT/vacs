mod app;
mod audio;
mod auth;
mod build;
mod config;
mod error;
mod keybinds;
mod platform;
mod radio;
mod secrets;
mod signaling;

use crate::app::open_fatal_error_dialog;
use crate::app::state::audio::AppStateAudioExt;
use crate::app::state::http::HttpState;
use crate::app::state::keybinds::AppStateKeybindsExt;
use crate::app::state::{AppState, AppStateInner};
use crate::audio::manager::AudioManagerHandle;
use crate::build::VersionInfo;
use crate::error::{StartupError, StartupErrorExt};
use crate::keybinds::engine::KeybindEngineHandle;
use crate::platform::Capabilities;
use anyhow::Context;
use tauri::{App, Manager, RunEvent};
use tauri_plugin_deep_link::DeepLinkExt;
use tokio::sync::Mutex as TokioMutex;

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
                app::handle_deep_link(app.clone(), url.to_string());
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_prevent_default::debug())
        .setup(|app| {
            log::info!("{:?}", VersionInfo::gather());

            #[cfg(target_os = "macos")]
            {
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    if let Some(url) = event.urls().first() {
                        app::handle_deep_link(handle.clone(), url.to_string());
                    }
                });
            }

            fn setup(app: &mut App) -> Result<(), StartupError> {
                #[cfg(not(target_os = "macos"))]
                {
                    app.deep_link()
                        .register_all()
                        .context("Failed to register deep link")
                        .map_startup_err(StartupError::Other)?;
                }

                let capabilities = Capabilities::default();

                let state = AppStateInner::new(app.handle())?;

                let main_window = app
                    .get_webview_window("main")
                    .context("Failed to get main window")
                    .map_startup_err(StartupError::Other)?;

                if state.config.client.always_on_top {
                    if capabilities.always_on_top {
                        if let Err(err) = main_window.set_always_on_top(true) {
                            log::warn!("Failed to set main window to be always on top: {err}");
                        } else {
                            log::debug!("Set main window to be always on top");
                        }
                    } else {
                        log::warn!("Your platform ({}) does not support always on top windows, setting is ignored.", capabilities.platform)
                    }
                }

                if state.config.client.fullscreen {
                    if let Err(err) = main_window.set_fullscreen(true) {
                        log::warn!("Failed to set main window to be fullscreen: {err}");
                    } else {
                        log::debug!("Set main window to be fullscreen");
                    }
                }

                let transmit_config = state.config.client.transmit_config.clone();
                let keybind_engine = state.keybind_engine_handle();

                app.manage::<HttpState>(HttpState::new(app.handle())?);
                app.manage::<AudioManagerHandle>(state.audio_manager_handle());
                app.manage::<AppState>(TokioMutex::new(state));

                if capabilities.keybinds {
                    keybind_engine
                        .write().set_config(&transmit_config)
                        .map_startup_err(StartupError::Keybinds)?;
                } else {
                    log::warn!("Your platform ({}) does not support keybinds, skipping registration", capabilities.platform);
                }

                app.manage::<KeybindEngineHandle>(keybind_engine);

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
            app::commands::app_platform_capabilities,
            app::commands::app_set_always_on_top,
            app::commands::app_set_fullscreen,
            app::commands::app_update,
            audio::commands::audio_get_devices,
            audio::commands::audio_get_hosts,
            audio::commands::audio_get_volumes,
            audio::commands::audio_play_ui_click,
            audio::commands::audio_set_device,
            audio::commands::audio_set_host,
            audio::commands::audio_set_radio_prio,
            audio::commands::audio_set_volume,
            audio::commands::audio_start_input_level_meter,
            audio::commands::audio_stop_input_level_meter,
            auth::commands::auth_check_session,
            auth::commands::auth_logout,
            auth::commands::auth_open_oauth_url,
            keybinds::commands::keybinds_get_radio_config,
            keybinds::commands::keybinds_get_transmit_config,
            keybinds::commands::keybinds_has_radio,
            keybinds::commands::keybinds_set_radio_config,
            keybinds::commands::keybinds_set_transmit_config,
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

                    app_handle.state::<KeybindEngineHandle>().write().shutdown();

                    app_handle.state::<AppState>().lock().await.shutdown();
                });
            }
        });
}
