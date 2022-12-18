use http::{header, Request, Response, Uri};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tracing::Level;
use uuid::Uuid;
use wiremock::MockServer;
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
    pub email_server: MockServer,
}

pub struct ConfirmationLinks {
    pub html: Uri,
    pub text: Uri,
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

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body = serde_json::from_slice::<serde_json::Value>(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();

            assert_eq!(links.len(), 1);

            let confirmation_link = links[0].as_str().parse::<Uri>().unwrap();
            format!(
                "{}{}",
                self.address,
                confirmation_link.path_and_query().unwrap()
            )
            .parse::<Uri>()
            .unwrap()
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let text = get_link(&body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, text }
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let settings = {
        let filename = Some("./tests/configuration.toml");
        let mut settings =
            zero2prod::Settings::load(filename).expect("failed to load configuration");

        settings.address = "127.0.0.1".parse().unwrap();
        settings.port = 0;
        settings.base_url = "http://127.0.0.1".to_string();
        settings.database.db_name = Uuid::new_v4().to_string();
        settings.email_client.base_url = email_server.uri();
        settings
    };

    configure_database(&settings.database).await;
    let db_pool = zero2prod::app::get_connection_pool(&settings.database);

    let server = zero2prod::Application::build(settings).expect("failed to build application");
    let port = server.port();
    let address = format!("http://127.0.0.1:{}", port);
    let _ = tokio::spawn(server);

    TestApp {
        address,
        db_pool,
        email_server,
    }
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
