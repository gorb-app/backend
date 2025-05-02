use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{Error, HttpResponse, error, post, web};
use argon2::{
    PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use futures::StreamExt;
use log::error;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::login::Response;
use crate::{
    Data,
    api::v1::auth::{EMAIL_REGEX, PASSWORD_REGEX, USERNAME_REGEX},
    crypto::{generate_access_token, generate_refresh_token},
};

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

    if !EMAIL_REGEX.is_match(&account_information.email) {
        return Ok(HttpResponse::Forbidden().json(ResponseError {
            email_valid: false,
            ..Default::default()
        }));
    }

    if !USERNAME_REGEX.is_match(&account_information.identifier)
        || account_information.identifier.len() < 3
        || account_information.identifier.len() > 32
    {
        return Ok(HttpResponse::Forbidden().json(ResponseError {
            gorb_id_valid: false,
            ..Default::default()
        }));
    }

    if !PASSWORD_REGEX.is_match(&account_information.password) {
        return Ok(HttpResponse::Forbidden().json(ResponseError {
            password_hashed: false,
            ..Default::default()
        }));
    }

    let salt = SaltString::generate(&mut OsRng);

    if let Ok(hashed_password) = data
        .argon2
        .hash_password(account_information.password.as_bytes(), &salt)
    {
        // TODO: Check security of this implementation
        return Ok(
            match sqlx::query(&format!(
                "INSERT INTO users (uuid, username, password, email) VALUES ( '{}', $1, $2, $3 )",
                uuid
            ))
            .bind(account_information.identifier)
            // FIXME: Password has no security currently, either from a client or server perspective
            .bind(hashed_password.to_string())
            .bind(account_information.email)
            .execute(&data.pool)
            .await
            {
                Ok(_out) => {
                    let refresh_token = generate_refresh_token();
                    let access_token = generate_access_token();

                    if refresh_token.is_err() {
                        error!("{}", refresh_token.unwrap_err());
                        return Ok(HttpResponse::InternalServerError().finish());
                    }

                    let refresh_token = refresh_token.unwrap();

                    if access_token.is_err() {
                        error!("{}", access_token.unwrap_err());
                        return Ok(HttpResponse::InternalServerError().finish());
                    }

                    let access_token = access_token.unwrap();

                    let current_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64;

                    if let Err(error) = sqlx::query(&format!("INSERT INTO refresh_tokens (token, uuid, created, device_name) VALUES ($1, '{}', $2, $3 )", uuid))
                        .bind(&refresh_token)
                        .bind(current_time)
                        .bind(account_information.device_name)
                        .execute(&data.pool)
                        .await {
                        error!("{}", error);
                        return Ok(HttpResponse::InternalServerError().finish())
                    }

                    if let Err(error) = sqlx::query(&format!("INSERT INTO access_tokens (token, refresh_token, uuid, created) VALUES ($1, $2, '{}', $3 )", uuid))
                        .bind(&access_token)
                        .bind(&refresh_token)
                        .bind(current_time)
                        .execute(&data.pool)
                        .await {
                        error!("{}", error);
                        return Ok(HttpResponse::InternalServerError().finish())
                    }

                    HttpResponse::Ok().json(Response {
                        access_token,
                        refresh_token,
                    })
                }
                Err(error) => {
                    let err_msg = error.as_database_error().unwrap().message();

                    match err_msg {
                        err_msg
                            if err_msg.contains("unique") && err_msg.contains("username_key") =>
                        {
                            HttpResponse::Forbidden().json(ResponseError {
                                gorb_id_available: false,
                                ..Default::default()
                            })
                        }
                        err_msg if err_msg.contains("unique") && err_msg.contains("email_key") => {
                            HttpResponse::Forbidden().json(ResponseError {
                                email_available: false,
                                ..Default::default()
                            })
                        }
                        _ => {
                            error!("{}", err_msg);
                            HttpResponse::InternalServerError().finish()
                        }
                    }
                }
            },
        );
    }

    Ok(HttpResponse::InternalServerError().finish())
}
