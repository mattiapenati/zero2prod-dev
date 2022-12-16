use std::net::{SocketAddr, TcpListener};

use axum::{routing, Router};
use axum_macros::FromRef;
use futures::future::BoxFuture;
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::{request_id::MakeRequestUuid, trace::TraceLayer, ServiceBuilderExt};

use crate::{email_client::EmailClient, routes, trace};

pub type Server = BoxFuture<'static, hyper::Result<()>>;

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> hyper::Result<Server> {
    let state = AppState {
        db_pool,
        email_client,
    };
    let middleware = ServiceBuilder::new()
        .set_x_request_id(MakeRequestUuid)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::MakeSpan)
                .on_response(trace::OnResponse),
        )
        .propagate_x_request_id();
    let server = hyper::Server::from_tcp(listener)?.serve(
        Router::new()
            .route("/health_check", routing::get(routes::health_check))
            .route("/subscriptions", routing::post(routes::subscribe))
            .layer(middleware)
            .with_state(state)
            .into_make_service_with_connect_info::<SocketAddr>(),
    );
    Ok(Box::pin(server))
}

#[derive(Clone, FromRef)]
struct AppState {
    db_pool: PgPool,
    email_client: EmailClient,
}
