use std::net::TcpListener;

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app();
    let client = hyper::Client::new();

    let response = client
        .get(format!("{}/health_check", address).parse().unwrap())
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(0, hyper::body::to_bytes(response).await.unwrap().len());
}

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::run(listener).expect("failed to bind address");
    let _ = tokio::spawn(server);
    format!("http://127.0.0.1:{}", port)
}
