use crate::{Error, Result};
use dotenv::dotenv;
use std::env;
use std::sync::OnceLock;
use tracing::warn;

pub fn load_config() -> &'static Config {
    dotenv().ok();

    static INSTANCE: OnceLock<Config> = OnceLock::new();

    INSTANCE.get_or_init(|| {
        Config::from_env().unwrap_or_else(|e| panic!("FATAL: Failed loading env variable. {:?}", e))
    })
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    #[cfg(feature = "azure")]
    pub AZURE_TENANT_ID: String,
    #[cfg(feature = "azure")]
    pub AZURE_CLIENT_ID: String,
    #[cfg(feature = "azure")]
    pub AZURE_CLIENT_SECRET: String,
    #[cfg(feature = "azure-storage")]
    pub STORAGE_ACCOUNT_ENDPOINT: String,
    #[cfg(feature = "azure-storage")]
    pub STORAGE_CONTAINER: String,
    #[cfg(feature = "rabbitmq")]
    pub RABBITMQ_CONNECTION_STRING: String,
    #[cfg(feature = "telemetry")]
    pub OTLP_COLLECTOR_URL: String,
    pub WORKER_NAME: String,
    pub BASE_URL: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "azure")]
            AZURE_TENANT_ID: get_env("AZURE_TENANT_ID")?,
            #[cfg(feature = "azure")]
            AZURE_CLIENT_ID: get_env("AZURE_CLIENT_ID")?,
            #[cfg(feature = "azure")]
            AZURE_CLIENT_SECRET: get_env("AZURE_CLIENT_SECRET")?,
            #[cfg(feature = "azure-storage")]
            STORAGE_ACCOUNT_ENDPOINT: get_env("STORAGE_ACCOUNT_ENDPOINT")?,
            #[cfg(feature = "azure-storage")]
            STORAGE_CONTAINER: get_env("STORAGE_CONTAINER")?,
            #[cfg(feature = "rabbitmq")]
            RABBITMQ_CONNECTION_STRING: get_env("RABBITMQ_CONNECTION_STRING")?,
            #[cfg(feature = "telemetry")]
            OTLP_COLLECTOR_URL: get_env("OTLP_COLLECTOR_URL")?,
            WORKER_NAME: get_env_opt("WORKER_NAME", "test-worker"),
            BASE_URL: get_env_opt("BASE_URL", "https://data.commoncrawl.org"),
        })
    }
}

fn get_env(key: &'static str) -> Result<String> {
    env::var(key).map_err(|_| Error::ConfigMissingEnv(key))
}

fn get_env_opt(key: &str, default: &str) -> String {
    warn!(
        "Failed loading env variable: {}. Using fallback value: {}",
        key, default
    );
    env::var(key).unwrap_or_else(|_| default.to_string())
}
