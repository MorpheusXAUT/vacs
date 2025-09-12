use crate::app::state::AppState;
use crate::config::BackendEndpoint;
use crate::error::Error;
use anyhow::Context;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{
    DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};
use url::Url;

pub(crate) mod commands;

pub(crate) mod state;

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
    let open_logs = "Open logs";
    let result = app
        .dialog()
        .message(msg)
        .kind(MessageDialogKind::Error)
        .title("Fatal Error")
        .buttons(MessageDialogButtons::OkCancelCustom(
            open_logs.to_string(),
            "Close".to_string(),
        ))
        .blocking_show_with_result();

    match result {
        MessageDialogResult::Custom(text) if text == open_logs => open_log_dir(app),
        _ => {}
    };
}

pub fn open_log_dir(app: &AppHandle) {
    if let Err(err) = (|app: &AppHandle| {
        let log_dir = app.path().app_log_dir().context("Failed to get log dir")?;
        let log_dir = log_dir.to_str().context("Log dir is empty")?;

        app.opener()
            .open_path(log_dir, None::<&str>)
            .context("Failed to open log dir")
    })(app)
    {
        log::error!("Failed to open log directory: {err}");
        app.dialog()
            .message("Failed to open the logs directory.")
            .kind(MessageDialogKind::Error)
            .title("Fatal Error")
            .blocking_show();
    };
}
