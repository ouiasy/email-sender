use crate::AppState;
use crate::errors::AppError;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;
use anyhow::Context;

#[derive(thiserror::Error, Debug)]
pub enum ConfirmationError {
    #[error("{0}")]
    ConfirmationError(#[from] anyhow::Error)
}

impl IntoResponse for ConfirmationError {
    fn into_response(self) -> Response {
        match self {
            ConfirmationError::ConfirmationError(error) => {
                (StatusCode::BAD_REQUEST, error.to_string()).into_response()
            },
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Parameters {
    token: String, // dbのuuidに相当
}
#[instrument(name = "confirm a pending subscriber")]
pub async fn confirm(
    State(app_state): State<AppState>,
    Query(param): Query<Parameters>,
) -> Result<StatusCode, ConfirmationError> {
    let subscriber_uuid = get_subscriber_uuid_from_token(&app_state.pg_pool, param.token)
        .await?;
    confirm_subscriber(&app_state.pg_pool, subscriber_uuid).await?;
    Ok(StatusCode::OK)
}

async fn confirm_subscriber(pool: &PgPool, subscriber_uuid: Uuid) -> anyhow::Result<()> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_uuid,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn get_subscriber_uuid_from_token(
    pool: &PgPool,
    token: String,
) -> anyhow::Result<Uuid> {
    let res = sqlx::query!(
        r"SELECT subscriber_uuid FROM subscription_tokens WHERE subscription_token = $1",
        token
    )
    .fetch_one(pool)
    .await
    .context("cannot find subscriber from the token")?;
    Ok(res.subscriber_uuid)
}
