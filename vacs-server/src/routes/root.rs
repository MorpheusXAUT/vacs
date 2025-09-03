use crate::state::AppState;
use axum::Router;
use axum::routing::get;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(get::health))
        .route("/version", get(get::version))
}

mod get {
    use crate::build::VersionInfo;
    use crate::http::ApiResult;
    use crate::state::AppState;
    use axum::Json;
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use std::sync::Arc;
    use std::time::Duration;

    pub async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
        match tokio::time::timeout(Duration::from_secs(3), state.health_check()).await {
            Ok(Ok(_)) => (StatusCode::OK, "OK"),
            _ => (StatusCode::SERVICE_UNAVAILABLE, "Service Unavailable"),
        }
    }

    pub async fn version() -> ApiResult<VersionInfo> {
        Ok(Json(VersionInfo::gather()))
    }
}
