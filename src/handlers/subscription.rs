use crate::AppState;
use crate::email_client::EmailClient;
use crate::errors::AppError;
use anyhow::Context;
use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use garde::Validate;
use serde::Deserialize;
use sqlx::types::chrono::Utc;
use sqlx::{Executor, Postgres, Transaction};
use tracing::instrument;
use uuid::Uuid;

#[derive(Deserialize, Validate, Debug)]
pub struct SubscriberInfo {
    #[garde(email)]
    email: String,
    #[garde(length(min = 1), alphanumeric)]
    username: String,
}

#[instrument(
name = "adding a new subscriber",
parent = &tracing::Span::current(),
skip(app_state),
fields(
    subscriber_email = %form.email,
    subscriber_name = %form.username,
)
)]
pub async fn subscribe(
    State(app_state): State<AppState>,
    Form(form): Form<SubscriberInfo>,
) -> Result<StatusCode, SubscriptionError> {
    // create id to identify given request
    form.validate()?;

    let mut transaction = app_state
        .pg_pool
        .begin()
        .await
        .context("error starting transaction")?;

    let subscriber_uuid = insert_subscriber(&mut transaction, &form)
        .await.context("error registering subscriber")?;

    send_confirmation_email(&app_state, &mut transaction, &form, &subscriber_uuid)
        .await.context("error sending confirmation email to client")?;

    transaction.commit().await
        .context("error commiting transaction")?;

    tracing::info!(
        parent: &tracing::Span::current(),
        msg="successfully saved subscriber..",
    );
    Ok(StatusCode::OK)
}

async fn insert_subscriber(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    form: &SubscriberInfo,
) -> anyhow::Result<Uuid> {
    let subscriber_uuid = Uuid::new_v4();
    let add_subscriber_query = sqlx::query!(
        "insert into subscriptions (id, email, name, subscribed_at, status) values ($1, $2, $3, $4, $5)",
        &subscriber_uuid,
        form.email,
        form.username,
        Utc::now(),
        "not-confirmed"
    );

    transaction.execute(add_subscriber_query).await?;
    Ok(subscriber_uuid)
}

pub async fn send_confirmation_email(
    app_state: &AppState,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    new_subscriber: &SubscriberInfo,
    subscriber_uuid: &Uuid,
) -> anyhow::Result<()> {
    let confirmation_link = app_state.conf.application.host.clone();
    // todo: uuidを生成してuserのuuidと紐づけ(7/14)
    let uuid_token_for_confirmation = Uuid::new_v4();

    let insert_new_subscriber_query = sqlx::query!(
        "INSERT INTO subscription_tokens (subscription_token, subscriber_uuid) VALUES ($1, $2)",
        uuid_token_for_confirmation.to_string(),
        subscriber_uuid,
    );
    let res = transaction
        .execute(insert_new_subscriber_query)
        .await
        .map_err(|e| {
            tracing::error!(
                parent: &tracing::Span::current(),
                error.kind="db registering subscription",
            );
            AppError::DbError(e.to_string())
        })?;
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit http://{confirmation_link}/subscription/confirm to confirm your subscription.",
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
Click <a href=http://\"{confirmation_link}\">here</a> to confirm your subscription.",
    );
    app_state
        .email_client
        .send_email(&new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
}

#[derive(Debug, thiserror::Error)]
pub enum SubscriptionError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    
    #[error("{0}")]
    ValidationError(#[from] garde::Report)
}

impl IntoResponse for SubscriptionError {
    fn into_response(self) -> Response {
        match self {
            SubscriptionError::UnexpectedError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            }
            SubscriptionError::ValidationError(e) => {
                (StatusCode::BAD_REQUEST, e.to_string()).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use fake::Fake;
    use garde::rules::email::parse_email;

    #[test]
    fn email_validity_check() {
        for _ in 0..10 {
            let email = fake::faker::internet::en::SafeEmail().fake::<String>();
            println!("tested: {email}");
            assert!(parse_email(email.as_str()).is_ok())
        }
    }
}
