use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{HttpResponse, post, web};
use argon2::{
    PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use diesel::{ExpressionMethods, dsl::insert_into};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Response;
use crate::{
    error::Error, objects::Member, schema::{
        access_tokens::{self, dsl as adsl},
        refresh_tokens::{self, dsl as rdsl},
        users::{self, dsl as udsl},
    }, utils::{
        generate_device_name, generate_token, new_refresh_token_cookie, EMAIL_REGEX, PASSWORD_REGEX, USERNAME_REGEX
    }, Data
};

#[derive(Deserialize)]
struct AccountInformation {
    identifier: String,
    email: String,
    password: String,
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

#[post("/register")]
pub async fn res(
    account_information: web::Json<AccountInformation>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    if !data.config.instance.registration {
        return Err(Error::Forbidden(
            "registration is disabled on this instance".to_string(),
        ));
    }

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
        let mut conn = data.pool.get().await?;

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

        if let Some(initial_guild) = data.config.instance.initial_guild {
            Member::new(&data, uuid, initial_guild).await?;
        }

        return Ok(HttpResponse::Ok()
            .cookie(new_refresh_token_cookie(&data.config, refresh_token))
            .json(Response { access_token, device_name }));
    }

    Ok(HttpResponse::InternalServerError().finish())
}
