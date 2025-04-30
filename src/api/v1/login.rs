use actix_web::{error, post, web, Error, HttpResponse};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use regex::Regex;
use serde::{Deserialize, Serialize};
use futures::StreamExt;

use crate::Data;

#[derive(Deserialize)]
struct LoginInformation {
    username: String,
    password: String,
    device_name: String,
}

#[derive(Serialize)]
struct Response {
    access_token: String,
    expires_in: u64,
    refresh_token: String,
}

const MAX_SIZE: usize = 262_144;

#[post("/login")]
pub async fn res(mut payload: web::Payload, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        body.extend_from_slice(&chunk);
    }

    let login_information = serde_json::from_slice::<LoginInformation>(&body)?;

    let email_regex = Regex::new(r"[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+(?:\.[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+)*@(?:[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?\.)+[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?").unwrap();

    // FIXME: This regex doesnt seem to be working
    let username_regex = Regex::new(r"[a-zA-Z0-9.-_]").unwrap();

    // Password is expected to be hashed using SHA3-384
    let password_regex = Regex::new(r"/[0-9a-f]{96}/i").unwrap();

    if !password_regex.is_match(&login_information.password) {
        return Ok(HttpResponse::Forbidden().json(r#"{ "password_hashed": false }"#));
    }

    if email_regex.is_match(&login_information.username) {
        if let Ok(password) = sqlx::query_scalar("SELECT password FROM users WHERE email = $1").bind(login_information.username).fetch_one(&data.pool).await {
            return Ok(login(data.argon2.clone(), login_information.password, password))
        }

        return Ok(HttpResponse::Unauthorized().finish())
    } else if username_regex.is_match(&login_information.username) {
        if let Ok(password) = sqlx::query_scalar("SELECT password FROM users WHERE username = $1").bind(login_information.username).fetch_one(&data.pool).await {
            return Ok(login(data.argon2.clone(), login_information.password, password))
        }

        return Ok(HttpResponse::Unauthorized().finish())
    }

    Ok(HttpResponse::Unauthorized().finish())
}

fn login(argon2: Argon2, request_password: String, database_password: String) -> HttpResponse {
    if let Ok(parsed_hash) = PasswordHash::new(&database_password) {
        if argon2.verify_password(request_password.as_bytes(), &parsed_hash).is_ok() {
            return HttpResponse::Ok().json(Response {
                access_token: "bogus".to_string(),
                expires_in: 0,
                refresh_token: "bogus".to_string(),
            })
        }

        return HttpResponse::Unauthorized().finish()
    }

    HttpResponse::InternalServerError().finish()
}
