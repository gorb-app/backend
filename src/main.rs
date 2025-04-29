use actix_web::{App, HttpServer, web};
use sqlx::{Executor, PgPool, Pool, Postgres};
use std::time::SystemTime;
mod config;
use config::{Config, ConfigBuilder};
mod api;

type Error = Box<dyn std::error::Error>;

#[derive(Clone)]
struct Data {
    pub pool: Pool<Postgres>,
    pub config: Config,
    pub start_time: SystemTime,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = ConfigBuilder::load().await?.build();

    let web = config.web.clone();

    let pool = PgPool::connect_with(config.database.connect_options()).await?;

    /* 
    TODO: Figure out if a table should be used here and if not then what.
    Also figure out if these should be different types from what they currently are and if we should add more "constraints"
    */
    pool.execute(r#"CREATE TABLE IF NOT EXISTS users (
    uuid uuid UNIQUE NOT NULL,
    username varchar(32) UNIQUE NOT NULL,
    display_name varchar(64),
    password varchar(512) NOT NULL,
    email varchar(100) UNIQUE NOT NULL,
    email_verified integer NOT NULL DEFAULT '0',
    PRIMARY KEY (uuid)
    )"#).await?;

    let data = Data {
        pool,
        config,
        start_time: SystemTime::now(),
    };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(data.clone()))
            .service(api::versions::res)
            .service(api::v1::web())
    })
    .bind((web.url, web.port))?
    .run()
    .await?;
    Ok(())
}
