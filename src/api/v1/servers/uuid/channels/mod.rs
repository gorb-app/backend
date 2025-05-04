use actix_web::{web, Scope};

mod uuid;

pub fn web() -> Scope {
    web::scope("/channels")
}