use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use argon2::{PasswordHash, PasswordVerifier};
use axum::{
    Json,
    extract::State,
    http::{HeaderValue, StatusCode},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, QueryDsl, dsl::insert_into};
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::{
    AppState,
    error::Error,
    schema::*,
    utils::{
        PASSWORD_REGEX, generate_token, new_access_token_cookie, new_refresh_token_cookie,
        user_uuid_from_identifier,
    },
};

#[derive(Deserialize)]
pub struct LoginInformation {
    username: String,
    password: String,
    device_name: String,
}

pub async fn response(
    State(app_state): State<Arc<AppState>>,
    Json(login_information): Json<LoginInformation>,
) -> Result<impl IntoResponse, Error> {
    if !PASSWORD_REGEX.is_match(&login_information.password) {
        return Err(Error::BadRequest("Bad password".to_string()));
    }

    use users::dsl;

    let mut conn = app_state.pool.get().await?;

    let uuid = user_uuid_from_identifier(&mut conn, &login_information.username).await?;

    let database_password: String = dsl::users
        .filter(dsl::uuid.eq(uuid))
        .select(dsl::password)
        .get_result(&mut conn)
        .await?;

    let parsed_hash = PasswordHash::new(&database_password)
        .map_err(|e| Error::PasswordHashError(e.to_string()))?;

    if app_state
        .argon2
        .verify_password(login_information.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(Error::Unauthorized(
            "Wrong username or password".to_string(),
        ));
    }

    let refresh_token = generate_token::<32>()?;
    let access_token = generate_token::<16>()?;

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

    let mut response = StatusCode::OK.into_response();

    response.headers_mut().append(
        "Set-Cookie",
        HeaderValue::from_str(
            &new_refresh_token_cookie(&app_state.config, refresh_token).to_string(),
        )?,
    );

    response.headers_mut().append(
        "Set-Cookie",
        HeaderValue::from_str(
            &new_access_token_cookie(&app_state.config, access_token).to_string(),
        )?,
    );

    Ok(response)
}
