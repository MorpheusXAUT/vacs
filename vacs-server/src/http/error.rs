use crate::auth::users::Backend;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_login::Error as LoginError;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemDetails {
    #[serde(rename = "type")]
    pub type_url: String,
    pub title: String,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl ProblemDetails {
    pub fn new(status: u16, title: &str) -> Self {
        Self {
            type_url: "about:blank".to_string(),
            title: title.to_string(),
            status,
            detail: None,
            instance: None,
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn with_type_url(mut self, type_url: &str) -> Self {
        self.type_url = type_url.to_string();
        self
    }

    pub fn with_detail(mut self, detail: &str) -> Self {
        self.detail = Some(detail.to_string());
        self
    }

    pub fn with_instance(mut self, instance: &str) -> Self {
        self.instance = Some(instance.to_string());
        self
    }
}

impl From<StatusCode> for ProblemDetails {
    fn from(status: StatusCode) -> Self {
        Self::new(
            status.as_u16(),
            status.canonical_reason().unwrap_or(status.as_str()),
        )
    }
}

impl IntoResponse for ProblemDetails {
    fn into_response(self) -> Response {
        (
            StatusCode::from_u16(self.status).unwrap(),
            [("Content-Type", "application/problem+json")],
            Json(self),
        )
            .into_response()
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Not Found")]
    NotFound,
    #[error(transparent)]
    InternalServerError(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::BadRequest(msg) => {
                tracing::debug!(?msg, "Bad Request");
                ProblemDetails::new(StatusCode::BAD_REQUEST.as_u16(), "Bad Request")
                    .with_detail(&msg)
            }
            AppError::Unauthorized(msg) => {
                tracing::debug!(?msg, "Unauthorized");
                ProblemDetails::new(StatusCode::UNAUTHORIZED.as_u16(), "Unauthorized")
                    .with_detail(&msg)
            }
            AppError::NotFound => ProblemDetails::new(StatusCode::NOT_FOUND.as_u16(), "Not Found"),
            AppError::InternalServerError(err) => {
                tracing::error!(?err, "Internal server error");
                ProblemDetails::new(
                    StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    "Internal Server Error",
                )
            }
        }
        .into_response()
    }
}

impl From<LoginError<Backend>> for AppError {
    fn from(err: LoginError<Backend>) -> Self {
        match err {
            LoginError::Backend(err) => err,
            LoginError::Session(err) => AppError::InternalServerError(err.into()),
        }
    }
}

impl From<String> for AppError {
    fn from(err: String) -> Self {
        AppError::InternalServerError(anyhow::anyhow!(err))
    }
}
