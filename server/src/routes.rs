use axum::{extract::Path, response::IntoResponse};
use tracing::debug;

pub async fn post_new() -> impl IntoResponse {}

pub async fn get_id(Path(id): Path<String>) -> impl IntoResponse {
    debug!(id, "getting level from DB")
}
