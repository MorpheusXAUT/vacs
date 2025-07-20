use crate::state::AppState;
use anyhow::Context;
use tauri::{AppHandle, Manager, State};

mod auth;
mod config;
mod signaling;
mod state;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
async fn greet(app_state: State<'_, AppState>) -> Result<(), String> {
    auth::open_auth_url(&app_state)
        .await
        .expect("Failed to open auth url");

    Ok(())
}

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
                    auth::handle_auth_callback(&app, &url)
                        .await
                        .expect("Failed to handle auth callback");
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
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
