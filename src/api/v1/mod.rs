use actix_web::{Scope, web};

mod stats;

pub fn web() -> Scope {
    web::scope("/v1").service(stats::res)
}
