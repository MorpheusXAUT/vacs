use crate::auth::users::{AuthSession, Backend, Credentials};
use crate::http::ApiResult;
use crate::http::error::AppError;
use crate::state::AppState;
use anyhow::Context;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_login::login_required;
use std::sync::Arc;
use tower_sessions::Session;
use vacs_protocol::http::auth::UserInfo;

const VATSIM_OAUTH_CSRF_TOKEN_KEY: &str = "vatsim.oauth.csrf_token";

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/vatsim", get(get::vatsim))
        .route("/vatsim/callback", post(post::vatsim_callback))
        .route("/user", get(get::user_info).layer(login_required!(Backend)))
        .route(
            "/logout",
            post(post::logout).layer(login_required!(Backend)),
        )
}

mod get {
    use super::*;
    use vacs_protocol::http::auth::InitVatsimLogin;

    pub async fn vatsim(auth_session: AuthSession, session: Session) -> ApiResult<InitVatsimLogin> {
        let (url, csrf_token) = auth_session.backend.authorize_url();

        session
            .insert(VATSIM_OAUTH_CSRF_TOKEN_KEY, csrf_token)
            .await
            .context("Failed to store CSRF token in session")?;

        Ok(Json(InitVatsimLogin {
            url: url.to_string(),
        }))
    }

    pub async fn user_info(auth_session: AuthSession) -> ApiResult<UserInfo> {
        let user = auth_session.user.expect("User not logged in");

        Ok(Json(UserInfo { cid: user.cid }))
    }
}

mod post {
    use super::*;
    use crate::http::StatusCodeResult;
    use axum::http::StatusCode;
    use vacs_protocol::http::auth::AuthExchangeToken;

    pub async fn vatsim_callback(
        mut auth_session: AuthSession,
        session: Session,
        Json(AuthExchangeToken { code, state }): Json<AuthExchangeToken>,
    ) -> ApiResult<UserInfo> {
        let stored_state = session
            .remove::<String>(VATSIM_OAUTH_CSRF_TOKEN_KEY)
            .await
            .context("Failed to remove CSRF token from session")?
            .ok_or(AppError::Unauthorized("Missing CSRF token".to_string()))?;

        let creds = Credentials {
            code,
            received_state: state,
            stored_state,
        };

        tracing::debug!("Authenticating with VATSIM");
        let user = match auth_session.authenticate(creds).await {
            Ok(Some(user)) => user,
            Ok(None) => return Err(AppError::Unauthorized("Invalid credentials".to_string())),
            Err(err) => return Err(err.into()),
        };

        auth_session
            .login(&user)
            .await
            .context("Failed to login user")?;

        Ok(Json(UserInfo { cid: user.cid }))
    }

    pub async fn logout(mut auth_session: AuthSession, session: Session) -> StatusCodeResult {
        tracing::debug!("Logging user out and destroying session");
        auth_session.logout().await.context("Failed to logout")?;
        session
            .delete()
            .await
            .context("Failed to destroy session")?;

        Ok(StatusCode::NO_CONTENT)
    }
}
