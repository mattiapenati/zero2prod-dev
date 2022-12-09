use std::net::TcpListener;

use env_logger::Env;
use sqlx::PgPool;
use zero2prod::{app::run, settings::Settings};

#[tokio::main]
async fn main() -> hyper::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let settings = Settings::load().expect("failed to load configuration");
    let address = format!("127.0.0.1:{}", settings.port);
    let listener = TcpListener::bind(address).expect("failed to bind address");
    let db_pool = PgPool::connect_with(settings.database.connect_options())
        .await
        .expect("failed to connect to database");
    run(listener, db_pool)?.await
}
