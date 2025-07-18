pub mod configuration;
pub mod email_client;
pub mod errors;
pub mod handlers;
mod telemetry;
pub mod validation;

use crate::configuration::{get_configuration, Settings};
use crate::email_client::EmailClient;
use crate::errors::AppError;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, MatchedPath};
use axum::http::{HeaderMap, Request};
use axum::response::Response;
use axum::routing::{get, post};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, PgPool};
use std::net::SocketAddr;
use std::sync::{Arc, Once};
use std::time::Duration;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::trace::TraceLayer;
use tracing::{Span, info_span};
use tracing_subscriber::EnvFilter;

pub async fn run() -> Result<(), AppError> {
    let conf = get_configuration().expect("error parsing configuration");

    let my_domain_email = conf
        .email_client
        .parse_email()
        .expect("invalid sender email addr...");
    let timeout = conf.email_client.timeout();
    let email_client = EmailClient::new(
        &conf.email_client.email_server_url,
        my_domain_email,
        &conf.email_client.authorization_token,
        timeout
    );

    let pool = PgPoolOptions::new().connect_lazy_with(conf.database.connection_options());
    let addr = format!("0.0.0.0:{}", conf.application.port);
    println!("trying to run server: {}", addr);
    // println!("db target is {}:{}", conf.database.host);

    let conn = pool
        .acquire()
        .await
        .map_err(|e| AppError::DbError(e.to_string()));
    conn?
        .ping()
        .await
        .expect("error establishing db connection");

    let app_state = AppState {
        pg_pool: Arc::new(pool),
        email_client: Arc::new(email_client),
        conf: Arc::new(conf),
    };
    let app = app_internal(app_state);

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

static TRACING: Once = Once::new();

fn init_tracing() {
    TRACING.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_target(false)
            .with_level(true)
            .with_file(false)
            .json()
            .init();
    });
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub pg_pool: Arc<PgPool>,
    pub email_client: Arc<EmailClient>,
    pub conf: Arc<Settings>
}

pub fn app_internal(app_state: AppState) -> Router {
    init_tracing();
    Router::new()
        .route("/health/{name}", get(handlers::health_check::health))
        .route("/subscription", post(handlers::subscription::subscribe))
        .route("/subscription/confirm", get(handlers::confirm_subscription::confirm))
        .with_state(app_state)
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
