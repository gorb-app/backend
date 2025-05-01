use actix_web::{Scope, web};

mod register;
mod login;
mod refresh;

pub fn web() -> Scope {
    web::scope("/auth")
        .service(register::res)
        .service(login::response)
        .service(refresh::res)
}
