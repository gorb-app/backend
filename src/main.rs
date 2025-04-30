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
    sqlx::raw_sql(r#"
        CREATE TABLE IF NOT EXISTS users (
            uuid uuid PRIMARY KEY UNIQUE NOT NULL,
            username varchar(32) UNIQUE NOT NULL,
            display_name varchar(64),
            password varchar(512) NOT NULL,
            email varchar(100) UNIQUE NOT NULL,
            email_verified boolean NOT NULL DEFAULT FALSE
        );
        CREATE TABLE IF NOT EXISTS instance_permissions (
            uuid uuid REFERENCES users(uuid),
            administrator boolean NOT NULL DEFAULT FALSE
        )
    "#)
    .execute(&pool)
    .await?;

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
