use std::time::SystemTime;
use actix_web::{web, App, HttpServer};
mod config;
use config::{Config, ConfigBuilder};
mod api;

type Error = Box<dyn std::error::Error>;

#[derive(Clone)]
struct Data {
    pub config: Config,
    pub start_time: SystemTime,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = ConfigBuilder::load().await?.build();

    let web = config.web.clone();

    let data = Data {
        config,
        start_time: SystemTime::now(),
    };

    HttpServer::new(move || {
        let data = data.clone();

        App::new()
            .app_data(web::Data::new(data))
            .service(api::versions::res)
            .service(api::v1::web())
    })
    .bind((web.url, web.port))?
    .run()
    .await?;
    Ok(())
}
