use std::{
    fmt::{self, Display},
    future::Future,
    net::{SocketAddr, TcpListener},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use axum::{routing, Router};
use axum_macros::FromRef;
use futures::future::BoxFuture;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower::ServiceBuilder;
use tower_http::{request_id::MakeRequestUuid, trace::TraceLayer, ServiceBuilderExt};

use crate::{email_client::EmailClient, routes, settings::DatabaseSettings, trace, Settings};

pub type Server = BoxFuture<'static, hyper::Result<()>>;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub fn build(settings: Settings) -> hyper::Result<Self> {
        let db_pool = get_connection_pool(&settings.database);
        let sender_email = settings
            .email_client
            .sender()
            .expect("invalid sender email address");
        let timeout = settings.email_client.timeout;
        let email_client = EmailClient::new(
            settings.email_client.base_url,
            sender_email,
            settings.email_client.authorization_token,
            timeout,
        );

        let address = format!("{}:{}", settings.address, settings.port);
        let listener = TcpListener::bind(address).expect("failed to bind address");
        let address = listener.local_addr().unwrap();
        tracing::info!("serving on {}", address);

        let port = address.port();
        let server = run(listener, db_pool, email_client, settings.base_url)?;
        Ok(Application { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Future for Application {
    type Output = hyper::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.server.as_mut().poll(cx)
    }
}

pub fn get_connection_pool(settings: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(settings.connect_options())
}

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> hyper::Result<Server> {
    let state = AppState {
        db_pool,
        email_client,
        base_url: BaseUrl(base_url),
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
            .route("/subscriptions/confirm", routing::get(routes::confirm))
            .route("/newsletters", routing::post(routes::publish_newsletter))
            .layer(middleware)
            .with_state(state)
            .into_make_service_with_connect_info::<SocketAddr>(),
    );
    Ok(Box::pin(server))
}

#[derive(Clone, Debug)]
pub struct BaseUrl(String);

impl Display for BaseUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[derive(Clone, FromRef)]
struct AppState {
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: BaseUrl,
}
