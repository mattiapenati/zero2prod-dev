use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod::{
    app::run,
    settings::Settings,
    trace::{get_subscriber, init_subscriber, stdout, TraceSettings},
};

#[tokio::main]
async fn main() -> hyper::Result<()> {
    let settings = Settings::load().expect("failed to load configuration");

    let subscriber = get_subscriber(TraceSettings {
        level: settings.log.level,
        writer: stdout(),
        endpoint: settings.log.endpoint.as_deref(),
        namespace: settings.log.namespace.as_deref(),
    });
    init_subscriber(subscriber);

    let address = format!("127.0.0.1:{}", settings.port);
    let listener = TcpListener::bind(address).expect("failed to bind address");
    let db_pool = PgPool::connect_with(settings.database.connect_options())
        .await
        .expect("failed to connect to database");
    run(listener, db_pool)?.await
}
