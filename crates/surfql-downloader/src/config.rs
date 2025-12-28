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
    pub STORAGE_ACCOUNT_ENDPOINT: String,
    pub STORAGE_CONTAINER: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Self {
            STORAGE_ACCOUNT_ENDPOINT: get_env("STORAGE_ACCOUNT_ENDPOINT")?,
            STORAGE_CONTAINER: get_env("STORAGE_CONTAINER")?,
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
