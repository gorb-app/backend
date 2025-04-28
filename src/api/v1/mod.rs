use actix_web::{web, Scope};

mod stats;

pub fn web() -> Scope {
    web::scope("/v1")
        .service(stats::res)
}
