use http::{header, Request, Response};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tracing::Level;
use uuid::Uuid;
use zero2prod::settings::DatabaseSettings;

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

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> Response<hyper::Body> {
        let client = hyper::Client::new();
        let request = Request::post(format!("{}/subscriptions", self.address))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body.into())
            .unwrap();
        client
            .request(request)
            .await
            .expect("failed to execute request")
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let settings = {
        let filename = Some("./tests/configuration.toml");
        let mut settings =
            zero2prod::Settings::load(filename).expect("failed to load configuration");

        settings.address = "127.0.0.1".parse().unwrap();
        settings.port = 0;
        settings.database.db_name = Uuid::new_v4().to_string();
        settings
    };

    configure_database(&settings.database).await;
    let db_pool = zero2prod::app::get_connection_pool(&settings.database);

    let server = zero2prod::Application::build(settings).expect("failed to build application");
    let port = server.port();
    let address = format!("http://127.0.0.1:{}", port);
    let _ = tokio::spawn(server);

    TestApp { address, db_pool }
}

async fn configure_database(config: &DatabaseSettings) {
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
}
