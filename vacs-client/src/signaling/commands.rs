use crate::app::state::AppState;
use crate::error::Error;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn signaling_connect(app: AppHandle) -> Result<(), Error> {
    app.state::<AppState>().lock().await.connect(&app).await
}
