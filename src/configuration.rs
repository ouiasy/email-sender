use crate::errors::AppError;
use config::{Config, Environment, File, FileFormat};
use tracing_subscriber::fmt::format;

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }
}

pub fn get_configuration() -> Result<Settings, AppError> {
    let base_path = std::env::current_dir().expect("error to find current dir");
    let config_dir = base_path.join("config");

    let env: Env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".into())
        .try_into().expect("error parsing Env: APP_ENV");
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
