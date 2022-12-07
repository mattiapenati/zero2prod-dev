use std::net::TcpListener;

use axum::{routing, Form, Router};
use futures::future::BoxFuture;
use hyper::StatusCode;
use serde::Deserialize;

pub type Server = BoxFuture<'static, hyper::Result<()>>;

pub fn run(listener: TcpListener) -> hyper::Result<Server> {
    let server = hyper::Server::from_tcp(listener)?.serve(
        Router::new()
            .route("/health_check", routing::get(health_check))
            .route("/subscriptions", routing::post(subscribe))
            .into_make_service(),
    );
    Ok(Box::pin(server))
}

async fn health_check() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn subscribe(Form(_form): Form<FormData>) -> StatusCode {
    StatusCode::OK
}

#[derive(Deserialize)]
struct FormData {
    email: String,
    name: String,
}
