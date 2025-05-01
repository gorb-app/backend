use crate::Error;
use log::debug;
use serde::Deserialize;
use sqlx::postgres::PgConnectOptions;
use tokio::fs::read_to_string;

#[derive(Debug, Deserialize)]
pub struct ConfigBuilder {
    database: Database,
    web: Option<WebBuilder>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
    username: String,
    password: String,
    host: String,
    database: String,
    port: u16,
}

#[derive(Debug, Deserialize)]
struct WebBuilder {
    url: Option<String>,
    port: Option<u16>,
    _ssl: Option<bool>,
}

impl ConfigBuilder {
    pub async fn load(path: String) -> Result<Self, Error> {
        debug!("loading config from: {}", path);
        let raw = read_to_string(path).await?;

        let config = toml::from_str(&raw)?;

        Ok(config)
    }

    pub fn build(self) -> Config {
        let web = if let Some(web) = self.web {
            Web {
                url: web.url.unwrap_or(String::from("0.0.0.0")),
                port: web.port.unwrap_or(8080),
            }
        } else {
            Web {
                url: String::from("0.0.0.0"),
                port: 8080,
            }
        };

        Config {
            database: self.database,
            web,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database: Database,
    pub web: Web,
}

#[derive(Debug, Clone)]
pub struct Web {
    pub url: String,
    pub port: u16,
}

impl Database {
    pub fn connect_options(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .database(&self.database)
            .host(&self.host)
            .username(&self.username)
            .password(&self.password)
            .port(self.port)
    }
}
