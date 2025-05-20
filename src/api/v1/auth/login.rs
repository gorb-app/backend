use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{Error, HttpResponse, post, web};
use argon2::{PasswordHash, PasswordVerifier};
use log::error;
use serde::Deserialize;

use crate::{
    Data,
    api::v1::auth::{EMAIL_REGEX, PASSWORD_REGEX, USERNAME_REGEX},
    utils::{generate_access_token, generate_refresh_token, refresh_token_cookie},
};

use super::Response;

#[derive(Deserialize)]
struct LoginInformation {
    username: String,
    password: String,
    device_name: String,
}

#[post("/login")]
pub async fn response(
    login_information: web::Json<LoginInformation>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    if !PASSWORD_REGEX.is_match(&login_information.password) {
        return Ok(HttpResponse::Forbidden().json(r#"{ "password_hashed": false }"#));
    }

    if EMAIL_REGEX.is_match(&login_information.username) {
        let row =
            sqlx::query_as("SELECT CAST(uuid as VARCHAR), password FROM users WHERE email = $1")
                .bind(&login_information.username)
                .fetch_one(&data.pool)
                .await;

        if let Err(error) = row {
            if error.to_string()
                == "no rows returned by a query that expected to return at least one row"
            {
                return Ok(HttpResponse::Unauthorized().finish());
            }

            error!("{}", error);
            return Ok(HttpResponse::InternalServerError().json(
                r#"{ "error": "Unhandled exception occured, contact the server administrator" }"#,
            ));
        }

        let (uuid, password): (String, String) = row.unwrap();

        return Ok(login(
            data.clone(),
            uuid,
            login_information.password.clone(),
            password,
            login_information.device_name.clone(),
        )
        .await);
    } else if USERNAME_REGEX.is_match(&login_information.username) {
        let row =
            sqlx::query_as("SELECT CAST(uuid as VARCHAR), password FROM users WHERE username = $1")
                .bind(&login_information.username)
                .fetch_one(&data.pool)
                .await;

        if let Err(error) = row {
            if error.to_string()
                == "no rows returned by a query that expected to return at least one row"
            {
                return Ok(HttpResponse::Unauthorized().finish());
            }

            error!("{}", error);
            return Ok(HttpResponse::InternalServerError().json(
                r#"{ "error": "Unhandled exception occured, contact the server administrator" }"#,
            ));
        }

        let (uuid, password): (String, String) = row.unwrap();

        return Ok(login(
            data.clone(),
            uuid,
            login_information.password.clone(),
            password,
            login_information.device_name.clone(),
        )
        .await);
    }

    Ok(HttpResponse::Unauthorized().finish())
}

async fn login(
    data: actix_web::web::Data<Data>,
    uuid: String,
    request_password: String,
    database_password: String,
    device_name: String,
) -> HttpResponse {
    let parsed_hash_raw = PasswordHash::new(&database_password);

    if let Err(error) = parsed_hash_raw {
        error!("{}", error);
        return HttpResponse::InternalServerError().finish();
    }

    let parsed_hash = parsed_hash_raw.unwrap();

    if data
        .argon2
        .verify_password(request_password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return HttpResponse::Unauthorized().finish();
    }

    let refresh_token_raw = generate_refresh_token();
    let access_token_raw = generate_access_token();

    if let Err(error) = refresh_token_raw {
        error!("{}", error);
        return HttpResponse::InternalServerError().finish();
    }

    let refresh_token = refresh_token_raw.unwrap();

    if let Err(error) = access_token_raw {
        error!("{}", error);
        return HttpResponse::InternalServerError().finish();
    }

    let access_token = access_token_raw.unwrap();

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    if let Err(error) = sqlx::query(&format!(
        "INSERT INTO refresh_tokens (token, uuid, created_at, device_name) VALUES ($1, '{}', $2, $3 )",
        uuid
    ))
    .bind(&refresh_token)
    .bind(current_time)
    .bind(device_name)
    .execute(&data.pool)
    .await
    {
        error!("{}", error);
        return HttpResponse::InternalServerError().finish();
    }

    if let Err(error) = sqlx::query(&format!(
        "INSERT INTO access_tokens (token, refresh_token, uuid, created_at) VALUES ($1, $2, '{}', $3 )",
        uuid
    ))
    .bind(&access_token)
    .bind(&refresh_token)
    .bind(current_time)
    .execute(&data.pool)
    .await
    {
        error!("{}", error);
        return HttpResponse::InternalServerError().finish()
    }

    HttpResponse::Ok()
        .cookie(refresh_token_cookie(refresh_token))
        .json(Response { access_token })
}
