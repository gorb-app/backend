use actix_web::{error, post, web, Error, HttpResponse};
use regex::Regex;
use serde::{Deserialize, Serialize};
use futures::StreamExt;
use uuid::Uuid;
use argon2::{password_hash::{rand_core::OsRng, SaltString}, PasswordHasher};

use crate::Data;

#[derive(Deserialize)]
struct AccountInformation {
    identifier: String,
    email: String,
    password: String,
    device_name: String,
}

#[derive(Serialize)]
struct ResponseError {
    signups_enabled: bool,
    gorb_id_valid: bool,
    gorb_id_available: bool,
    email_valid: bool,
    email_available: bool,
    password_hashed: bool,
    password_minimum_length: bool,
    password_special_characters: bool,
    password_letters: bool,
    password_numbers: bool,
}

impl Default for ResponseError {
    fn default() -> Self {
        Self {
            signups_enabled: true,
            gorb_id_valid: true,
            gorb_id_available: true,
            email_valid: true,
            email_available: true,
            password_hashed: true,
            password_minimum_length: true,
            password_special_characters: true,
            password_letters: true,
            password_numbers: true,
        }
    }
}

#[derive(Serialize)]
struct Response {
    access_token: String,
    user_id: String,
    expires_in: u64,
    refresh_token: String,
}

const MAX_SIZE: usize = 262_144;

#[post("/register")]
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
    let account_information = serde_json::from_slice::<AccountInformation>(&body)?;

    let uuid = Uuid::now_v7();

    let email_regex = Regex::new(r"[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+(?:\.[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+)*@(?:[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?\.)+[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?").unwrap();

    if !email_regex.is_match(&account_information.email) {
        return Ok(HttpResponse::Forbidden().json(
            ResponseError {
                email_valid: false,
                ..Default::default()
            }
        ))
    }

    // FIXME: This regex doesnt seem to be working
    let username_regex = Regex::new(r"[a-zA-Z0-9.-_]").unwrap();

    if !username_regex.is_match(&account_information.identifier) || account_information.identifier.len() < 3 || account_information.identifier.len() > 32 {
        return Ok(HttpResponse::Forbidden().json(
            ResponseError {
                gorb_id_valid: false,
                ..Default::default()
            }
        ))
    }

    // Password is expected to be hashed using SHA3-384
    let password_regex = Regex::new(r"/[0-9a-f]{96}/i").unwrap();

    if !password_regex.is_match(&account_information.password) {
        return Ok(HttpResponse::Forbidden().json(
            ResponseError {
                password_hashed: false,
                ..Default::default()
            }
        ))
    }

    let salt = SaltString::generate(&mut OsRng);

    if let Ok(hashed_password) = data.argon2.hash_password(account_information.password.as_bytes(), &salt) {
        // TODO: Check security of this implementation
        return Ok(match sqlx::query(&format!("INSERT INTO users VALUES ( '{}', $1, NULL, $2, $3, false )", uuid))
            .bind(account_information.identifier)
            // FIXME: Password has no security currently, either from a client or server perspective
            .bind(hashed_password.to_string())
            .bind(account_information.email)
            .execute(&data.pool)
            .await {
                Ok(_out) => {
                    HttpResponse::Ok().json(
                        Response {
                            access_token: "bogus".to_string(),
                            user_id: "bogus".to_string(),
                            expires_in: 1,
                            refresh_token: "bogus".to_string(),
                        }
                    )
                },
                Err(error) => {
                    let err_msg = error.as_database_error().unwrap().message();
        
                    match err_msg {
                        err_msg if err_msg.contains("unique") && err_msg.contains("username_key") => HttpResponse::Forbidden().json(ResponseError {
                            gorb_id_available: false,
                            ..Default::default()
                        }),
                        err_msg if err_msg.contains("unique") && err_msg.contains("email_key") => HttpResponse::Forbidden().json(ResponseError {
                            email_available: false,
                            ..Default::default()
                        }),
                        _ => {
                            eprintln!("{}", err_msg);
                            HttpResponse::InternalServerError().finish()
                        }
                    }
                },
        })
    }

    Ok(HttpResponse::InternalServerError().finish())
}
