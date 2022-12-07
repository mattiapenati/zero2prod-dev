use std::net::TcpListener;

use zero2prod::run;

#[tokio::main]
async fn main() -> hyper::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8000").expect("failed to bind address");
    run(listener)?.await
}
