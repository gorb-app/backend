use std::time::SystemTime;

use actix_web::{HttpResponse, Responder, get, web};
use serde::Serialize;

use crate::Data;

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
struct Response {
    accounts: usize,
    uptime: u64,
    version: String,
    build_number: String,
}

#[get("/stats")]
pub async fn res(data: web::Data<Data>) -> impl Responder {
    let response = Response {
        // TODO: Get number of accounts from db
        accounts: 0,
        uptime: SystemTime::now()
            .duration_since(data.start_time)
            .expect("Seriously why dont you have time??")
            .as_secs(),
        version: String::from(VERSION.unwrap_or("UNKNOWN")),
        // TODO: Get build number from git hash or remove this from the spec
        build_number: String::from("how do i implement this?"),
    };

    HttpResponse::Ok().json(response)
}
