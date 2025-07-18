use crate::{errors::AppError, validation::ValidatedEmail};
use config::{Config, File, FileFormat};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

#[derive(serde::Deserialize, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
}

#[derive(serde::Deserialize, Debug)]
pub struct EmailClientSettings {
    pub email_server_url: String,
    pub sender_email: String,
    pub authorization_token: String,
    pub timeout_seconds: u64
}

impl EmailClientSettings {
    pub fn parse_email(&self) -> anyhow::Result<ValidatedEmail> {
        ValidatedEmail::parse(&self.sender_email)
    }
    
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout_seconds)
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub require_ssl: bool,
    pub database_name: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    // pub base_url: String,
}

impl DatabaseSettings {
    pub fn connection_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        let opt = PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(&self.password)
            .database(&self.database_name)
            .port(self.port)
            .ssl_mode(ssl_mode);
        opt
    }
}

pub fn get_configuration() -> Result<Settings, AppError> {
    let base_path = std::env::current_dir().expect("error to find current dir");
    let config_dir = base_path.join("config");
    let env: Env = std::env::var("APP_ENV")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("error parsing Env: APP_ENV");
    let additional_conf_name = format!("{}.yaml", env.as_str());

    let settings = Config::builder()
        .add_source(config::File::from(config_dir.join("base.yaml")))
        .add_source(File::new(
            config_dir
                .join(additional_conf_name.as_str())
                .to_str()
                .expect("error parsing additional yaml file..."),
            FileFormat::Yaml,
        ))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()
        .map_err(|e| AppError::ConfigError(e.to_string()))?;
    settings
        .try_deserialize::<Settings>()
        .map_err(|e| AppError::ConfigError(e.to_string()))
}

pub enum Env {
    Local,
    Production,
}

impl Env {
    pub fn as_str(&self) -> &str {
        match self {
            Env::Local => "local",
            Env::Production => "production",
        }
    }
}

impl TryFrom<String> for Env {
    type Error = AppError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            _ => Err(AppError::EnvError(value)),
        }
    }
}
