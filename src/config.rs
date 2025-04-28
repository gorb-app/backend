use serde::Deserialize;
use tokio::fs::read_to_string;
use crate::Error;
use url::Url;


#[derive(Debug, Deserialize)]
pub struct ConfigBuilder {
    database: Database,
    web: Option<WebBuilder>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
    username: String,
    password: String,
    hostname: String,
    port: u16
}

#[derive(Debug, Deserialize)]
struct WebBuilder {
    url: Option<String>,
    port: Option<u16>,
    ssl: Option<bool>,
}

impl ConfigBuilder {
    pub async fn load() -> Result<Self, Error> {
        let raw = read_to_string("./config.toml").await?;

        let config = toml::from_str(&raw)?;

        Ok(config)
    }

    pub fn build(self) -> Config {

        let web = if let Some(web) = self.web {
            Web {
                url: web.url.unwrap_or(String::from("0.0.0.0")),
                port: web.port.unwrap_or(8080),
                ssl: web.ssl.unwrap_or_default()
            }
        } else {
            Web {
                url: String::from("0.0.0.0"),
                port: 8080,
                ssl: false
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
    pub ssl: bool,
}
