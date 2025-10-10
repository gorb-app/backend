use std::time::{SystemTime, UNIX_EPOCH};

use argon2::{
    PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    Json,
    extract::State,
    http::{HeaderValue, StatusCode},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, dsl::insert_into};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Response;
use crate::{
    AppState,
    error::Error,
    objects::Member,
    schema::{
        access_tokens::{self, dsl as adsl},
        refresh_tokens::{self, dsl as rdsl},
        users::{self, dsl as udsl},
    },
    utils::{
        EMAIL_REGEX, PASSWORD_REGEX, USERNAME_REGEX, generate_device_name, generate_token,
        new_refresh_token_cookie,
    },
};

#[derive(Deserialize)]
pub struct AccountInformation {
    identifier: String,
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct ResponseError {
    signups_enabled: bool,
    gorb_id_valid: bool,
    gorb_id_available: bool,
    email_valid: bool,
    email_available: bool,
    password_strength: bool,
}

impl Default for ResponseError {
    fn default() -> Self {
        Self {
            signups_enabled: true,
            gorb_id_valid: true,
            gorb_id_available: true,
            email_valid: true,
            email_available: true,
            password_strength: true,
        }
    }
}

pub async fn post(
    State(app_state): State<&'static AppState>,
    Json(account_information): Json<AccountInformation>,
) -> Result<impl IntoResponse, Error> {
    if !app_state.config.instance.registration {
        return Err(Error::Forbidden(
            "registration is disabled on this instance".to_string(),
        ));
    }

    let uuid = Uuid::now_v7();

    if !EMAIL_REGEX.is_match(&account_information.email) {
        return Ok((
            StatusCode::FORBIDDEN,
            Json(ResponseError {
                email_valid: false,
                ..Default::default()
            }),
        )
            .into_response());
    }

    if !USERNAME_REGEX.is_match(&account_information.identifier)
        || account_information.identifier.len() < 3
        || account_information.identifier.len() > 32
    {
        return Ok((
            StatusCode::FORBIDDEN,
            Json(ResponseError {
                gorb_id_valid: false,
                ..Default::default()
            }),
        )
            .into_response());
    }

    if !PASSWORD_REGEX.is_match(&account_information.password) {
        return Ok((
            StatusCode::FORBIDDEN,
            Json(ResponseError {
                password_strength: false,
                ..Default::default()
            }),
        )
            .into_response());
    }

    let salt = SaltString::generate(&mut OsRng);

    if let Ok(hashed_password) = app_state
        .argon2
        .hash_password(account_information.password.as_bytes(), &salt)
    {
        let mut conn = app_state.pool.get().await?;

        // TODO: Check security of this implementation
        insert_into(users::table)
            .values((
                udsl::uuid.eq(uuid),
                udsl::username.eq(&account_information.identifier),
                udsl::password.eq(hashed_password.to_string()),
                udsl::email.eq(&account_information.email),
            ))
            .execute(&mut conn)
            .await?;

        let refresh_token = generate_token::<32>()?;
        let access_token = generate_token::<16>()?;

        let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        let device_name = generate_device_name();

        insert_into(refresh_tokens::table)
            .values((
                rdsl::token.eq(&refresh_token),
                rdsl::uuid.eq(uuid),
                rdsl::created_at.eq(current_time),
                rdsl::device_name.eq(&device_name),
            ))
            .execute(&mut conn)
            .await?;

        insert_into(access_tokens::table)
            .values((
                adsl::token.eq(&access_token),
                adsl::refresh_token.eq(&refresh_token),
                adsl::uuid.eq(uuid),
                adsl::created_at.eq(current_time),
            ))
            .execute(&mut conn)
            .await?;

        if let Some(initial_guild) = app_state.config.instance.initial_guild {
            Member::new(&mut conn, &app_state.cache_pool, uuid, initial_guild).await?;
        }

        let mut response = (
            StatusCode::OK,
            Json(Response {
                access_token,
                device_name,
            }),
        )
            .into_response();

        response.headers_mut().append(
            "Set-Cookie",
            HeaderValue::from_str(
                &new_refresh_token_cookie(&app_state.config, refresh_token).to_string(),
            )?,
        );

        return Ok(response);
    }

    Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
