use std::{net::IpAddr, time::Duration};

use config::{Config, ConfigError, Environment, File, FileFormat};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use sqlx::postgres::PgConnectOptions;

use crate::domain::SubscriberEmail;

#[derive(Deserialize)]
pub struct Settings {
    pub address: IpAddr,
    pub port: u16,
    pub base_url: String,
    pub log: LogSettings,
    pub database: DatabaseSettings,
    pub email_client: EmailClientSettings,
}

#[serde_as]
#[derive(Deserialize)]
pub struct LogSettings {
    #[serde_as(as = "DisplayFromStr")]
    pub level: tracing::Level,
    pub endpoint: Option<String>,
    pub namespace: Option<String>,
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Secret<String>,
    pub db_name: String,
    pub migrate: bool,
}

#[derive(Deserialize)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: Secret<String>,
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
}

impl Settings {
    pub fn load(filename: Option<&str>) -> Result<Self, ConfigError> {
        let mut config = Config::builder()
            .set_default("address", "127.0.0.1")?
            .set_default("port", "8000")?
            .set_default("base_url", "http://127.0.0.1")?
            .set_default("log.level", "info")?
            .set_default("database.host", "localhost")?
            .set_default("database.port", "5432")?
            .set_default("database.migrate", "false")?
            .set_default("email_client.timeout", "10s")?;

        if let Some(filename) = filename {
            config = config.add_source(File::new(filename, FileFormat::Toml).required(true))
        }

        config
            .add_source(
                Environment::with_prefix("ZERO2PROD")
                    .separator("__")
                    .prefix_separator("__"),
            )
            .build()?
            .try_deserialize()
    }
}

impl DatabaseSettings {
    pub fn connect_options(&self) -> PgConnectOptions {
        self.connect_options_without_db().database(&self.db_name)
    }

    pub fn connect_options_without_db(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .username(&self.username)
            .password(self.password.expose_secret())
    }
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }
}
