use axum::Form;
use axum::extract::{ConnectInfo, OriginalUri, Request, State};
use axum::http::{Method, StatusCode, Uri};
use serde::Deserialize;
use sqlx::PgPool;
use sqlx::types::chrono::Utc;
use std::net::SocketAddr;
use tracing::instrument;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct Subscribe {
    email: String,
    username: String,
}

// #[instrument(parent = &tracing::Span::current(), skip(db_pool, subscribe))]
pub async fn subscribe(
    State(db_pool): State<PgPool>,
    Form(subscribe): Form<Subscribe>,
) -> Result<StatusCode, StatusCode> {
    // create id to identify given request
    let request_id = Uuid::new_v4();

    //todo: hashing or encryption is needed to log personal info..
    let request_span = tracing::info_span!(
        parent: &tracing::Span::current(),
        "Adding a new subscriber.",
        user=%subscribe.username,
        email=%subscribe.email,
        // request_uri=%addr,
        // method=%method.to_string(),
    );
    // Using `enter` in an async function is a recipe for disaster!
    // Bear with me for now, but don't do this at home.
    // See the following section on `Instrumenting Futures`
    // let _request_span_guard = request_span.enter();

    // tracing::info!("request_id: {} - Adding subscriber `{}`", request_id, subscribe.email);
    sqlx::query!(
        "insert into subscriptions (id, email, name, subscribed_at) values ($1, $2, $3, $4)",
        Uuid::new_v4(),
        subscribe.email,
        subscribe.username,
        Utc::now(),
    )
    .execute(&db_pool)
    .await
    .map_err(|e| {
        tracing::error!(
            parent: &request_span,
            error.kind="db registering subscription",
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!(
        parent: &request_span,
        msg="successfully saved subscriber..",
    );
    Ok(StatusCode::OK)
}
