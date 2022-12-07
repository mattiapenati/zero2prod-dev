use std::net::TcpListener;

use axum::{routing, Router};
use futures::future::BoxFuture;

use crate::routes;

pub type Server = BoxFuture<'static, hyper::Result<()>>;

pub fn run(listener: TcpListener) -> hyper::Result<Server> {
    let server = hyper::Server::from_tcp(listener)?.serve(
        Router::new()
            .route("/health_check", routing::get(routes::health_check))
            .route("/subscriptions", routing::post(routes::subscribe))
            .into_make_service(),
    );
    Ok(Box::pin(server))
}
