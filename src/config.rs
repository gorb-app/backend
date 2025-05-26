use crate::error::Error;
use bunny_api_tokio::edge_storage::Endpoint;
use log::debug;
use serde::Deserialize;
use tokio::fs::read_to_string;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct ConfigBuilder {
    database: Database,
    cache_database: CacheDatabase,
    web: WebBuilder,
    bunny: Option<BunnyBuilder>,
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
    ip: Option<String>,
    port: Option<u16>,
    url: Url,
    _ssl: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct BunnyBuilder {
    api_key: String,
    endpoint: String,
    storage_zone: String,
    cdn_url: Url,
}

impl ConfigBuilder {
    pub async fn load(path: String) -> Result<Self, Error> {
        debug!("loading config from: {}", path);
        let raw = read_to_string(path).await?;

        let config = toml::from_str(&raw)?;

        Ok(config)
    }

    pub fn build(self) -> Config {
        let web = Web {
                ip: self.web.ip.unwrap_or(String::from("0.0.0.0")),
                port: self.web.port.unwrap_or(8080),
                url: self.web.url
            };

        let bunny;

        if let Some(bunny_builder) = self.bunny {
            let endpoint = match &*bunny_builder.endpoint {
                "Frankfurt" => Endpoint::Frankfurt,
                "London" => Endpoint::London,
                "New York" => Endpoint::NewYork,
                "Los Angeles" => Endpoint::LosAngeles,
                "Singapore" => Endpoint::Singapore,
                "Stockholm" => Endpoint::Stockholm,
                "Sao Paulo" => Endpoint::SaoPaulo,
                "Johannesburg" => Endpoint::Johannesburg,
                "Sydney" => Endpoint::Sydney,
                url => Endpoint::Custom(url.to_string()),
            };

            bunny = Some(Bunny {
                api_key: bunny_builder.api_key,
                endpoint,
                storage_zone: bunny_builder.storage_zone,
                cdn_url: bunny_builder.cdn_url,
            });
        } else {
            bunny = None;
        }

        Config {
            database: self.database,
            cache_database: self.cache_database,
            web,
            bunny,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database: Database,
    pub cache_database: CacheDatabase,
    pub web: Web,
    pub bunny: Option<Bunny>,
}

#[derive(Debug, Clone)]
pub struct Web {
    pub ip: String,
    pub port: u16,
    pub url: Url,
}

#[derive(Debug, Clone)]
pub struct Bunny {
    pub api_key: String,
    pub endpoint: Endpoint,
    pub storage_zone: String,
    pub cdn_url: Url,
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
