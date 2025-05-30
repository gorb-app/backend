use actix_web::{web, Scope};

mod uuid;

pub fn web() -> Scope {
    web::scope("/channels")
        .service(uuid::get)
        .service(uuid::delete)
        .service(uuid::messages::get)
        .service(uuid::socket::ws)
}
