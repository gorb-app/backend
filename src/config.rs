use crate::error::Error;
use log::debug;
use serde::Deserialize;
use tokio::fs::read_to_string;

#[derive(Debug, Deserialize)]
pub struct ConfigBuilder {
    database: Database,
    cache_database: CacheDatabase,
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

#[derive(Debug, Deserialize, Clone)]
pub struct CacheDatabase {
    username: Option<String>,
    password: Option<String>,
    host: String,
    database: Option<String>,
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
            cache_database: self.cache_database,
            web,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database: Database,
    pub cache_database: CacheDatabase,
    pub web: Web,
}

#[derive(Debug, Clone)]
pub struct Web {
    pub url: String,
    pub port: u16,
}

impl Database {
    pub fn url(&self) -> String {
        let mut url = String::from("postgres://");

        url += &self.username;

        url += ":";
        url += &self.password;

        url += "@";

        url += &self.host;
        url += ":";
        url += &self.port.to_string();

        url += "/";
        url += &self.database;

        url
    }
}

impl CacheDatabase {
    pub fn url(&self) -> String {
        let mut url = String::from("redis://");

        if let Some(username) = &self.username {
            url += username;
        }

        if let Some(password) = &self.password {
            url += ":";
            url += password;
        }

        if self.username.is_some() || self.password.is_some() {
            url += "@";
        }

        url += &self.host;
        url += ":";
        url += &self.port.to_string();

        if let Some(database) = &self.database {
            url += "/";
            url += database;
        }

        url
    }
}
