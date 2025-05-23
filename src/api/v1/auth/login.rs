use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{HttpResponse, post, web};
use argon2::{PasswordHash, PasswordVerifier};
use diesel::{dsl::insert_into, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::Error, api::v1::auth::{EMAIL_REGEX, PASSWORD_REGEX, USERNAME_REGEX}, schema::*, utils::{generate_access_token, generate_refresh_token, refresh_token_cookie}, Data
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

    use users::dsl;

    let mut conn = data.pool.get().await?;

    if EMAIL_REGEX.is_match(&login_information.username) {
        // FIXME: error handling, right now i just want this to work
        let (uuid, password): (Uuid, String) = dsl::users
            .filter(dsl::email.eq(&login_information.username))
            .select((dsl::uuid, dsl::password))
            .get_result(&mut conn)
            .await?;

        return login(
            data.clone(),
            uuid,
            login_information.password.clone(),
            password,
            login_information.device_name.clone(),
        )
        .await;
    } else if USERNAME_REGEX.is_match(&login_information.username) {
        // FIXME: error handling, right now i just want this to work
        let (uuid, password): (Uuid, String) = dsl::users
            .filter(dsl::username.eq(&login_information.username))
            .select((dsl::uuid, dsl::password))
            .get_result(&mut conn)
            .await?;

        return login(
            data.clone(),
            uuid,
            login_information.password.clone(),
            password,
            login_information.device_name.clone(),
        )
        .await;
    }

    Ok(HttpResponse::Unauthorized().finish())
}

async fn login(
    data: actix_web::web::Data<Data>,
    uuid: Uuid,
    request_password: String,
    database_password: String,
    device_name: String,
) -> Result<HttpResponse, Error> {
    let mut conn = data.pool.get().await?;

    let parsed_hash = PasswordHash::new(&database_password).map_err(|e| Error::PasswordHashError(e.to_string()))?;

    if data
        .argon2
        .verify_password(request_password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(Error::Unauthorized("Wrong username or password".to_string()));
    }

    let refresh_token = generate_refresh_token()?;
    let access_token = generate_access_token()?;

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs() as i64;

    use refresh_tokens::dsl as rdsl;

    insert_into(refresh_tokens::table)
        .values((rdsl::token.eq(&refresh_token), rdsl::uuid.eq(uuid), rdsl::created_at.eq(current_time), rdsl::device_name.eq(device_name)))
        .execute(&mut conn)
        .await?;

    use access_tokens::dsl as adsl;

    insert_into(access_tokens::table)
        .values((adsl::token.eq(&access_token), adsl::refresh_token.eq(&refresh_token), adsl::uuid.eq(uuid), adsl::created_at.eq(current_time)))
        .execute(&mut conn)
        .await?;

    Ok(HttpResponse::Ok()
        .cookie(refresh_token_cookie(refresh_token))
        .json(Response { access_token }))
}
