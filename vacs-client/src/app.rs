use crate::app::state::AppState;
use crate::auth;
use crate::config::BackendEndpoint;
use crate::error::{Error, FrontendError};
use anyhow::Context;
use rfd::{MessageButtons, MessageDialogResult};
use serde::Serialize;
use serde_json::Value;
use std::sync::mpsc::sync_channel;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};
use url::Url;

pub(crate) mod commands;

pub(crate) mod state;

pub fn handle_deep_link(app: AppHandle, url: String) {
    let url = url.to_string();
    tauri::async_runtime::spawn(async move {
        if let Err(err) = auth::handle_auth_callback(&app, &url).await {
            app.emit("auth:error", Value::Null).ok();
            app.emit::<FrontendError>("error", err.into()).ok();
        }
    });
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    current_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_version: Option<String>,
    required: bool,
}

pub async fn get_update(app: &AppHandle) -> Result<Option<Update>, Error> {
    let state = app.state::<AppState>();
    let state = state.lock().await;
    let channel = &state.config.client.release_channel;
    let updater_url = state
        .config
        .backend
        .endpoint_url(BackendEndpoint::VersionUpdateCheck)
        .replace("{{channel}}", channel.as_str());

    log::info!("Checking for update at {updater_url}...");

    Ok(app
        .updater_builder()
        .endpoints(vec![
            Url::parse(&updater_url).context("Failed to parse update url")?,
        ])
        .context("Failed to set update url")?
        .build()
        .context("Failed to build updater")?
        .check()
        .await
        .context("Failed to check for updates")?)
}

pub fn open_fatal_error_dialog(app: &AppHandle, msg: &str) {
    let open_logs = "Open logs folder";
    let result = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("vacs - Fatal error")
        .set_description(msg)
        .set_buttons(MessageButtons::OkCancelCustom(
            open_logs.to_string(),
            "Close".to_string(),
        ))
        .show_blocking();

    match result {
        MessageDialogResult::Custom(text) if text == open_logs => {
            if let Err(err) = open_logs_folder(app) {
                log::error!("Failed to open logs folder: {err}");

                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title("vacs - Fatal error")
                    .set_description("Failed to open logs folder.")
                    .show_blocking();
            }
        }
        _ => {}
    };
}

pub fn open_logs_folder(app: &AppHandle) -> Result<(), Error> {
    let log_dir = app
        .path()
        .app_log_dir()
        .context("Failed to get logs folder")?;
    let log_dir = log_dir.to_str().context("Log dir is empty")?;

    app.opener()
        .open_path(log_dir, None::<&str>)
        .context("Failed to open logs folder")?;

    Ok(())
}

trait BlockingMessageDialog {
    fn show_blocking(self) -> MessageDialogResult;
}

impl BlockingMessageDialog for rfd::MessageDialog {
    fn show_blocking(self) -> MessageDialogResult {
        let (tx, rx) = sync_channel(0);

        std::thread::spawn(move || {
            let result = self.show();
            tx.send(result).unwrap();
        });

        rx.recv().unwrap()
    }
}
