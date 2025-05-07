use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use argon2::Argon2;
use clap::Parser;
use simple_logger::SimpleLogger;
use sqlx::{PgPool, Pool, Postgres};
use std::time::SystemTime;
mod config;
use config::{Config, ConfigBuilder};
mod api;

pub mod utils;

type Error = Box<dyn std::error::Error>;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("/etc/gorb/config.toml"))]
    config: String,
}

#[derive(Clone)]
struct Data {
    pub pool: Pool<Postgres>,
    pub cache_pool: redis::Client,
    pub _config: Config,
    pub argon2: Argon2<'static>,
    pub start_time: SystemTime,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .with_colors(true)
        .env()
        .init()
        .unwrap();
    let args = Args::parse();

    let config = ConfigBuilder::load(args.config).await?.build();

    let web = config.web.clone();

    let pool = PgPool::connect_with(config.database.connect_options()).await?;

    let cache_pool = redis::Client::open(config.cache_database.url())?;

    /*
    TODO: Figure out if a table should be used here and if not then what.
    Also figure out if these should be different types from what they currently are and if we should add more "constraints"
    */
    sqlx::raw_sql(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            uuid uuid PRIMARY KEY UNIQUE NOT NULL,
            username varchar(32) UNIQUE NOT NULL,
            display_name varchar(64) DEFAULT NULL,
            password varchar(512) NOT NULL,
            email varchar(100) UNIQUE NOT NULL,
            email_verified boolean NOT NULL DEFAULT FALSE
        );
        CREATE TABLE IF NOT EXISTS instance_permissions (
            uuid uuid NOT NULL REFERENCES users(uuid),
            administrator boolean NOT NULL DEFAULT FALSE
        );
        CREATE TABLE IF NOT EXISTS refresh_tokens (
            token varchar(64) PRIMARY KEY UNIQUE NOT NULL,
            uuid uuid NOT NULL REFERENCES users(uuid),
            created_at int8 NOT NULL,
            device_name varchar(16) NOT NULL
        );
        CREATE TABLE IF NOT EXISTS access_tokens (
            token varchar(32) PRIMARY KEY UNIQUE NOT NULL,
            refresh_token varchar(64) UNIQUE NOT NULL REFERENCES refresh_tokens(token) ON UPDATE CASCADE ON DELETE CASCADE,
            uuid uuid NOT NULL REFERENCES users(uuid),
            created_at int8 NOT NULL
        )
    "#,
    )
    .execute(&pool)
    .await?;

    let data = Data {
        pool,
        cache_pool,
        _config: config,
        // TODO: Possibly implement "pepper" into this (thinking it could generate one if it doesnt exist and store it on disk)
        argon2: Argon2::default(),
        start_time: SystemTime::now(),
    };

    
    HttpServer::new(move || {
        // Set CORS headers
        let cors = Cors::default()
            /*
                Set Allowed-Control-Allow-Origin header to whatever
                the request's Origin header is. Must be done like this
                rather than setting it to "*" due to CORS not allowing
                sending of credentials (cookies) with wildcard origin.
            */
            .allowed_origin_fn(|_origin, _req_head| {
                true
            })
            /*
                Allows any request method in CORS preflight requests.
                This will be restricted to only ones actually in use later.
            */
            .allow_any_method()
            /*
                Allows any header(s) in request in CORS preflight requests.
                This wll be restricted to only ones actually in use later.
            */
            .allow_any_header()
            /*
                Allows browser to include cookies in requests.
                This is needed for receiving the secure HttpOnly refresh_token cookie.
            */
            .supports_credentials();

        App::new()
            .app_data(web::Data::new(data.clone()))
            .wrap(cors)
            .service(api::web())
    })
    .bind((web.url, web.port))?
    .run()
    .await?;
    Ok(())
}
