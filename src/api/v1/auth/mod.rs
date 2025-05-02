use std::{
    str::FromStr,
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};

use actix_web::{HttpResponse, Scope, web};
use log::error;
use regex::Regex;
use sqlx::Postgres;
use uuid::Uuid;

mod login;
mod refresh;
mod register;
mod revoke;

static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+(?:\.[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+)*@(?:[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?\.)+[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?").unwrap()
});

// FIXME: This regex doesnt seem to be working
static USERNAME_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[a-zA-Z0-9.-_]").unwrap());

// Password is expected to be hashed using SHA3-384
static PASSWORD_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[0-9a-f]{96}").unwrap());

pub fn web() -> Scope {
    web::scope("/auth")
        .service(register::res)
        .service(login::response)
        .service(refresh::res)
        .service(revoke::res)
}

pub async fn check_access_token(
    access_token: String,
    pool: &sqlx::Pool<Postgres>,
) -> Result<Uuid, HttpResponse> {
    let row = sqlx::query_as(
        "SELECT CAST(uuid as VARCHAR), created FROM access_tokens WHERE token = $1",
    )
    .bind(&access_token)
    .fetch_one(pool)
    .await;

    if let Err(error) = row {
        if error.to_string() == "no rows returned by a query that expected to return at least one row" {
            return Err(HttpResponse::Unauthorized().finish())
        }

        error!("{}", error);
        return Err(HttpResponse::InternalServerError().json(r#"{ "error": "Unhandled exception occured, contact the server administrator" }"#))
    }

    let (uuid, created): (String, i64) = row.unwrap();

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let lifetime = current_time - created;

    if lifetime > 3600 {
        return Err(HttpResponse::Unauthorized().finish());
    }

    Ok(Uuid::from_str(&uuid).unwrap())
}
