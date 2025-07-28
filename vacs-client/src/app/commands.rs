#[tauri::command]
pub fn app_frontend_ready() {
    log::info!("Frontend ready");
}
