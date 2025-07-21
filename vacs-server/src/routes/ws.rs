use crate::auth::users::Backend;
use crate::http::ApiResult;
use crate::state::AppState;
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use axum_login::login_required;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/token", get(get::token))
        .route_layer(login_required!(Backend))
}

mod get {
    use super::*;
    use crate::auth::users::AuthSession;
    use vacs_protocol::http::ws::WebSocketToken;

    pub async fn token(
        auth_session: AuthSession,
        State(state): State<Arc<AppState>>,
    ) -> ApiResult<WebSocketToken> {
        let user = auth_session.user.expect("User not logged in");

        tracing::debug!(?user, "Generating websocket token");
        let token = state.generate_ws_auth_token(user.cid.as_str()).await?;

        Ok(Json(WebSocketToken { token }))
    }
}
