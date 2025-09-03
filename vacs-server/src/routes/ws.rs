use crate::auth::users::AuthSession;
use crate::auth::users::Backend;
use crate::http::ApiResult;
use crate::state::AppState;
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::{delete, get};
use axum_login::login_required;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/token", get(get::token).layer(login_required!(Backend)))
        .route(
            "/",
            delete(delete::terminate_connection).layer(login_required!(Backend)),
        )
}

mod get {
    use super::*;
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

mod delete {
    use super::*;
    use crate::http::StatusCodeResult;
    use axum::http::StatusCode;

    pub async fn terminate_connection(
        auth_session: AuthSession,
        State(state): State<Arc<AppState>>,
    ) -> StatusCodeResult {
        let user = auth_session.user.expect("User not logged in");

        tracing::debug!(?user, "Terminating existing web socket connection");
        state.unregister_client(user.cid.as_str()).await;

        Ok(StatusCode::NO_CONTENT)
    }
}
