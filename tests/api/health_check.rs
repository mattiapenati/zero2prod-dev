use crate::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;
    let client = hyper::Client::new();

    let response = client
        .get(format!("{}/health_check", app.address).parse().unwrap())
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(0, hyper::body::to_bytes(response).await.unwrap().len());
}
