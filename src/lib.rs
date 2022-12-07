use std::net::TcpListener;

use axum::{routing, Router};
use futures::future::BoxFuture;
use hyper::StatusCode;

pub type Server = BoxFuture<'static, hyper::Result<()>>;

pub fn run(listener: TcpListener) -> hyper::Result<Server> {
    let server = hyper::Server::from_tcp(listener)?.serve(
        Router::new()
            .route("/health_check", routing::get(health_check))
            .into_make_service(),
    );
    Ok(Box::pin(server))
}

async fn health_check() -> StatusCode {
    StatusCode::NO_CONTENT
}
