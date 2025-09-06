use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;
use url::Url;

pub(crate) mod commands;

pub(crate) mod state;

pub async fn update(app: &AppHandle, updater_url: &str) -> anyhow::Result<()> {
    log::info!("Checking for updates at {updater_url}...");
    if let Some(update) = app
        .updater_builder()
        .endpoints(vec![Url::parse(updater_url)?])?
        .build()?
        .check()
        .await?
    {
        let required = update
            .raw_json
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        log::info!("Update available. Required: {required}");

        if !required {
            // TODO Ask user if he wants to update -> return if not
        }

        log::info!("Installing update");
        let mut downloaded = 0;
        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    log::debug!("Downloaded {downloaded} of {content_length:?}");
                },
                || {
                    log::debug!("Download finished");
                },
            )
            .await?;

        log::info!("Update installed. Restarting...");
        app.restart();
    }

    Ok(())
}
