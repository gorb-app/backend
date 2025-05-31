use crate::error::Error;
use bunny_api_tokio::edge_storage::Endpoint;
use lettre::transport::smtp::authentication::Credentials;
use log::debug;
use serde::Deserialize;
use tokio::fs::read_to_string;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct ConfigBuilder {
    database: Database,
    cache_database: CacheDatabase,
    web: WebBuilder,
    instance: Option<InstanceBuilder>,
    bunny: BunnyBuilder,
    mail: Mail,
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
    frontend_url: Url,
    backend_url: Option<Url>,
    _ssl: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct InstanceBuilder {
    name: Option<String>,
    registration: Option<bool>,
    require_email_verification: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct BunnyBuilder {
    api_key: String,
    endpoint: String,
    storage_zone: String,
    cdn_url: Url,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Mail {
    pub smtp: Smtp,
    pub address: String,
    pub tls: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Smtp {
    pub server: String,
    username: String,
    password: String,
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
            frontend_url: self.web.frontend_url.clone(),
            backend_url: self.web.backend_url.or_else(|| self.web.frontend_url.join("/api").ok()).unwrap(),
        };

        let endpoint = match &*self.bunny.endpoint {
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

        let bunny = Bunny {
            api_key: self.bunny.api_key,
            endpoint,
            storage_zone: self.bunny.storage_zone,
            cdn_url: self.bunny.cdn_url,
        };

        let instance = match self.instance {
            Some(instance) => Instance {
                name: instance.name.unwrap_or("Gorb".to_string()),
                registration: instance.registration.unwrap_or(true),
                require_email_verification: instance.require_email_verification.unwrap_or(false),
            },
            None => Instance {
                name: "Gorb".to_string(),
                registration: true,
                require_email_verification: false,
            },
        };

        Config {
            database: self.database,
            cache_database: self.cache_database,
            web,
            instance,
            bunny,
            mail: self.mail,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database: Database,
    pub cache_database: CacheDatabase,
    pub web: Web,
    pub instance: Instance,
    pub bunny: Bunny,
    pub mail: Mail,
}

#[derive(Debug, Clone)]
pub struct Web {
    pub ip: String,
    pub port: u16,
    pub frontend_url: Url,
    pub backend_url: Url,
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub name: String,
    pub registration: bool,
    pub require_email_verification: bool,
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

impl Smtp {
    pub fn credentials(&self) -> Credentials {
        Credentials::new(self.username.clone(), self.password.clone())
    }
}
