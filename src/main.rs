use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use argon2::Argon2;
use clap::Parser;
use simple_logger::SimpleLogger;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::RunQueryDsl;
use std::time::SystemTime;
mod config;
use config::{Config, ConfigBuilder};
mod api;

type Conn = deadpool::managed::Object<AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>>;

pub mod structs;
pub mod utils;
pub mod schema;

type Error = Box<dyn std::error::Error>;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("/etc/gorb/config.toml"))]
    config: String,
}

#[derive(Clone)]
pub struct Data {
    pub pool: deadpool::managed::Pool<AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>, Conn>,
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

    // create a new connection pool with the default config
    let pool_config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(config.database.url());
    let pool = Pool::builder(pool_config).build()?;

    let cache_pool = redis::Client::open(config.cache_database.url())?;

    let mut conn = pool.get().await?;


    /*
    **Stored for later possible use**

        CREATE TABLE IF NOT EXISTS emojis (
            uuid uuid PRIMARY KEY NOT NULL,
            name varchar(32) NOT NULL,
            guild_uuid uuid REFERENCES guilds(uuid) ON DELETE SET NULL,
            deleted boolean DEFAULT FALSE
        );
        CREATE TABLE IF NOT EXISTS message_reactions (
            message_uuid uuid NOT NULL REFERENCES messages(uuid),
            user_uuid uuid NOT NULL REFERENCES users(uuid),
            emoji_uuid uuid NOT NULL REFERENCES emojis(uuid),
            PRIMARY KEY (message_uuid, user_uuid, emoji_uuid)
        )
    */

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
            .allowed_origin_fn(|_origin, _req_head| true)
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
