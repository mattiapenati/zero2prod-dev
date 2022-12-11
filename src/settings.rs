use config::{Config, ConfigError, File, FileFormat};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use sqlx::postgres::PgConnectOptions;

#[derive(Deserialize)]
pub struct Settings {
    pub port: u16,
    pub log: LogSettings,
    pub database: DatabaseSettings,
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
}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        Config::builder()
            .set_default("port", "8000")?
            .set_default("log.level", "info")?
            .set_default("database.host", "localhost")?
            .set_default("database.port", "5432")?
            .add_source(File::new("configuration.toml", FileFormat::Toml))
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
