use axum::{routing, Router};
use hyper::StatusCode;

pub async fn run() -> hyper::Result<()> {
    hyper::Server::bind(&"127.0.0.1:8000".parse().expect("invalid bind address"))
        .serve(
            Router::new()
                .route("/health_check", routing::get(health_check))
                .into_make_service(),
        )
        .await
}

async fn health_check() -> StatusCode {
    StatusCode::NO_CONTENT
}
