use crate::state::AppState;
use keyring::Entry;
use keyring::Error::NoEntry;
use tauri::{AppHandle, Emitter, Manager};

mod auth;
mod config;
mod secrets;
mod signaling;
mod state;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
async fn greet(app: AppHandle) -> Result<(), String> {
    match secrets::get("cid") {
        Ok(Some(cid)) => {
            log::debug!("Found CID in secrets: {cid}");
            app.emit("vatsim-cid", cid).unwrap()
        }
        Ok(None) => {
            log::debug!("No CID found in secrets, starting auth flow");
            auth::open_auth_url(&app.state::<AppState>())
                .await
                .expect("Failed to open auth url");
        }
        Err(err) => return Err(err.to_string()),
    }

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
