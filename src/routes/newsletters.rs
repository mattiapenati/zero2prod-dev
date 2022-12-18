use axum::Json;
use http::StatusCode;
use serde::Deserialize;

#[tracing::instrument(level = "info", name = "publish a newsletter issue", skip(body))]
pub async fn publish_newsletter(Json(body): Json<BodyData>) -> StatusCode {
    StatusCode::OK
}

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
struct Content {
    html: String,
    text: String,
}
