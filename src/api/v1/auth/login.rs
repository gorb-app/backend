use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{error, post, web, Error, HttpResponse};
use argon2::{PasswordHash, PasswordVerifier};
use log::error;
use regex::Regex;
use serde::{Deserialize, Serialize};
use futures::StreamExt;

use crate::{crypto::{generate_access_token, generate_refresh_token}, Data};

#[derive(Deserialize)]
struct LoginInformation {
    username: String,
    password: String,
    device_name: String,
}

#[derive(Serialize)]
pub struct Response {
    pub access_token: String,
    pub refresh_token: String,
}

const MAX_SIZE: usize = 262_144;

#[post("/login")]
pub async fn response(mut payload: web::Payload, data: web::Data<Data>) -> Result<HttpResponse, Error> {
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
    let password_regex = Regex::new(r"[0-9a-f]{96}").unwrap();

    if !password_regex.is_match(&login_information.password) {
        return Ok(HttpResponse::Forbidden().json(r#"{ "password_hashed": false }"#));
    }

    if email_regex.is_match(&login_information.username) {
        if let Ok(row) = sqlx::query_as("SELECT CAST(uuid as VARCHAR), password FROM users WHERE email = $1").bind(login_information.username).fetch_one(&data.pool).await {
            let (uuid, password): (String, String) = row;
            return Ok(login(data.clone(), uuid, login_information.password, password, login_information.device_name).await)
        }

        return Ok(HttpResponse::Unauthorized().finish())
    } else if username_regex.is_match(&login_information.username) {
        if let Ok(row) = sqlx::query_as("SELECT CAST(uuid as VARCHAR), password FROM users WHERE username = $1").bind(login_information.username).fetch_one(&data.pool).await {
            let (uuid, password): (String, String) = row;
            return Ok(login(data.clone(), uuid, login_information.password, password, login_information.device_name).await)
        }

        return Ok(HttpResponse::Unauthorized().finish())
    }

    Ok(HttpResponse::Unauthorized().finish())
}

async fn login(data: actix_web::web::Data<Data>, uuid: String, request_password: String, database_password: String, device_name: String) -> HttpResponse {
    if let Ok(parsed_hash) = PasswordHash::new(&database_password) {
        if data.argon2.verify_password(request_password.as_bytes(), &parsed_hash).is_ok() {
            let refresh_token = generate_refresh_token();
            let access_token = generate_access_token();

            if refresh_token.is_err() {
                error!("{}", refresh_token.unwrap_err());
                return HttpResponse::InternalServerError().finish()
            }

            let refresh_token = refresh_token.unwrap();

            if access_token.is_err() {
                error!("{}", access_token.unwrap_err());
                return HttpResponse::InternalServerError().finish()
            }

            let access_token = access_token.unwrap();

            let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

            if let Err(error) = sqlx::query(&format!("INSERT INTO refresh_tokens (token, uuid, created, device_name) VALUES ($1, '{}', $2, $3 )", uuid))
                .bind(&refresh_token)
                .bind(current_time)
                .bind(device_name)
                .execute(&data.pool)
                .await {
                error!("{}", error);
                return HttpResponse::InternalServerError().finish()
            }

            if let Err(error) = sqlx::query(&format!("INSERT INTO access_tokens (token, refresh_token, uuid, created) VALUES ($1, $2, '{}', $3 )", uuid))
                .bind(&access_token)
                .bind(&refresh_token)
                .bind(current_time)
                .execute(&data.pool)
                .await {
                error!("{}", error);
                return HttpResponse::InternalServerError().finish()
            }

            return HttpResponse::Ok().json(Response {
                access_token,
                refresh_token,
            })
        }

        return HttpResponse::Unauthorized().finish()
    }

    HttpResponse::InternalServerError().finish()
}
