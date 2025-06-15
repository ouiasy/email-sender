pub mod configuration;
pub mod errors;
mod handlers;
mod telemetry;
#[path = "middleware-examples.rs"]
use crate::configuration::get_configuration;
use crate::errors::AppError;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, MatchedPath};
use axum::http::{HeaderMap, Request};
use axum::response::Response;
use axum::routing::{get, post};
use sqlx::PgPool;
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::instrument::WithSubscriber;
use tracing::{Span, info_span};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::util::SubscriberInitExt;

pub async fn run() -> Result<(), AppError> {
    let conf = get_configuration().expect("error parsing configuration");

    // todo: optionで詳細の設定
    let conn = PgPool::connect_lazy(&conf.database.connection_string())
        // .await
        .expect("error getting connection pool from postgres");

    let app = app_internal(conn);
    let addr = format!("0.0.0.0:{}", conf.application.port);

    println!("running server: {}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::EstablishServer(e.to_string()))?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(|e| AppError::EstablishServer(e.to_string()))
}

pub fn app_internal(pg_pool: PgPool) -> Router {
    tracing_subscriber::fmt()
        // ↓ 環境変数RUST_LOGからfilterするlevelを決定する
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .with_level(true)
        .with_file(false)
        // .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .json()
        .init();
    Router::new()
        .route("/health/{name}", get(handlers::health_check::health))
        .route("/subscription", post(handlers::subscription::subscribe))
        .with_state(pg_pool)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    // Log the matched route's path (with placeholders not filled in).
                    // Use request.uri() or OriginalUri if you want the real path.
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);
                    let client_ip = request
                        .extensions()
                        .get::<ConnectInfo<SocketAddr>>()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    info_span!(
                        "request_to_email_sender",
                        method = ?request.method(),
                        matched_path,
                        client_ip=%client_ip,
                        // some_other_field = tracing::field::Empty,
                    )
                })
                .on_request(|_request: &Request<_>, _span: &Span| {
                    // You can use `_span.record("some_other_field", value)` in one of these
                    // closures to attach a value to the initially empty field in the info_span
                    // created above.
                })
                .on_response(|_response: &Response, _latency: Duration, _span: &Span| {
                    // ...
                })
                .on_body_chunk(|_chunk: &Bytes, _latency: Duration, _span: &Span| {
                    // ...
                })
                .on_eos(
                    |_trailers: Option<&HeaderMap>, _stream_duration: Duration, _span: &Span| {
                        // ...
                    },
                )
                .on_failure(
                    |error: ServerErrorsFailureClass, latency: Duration, span: &Span| {
                        span.record("error_type", &tracing::field::display(&error));
                        tracing::error!("error: {:?}", error);
                    },
                ),
        )
}
