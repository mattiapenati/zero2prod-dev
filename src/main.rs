use std::net::TcpListener;

use zero2prod::{app::run, settings::Settings};

#[tokio::main]
async fn main() -> hyper::Result<()> {
    let settings = Settings::load().expect("failed to load configuration");
    let address = format!("127.0.0.1:{}", settings.port);
    let listener = TcpListener::bind(address).expect("failed to bind address");
    run(listener)?.await
}
