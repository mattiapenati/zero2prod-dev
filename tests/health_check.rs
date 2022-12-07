use std::net::TcpListener;

use hyper::{header, Request};
use sqlx::{Connection, PgConnection};
use zero2prod::settings::Settings;

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

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let address = spawn_app();
    let settings = Settings::load().expect("failed to load configuration");
    let mut connection = PgConnection::connect_with(&settings.database.connect_options())
        .await
        .expect("failed to connect to database");
    let client = hyper::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let request = Request::post(format!("{}/subscriptions", address))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body.into())
        .unwrap();
    let response = client
        .request(request)
        .await
        .expect("failed to execute request");

    assert_eq!(200, response.status());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&mut connection)
        .await
        .expect("failed to fetch saved subscription");

    assert_eq!("ursula_le_quin@gmail.com", saved.email);
    assert_eq!("le guin", saved.name);
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let address = spawn_app();
    let client = hyper::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let request = Request::post(format!("{}/subscriptions", address))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(invalid_body.into())
            .unwrap();
        let response = client
            .request(request)
            .await
            .expect("failed to execute request");

        assert_eq!(
            400,
            response.status(),
            "the API did not fail with 400 Bad Request when the payload was {}",
            error_message,
        );
    }
}

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::app::run(listener).expect("failed to bind address");
    let _ = tokio::spawn(server);
    format!("http://127.0.0.1:{}", port)
}
