use std::{str::FromStr, time::{SystemTime, UNIX_EPOCH}};

use actix_web::{web, HttpResponse, Scope};
use sqlx::Postgres;
use uuid::Uuid;

mod register;
mod login;
mod refresh;
mod revoke;

pub fn web() -> Scope {
    web::scope("/auth")
        .service(register::res)
        .service(login::response)
        .service(refresh::res)
        .service(revoke::res)
}

pub async fn check_access_token<'a>(access_token: String, pool: &'a sqlx::Pool<Postgres>) -> Result<Uuid, HttpResponse> {
    match sqlx::query_as("SELECT CAST(uuid as VARCHAR), created FROM access_tokens WHERE token = $1")
        .bind(&access_token)
        .fetch_one(&*pool)
        .await {
        Ok(row) => {
            let (uuid, created): (String, i64) = row;

            let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        
            let lifetime = current_time - created;
        
            if lifetime > 3600 {
                return Err(HttpResponse::Unauthorized().finish())
            }
        
            Ok(Uuid::from_str(&uuid).unwrap())
        },
        Err(error) => {
            eprintln!("{}", error);
            Err(HttpResponse::InternalServerError().finish())
        }
    }
}
