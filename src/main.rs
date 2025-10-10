use argon2::Argon2;
use axum::{
    Router,
    http::{Method, header},
};
use clap::Parser;
use config::{Config, ConfigBuilder};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use error::Error;
use objects::MailClient;
use std::time::SystemTime;
use tower_http::cors::{AllowOrigin, CorsLayer};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

type Conn =
    deadpool::managed::Object<AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>>;

mod api;
mod config;
pub mod error;
pub mod objects;
pub mod schema;
//mod socket;
pub mod utils;
mod wordlist;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("/etc/gorb/config.toml"))]
    config: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: deadpool::managed::Pool<
        AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>,
        Conn,
    >,
    pub cache_pool: redis::Client,
    pub config: Config,
    pub argon2: Argon2<'static>,
    pub start_time: SystemTime,
    pub bunny_storage: bunny_api_tokio::EdgeStorageClient,
    pub mail_client: MailClient,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let config = ConfigBuilder::load(args.config).await?.build();

    let web = config.web.clone();

    // create a new connection pool with the default config
    let pool_config =
        AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(config.database.url());
    let pool = Pool::builder(pool_config).build()?;

    let cache_pool = redis::Client::open(config.cache_database.url())?;

    let bunny = config.bunny.clone();

    let bunny_storage =
        bunny_api_tokio::EdgeStorageClient::new(bunny.api_key, bunny.endpoint, bunny.storage_zone)
            .await?;

    let mail = config.mail.clone();

    let mail_client = MailClient::new(
        mail.smtp.credentials(),
        mail.smtp.server,
        mail.address,
        mail.tls,
    )?;

    let database_url = config.database.url();

    tokio::task::spawn_blocking(move || {
        use diesel::prelude::Connection;
        use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;

        let mut conn =
            AsyncConnectionWrapper::<diesel_async::AsyncPgConnection>::establish(&database_url)?;

        conn.run_pending_migrations(MIGRATIONS)?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    })
    .await?
    .unwrap();

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

    let app_state = Box::leak(Box::new(AppState {
        pool,
        cache_pool,
        config,
        // TODO: Possibly implement "pepper" into this (thinking it could generate one if it doesnt exist and store it on disk)
        argon2: Argon2::default(),
        start_time: SystemTime::now(),
        bunny_storage,
        mail_client,
    }));

    let cors = CorsLayer::new()
        // Allow any origin (equivalent to allowed_origin_fn returning true)
        .allow_origin(AllowOrigin::predicate(|_origin, _request_head| true))
        .allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::HEAD,
            Method::OPTIONS,
            Method::CONNECT,
            Method::PATCH,
            Method::TRACE,
        ])
        .allow_headers(vec![
            header::ACCEPT,
            header::ACCEPT_LANGUAGE,
            header::AUTHORIZATION,
            header::CONTENT_LANGUAGE,
            header::CONTENT_TYPE,
            header::ORIGIN,
            header::ACCEPT,
            header::COOKIE,
            "x-requested-with".parse().unwrap(),
        ])
        // Allow credentials
        .allow_credentials(true);

    /*let (socket_io, io) = SocketIo::builder()
        .with_state(app_state.clone())
        .build_layer();

    io.ns("/", socket::on_connect);
    */
    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .merge(api::router(
            web.backend_url.path().trim_end_matches("/"),
            app_state,
        ))
        .with_state(app_state)
        //.layer(socket_io)
        .layer(cors);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(web.ip + ":" + &web.port.to_string()).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
