mod auth;
mod root;
mod version;
mod ws;

use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::Request;
use axum::middleware::Next;
use axum::{Router, extract, middleware};
use axum_client_ip::{ClientIp, ClientIpSource};
use axum_login::{AuthManagerLayer, AuthnBackend};
use std::sync::Arc;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tower_sessions::SessionStore;
use tower_sessions::service::SignedCookie;
use tracing::{Span, debug_span};

pub fn create_app<B, S>(
    auth_layer: AuthManagerLayer<B, S, SignedCookie>,
    client_ip_source: ClientIpSource,
) -> Router<Arc<AppState>>
where
    B: AuthnBackend + Send + Sync + 'static + Clone,
    S: SessionStore + Send + Sync + 'static + Clone,
{
    Router::new()
        .nest("/auth", auth::routes())
        .nest("/ws", ws::routes().merge(crate::ws::routes()))
        .nest("/version", version::routes())
        .merge(root::routes())
        .layer(middleware::from_fn(
            async |request: extract::Request, next: Next| {
                let (mut parts, body) = request.into_parts();
                if let Ok(ip) = ClientIp::from_request_parts(&mut parts, &()).await {
                    Span::current().record("client_ip", ip.0.to_string());
                }
                next.run(Request::from_parts(parts, body)).await
            },
        ))
        .layer(
            TraceLayer::new_for_http().make_span_with(move |req: &Request<_>| {
                let path = req.uri().path();
                match path {
                    "/health" | "/favicon.ico" => Span::none(),
                    _ => debug_span!(
                        "request",
                        method = %req.method(),
                        uri = %req.uri(),
                        version = ?req.version(),
                        client_ip = tracing::field::Empty),
                }
            }),
        )
        .merge(root::untraced_routes())
        .layer(TimeoutLayer::new(crate::config::SERVER_SHUTDOWN_TIMEOUT))
        .layer(auth_layer)
        .layer(client_ip_source.into_extension())
}
