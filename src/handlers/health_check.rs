use axum::extract::{Path, State};
use axum::http::StatusCode;
use sqlx::PgPool;

pub async fn health(
    Path(name): Path<String>,
    State(_): State<PgPool>,
) -> Result<(StatusCode, String), StatusCode> {
    if name == "me" {
        tracing::info!("here bad ans");
        return Err(StatusCode::NOT_FOUND);
    }
    Ok((StatusCode::OK, format!("hello {name}")))
}
