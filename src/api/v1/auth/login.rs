use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{HttpResponse, post, web};
use argon2::{PasswordHash, PasswordVerifier};
use diesel::{ExpressionMethods, QueryDsl, dsl::insert_into};
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::{
    Data,
    error::Error,
    schema::*,
    utils::{
        PASSWORD_REGEX, generate_access_token, generate_refresh_token, new_refresh_token_cookie,
        user_uuid_from_identifier,
    },
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

    let uuid = user_uuid_from_identifier(&mut conn, &login_information.username).await?;

    let database_password: String = dsl::users
        .filter(dsl::uuid.eq(uuid))
        .select(dsl::password)
        .get_result(&mut conn)
        .await?;

    let parsed_hash = PasswordHash::new(&database_password)
        .map_err(|e| Error::PasswordHashError(e.to_string()))?;

    if data
        .argon2
        .verify_password(login_information.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(Error::Unauthorized(
            "Wrong username or password".to_string(),
        ));
    }

    let refresh_token = generate_refresh_token()?;
    let access_token = generate_access_token()?;

    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    use refresh_tokens::dsl as rdsl;

    insert_into(refresh_tokens::table)
        .values((
            rdsl::token.eq(&refresh_token),
            rdsl::uuid.eq(uuid),
            rdsl::created_at.eq(current_time),
            rdsl::device_name.eq(&login_information.device_name),
        ))
        .execute(&mut conn)
        .await?;

    use access_tokens::dsl as adsl;

    insert_into(access_tokens::table)
        .values((
            adsl::token.eq(&access_token),
            adsl::refresh_token.eq(&refresh_token),
            adsl::uuid.eq(uuid),
            adsl::created_at.eq(current_time),
        ))
        .execute(&mut conn)
        .await?;

    Ok(HttpResponse::Ok()
        .cookie(new_refresh_token_cookie(&data.config, refresh_token))
        .json(Response { access_token }))
}
