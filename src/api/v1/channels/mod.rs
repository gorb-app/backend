use actix_web::{Scope, web};

mod uuid;

pub fn web() -> Scope {
    web::scope("/channels")
        .service(uuid::get)
        .service(uuid::delete)
        .service(uuid::patch)
        .service(uuid::messages::get)
        .service(uuid::socket::ws)
}
