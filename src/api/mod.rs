//! `/api` Contains the entire API

use actix_web::Scope;
use actix_web::web;

mod v1;
mod versions;

pub fn web(path: &str) -> Scope {
    web::scope(path.trim_end_matches('/'))
        .service(v1::web())
        .service(versions::get)
}
