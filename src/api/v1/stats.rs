use std::time::SystemTime;

use actix_web::{HttpResponse, get, web};
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use serde::Serialize;

use crate::error::Error;
use crate::Data;
use crate::schema::users::dsl::{users, uuid};

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
struct Response {
    accounts: i64,
    uptime: u64,
    version: String,
    build_number: String,
}

#[get("/stats")]
pub async fn res(data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let accounts: i64 = users
        .select(uuid)
        .count()
        .get_result(&mut data.pool.get().await?)
        .await?;

    let response = Response {
        // TODO: Get number of accounts from db
        accounts,
        uptime: SystemTime::now()
            .duration_since(data.start_time)
            .expect("Seriously why dont you have time??")
            .as_secs(),
        version: String::from(VERSION.unwrap_or("UNKNOWN")),
        // TODO: Get build number from git hash or remove this from the spec
        build_number: String::from("how do i implement this?"),
    };

    Ok(HttpResponse::Ok().json(response))
}
