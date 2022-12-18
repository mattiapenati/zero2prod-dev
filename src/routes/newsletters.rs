use http::StatusCode;

#[tracing::instrument(level = "info", name = "publish a newsletter issue")]
pub async fn publish_newsletter() -> StatusCode {
    StatusCode::OK
}
