use crate::auth::users::Backend;
use crate::state::AppState;
use axum::Router;
use axum::routing::get;
use axum_login::login_required;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/ice-config",
        get(get::ice_config).layer(login_required!(Backend)),
    )
}

mod get {
    use super::*;
    use crate::auth::users::AuthSession;
    use crate::http::ApiResult;
    use axum::Json;
    use axum::extract::State;
    use vacs_protocol::http::webrtc::IceConfig;

    pub async fn ice_config(
        auth_session: AuthSession,
        State(state): State<Arc<AppState>>,
    ) -> ApiResult<IceConfig> {
        let user = auth_session.user.expect("User not logged in");

        tracing::debug!(?user, "Retrieving ICE config for user");
        let config = state.ice_config_provider.get_ice_config(&user.cid).await?;

        Ok(Json(config))
    }
}
