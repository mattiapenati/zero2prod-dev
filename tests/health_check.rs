use std::net::TcpListener;

use hyper::{header, Request};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tracing::Level;
use uuid::Uuid;
use zero2prod::settings::DatabaseSettings;

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

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;
    let client = hyper::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let request = Request::post(format!("{}/subscriptions", app.address))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body.into())
        .unwrap();
    let response = client
        .request(request)
        .await
        .expect("failed to execute request");

    assert_eq!(200, response.status());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("failed to fetch saved subscription");

    assert_eq!("ursula_le_guin@gmail.com", saved.email);
    assert_eq!("le guin", saved.name);
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let client = hyper::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let request = Request::post(format!("{}/subscriptions", app.address))
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

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    let app = spawn_app().await;
    let client = hyper::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let request = Request::post(format!("{}/subscriptions", app.address))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body.into())
            .unwrap();
        let response = client
            .request(request)
            .await
            .expect("failed to execute request");

        assert_eq!(
            400,
            response.status(),
            "the API did not return 400 Bad Request when payload was {}",
            description,
        );
    }
}

async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let filename = Some("./tests/configuration.toml");
    let mut settings = zero2prod::Settings::load(filename).expect("failed to load configuration");
    settings.database.db_name = Uuid::new_v4().to_string();

    let db_pool = configure_database(&settings.database).await;

    let server = zero2prod::app::run(listener, db_pool.clone()).expect("failed to bind address");
    let _ = tokio::spawn(server);

    TestApp { address, db_pool }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.connect_options_without_db())
        .await
        .expect("failed to connect to database");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.db_name).as_str())
        .await
        .expect("failed to create database");

    let db_pool = PgPool::connect_with(config.connect_options())
        .await
        .expect("failed to connect to database");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("failed to migrate the database");

    db_pool
}

static TRACING: Lazy<()> = Lazy::new(|| {
    use zero2prod::trace::*;

    let subscriber = if std::env::var("TEST_LOG").is_ok() {
        get_subscriber(TraceSettings {
            level: Level::DEBUG,
            writer: stdout(),
            endpoint: None,
            namespace: None,
        })
    } else {
        get_subscriber(TraceSettings {
            level: Level::DEBUG,
            writer: std::io::sink,
            endpoint: None,
            namespace: None,
        })
    };

    init_subscriber(subscriber);
});

struct TestApp {
    address: String,
    db_pool: PgPool,
}
