//! `/api` Contains the entire API

use actix_web::Scope;
use actix_web::web;

mod v1;
mod versions;

pub fn web() -> Scope {
    web::scope("/api").service(v1::web()).service(versions::get)
}
