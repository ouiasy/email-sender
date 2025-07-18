use axum::extract::{Path, };
use axum::http::StatusCode;


pub async fn health(
    Path(name): Path<String>,
) -> Result<(StatusCode, String), StatusCode> {
    if name == "me" {
        tracing::info!("here bad ans");
        return Err(StatusCode::NOT_FOUND);
    }
    Ok((StatusCode::OK, format!("hello {name}")))
}
