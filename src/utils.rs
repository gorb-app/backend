use std::sync::LazyLock;
use rand::{seq::IndexedRandom};

use axum::body::Bytes;
use axum_extra::extract::cookie::{Cookie, SameSite};
use bindet::FileType;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use getrandom::fill;
use hex::encode;
use redis::RedisError;
use regex::Regex;
use serde::Serialize;
use time::Duration;
use uuid::Uuid;

use crate::{
    AppState, Conn,
    config::Config,
    error::Error,
    objects::{HasIsAbove, HasUuid},
    schema::users,
    wordlist::{ADJECTIVES, ANIMALS}
};

pub static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+(?:\.[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+)*@(?:[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?\.)+[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?").unwrap()
});

pub static USERNAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9_.-]+$").unwrap());

pub static CHANNEL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9_.-]+$").unwrap());

pub static PASSWORD_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[0-9a-f]{96}").unwrap());

pub fn new_refresh_token_cookie(config: &Config, refresh_token: String) -> Cookie {
    Cookie::build(("refresh_token", refresh_token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::None)
        .path(config.web.backend_url.path().to_string())
        .max_age(Duration::days(30))
        .build()
}

pub fn generate_token<const N: usize>() -> Result<String, getrandom::Error> {
    let mut buf = [0u8; N];
    fill(&mut buf)?;
    Ok(encode(buf))
}

pub fn image_check(icon: Bytes) -> Result<String, Error> {
    let buf = std::io::Cursor::new(icon);

    let detect = bindet::detect(buf).map_err(|e| e.kind());

    if let Ok(Some(file_type)) = detect {
        if file_type.likely_to_be == vec![FileType::Jpg] {
            return Ok(String::from("jpg"));
        } else if file_type.likely_to_be == vec![FileType::Png] {
            return Ok(String::from("png"));
        }
    }

    Err(Error::BadRequest(
        "Uploaded file is not an image".to_string(),
    ))
}

pub async fn user_uuid_from_identifier(
    conn: &mut Conn,
    identifier: &String,
) -> Result<Uuid, Error> {
    if EMAIL_REGEX.is_match(identifier) {
        use users::dsl;
        let user_uuid = dsl::users
            .filter(dsl::email.eq(identifier))
            .select(dsl::uuid)
            .get_result(conn)
            .await?;

        Ok(user_uuid)
    } else if USERNAME_REGEX.is_match(identifier) {
        use users::dsl;
        let user_uuid = dsl::users
            .filter(dsl::username.eq(identifier))
            .select(dsl::uuid)
            .get_result(conn)
            .await?;

        Ok(user_uuid)
    } else {
        Err(Error::BadRequest(
            "Please provide a valid username or email".to_string(),
        ))
    }
}

pub async fn user_uuid_from_username(conn: &mut Conn, username: &String) -> Result<Uuid, Error> {
    if USERNAME_REGEX.is_match(username) {
        use users::dsl;
        let user_uuid = dsl::users
            .filter(dsl::username.eq(username))
            .select(dsl::uuid)
            .get_result(conn)
            .await?;

        Ok(user_uuid)
    } else {
        Err(Error::BadRequest(
            "Please provide a valid username".to_string(),
        ))
    }
}

pub async fn global_checks(app_state: &AppState, user_uuid: Uuid) -> Result<(), Error> {
    if app_state.config.instance.require_email_verification {
        let mut conn = app_state.pool.get().await?;

        use users::dsl;
        let email_verified: bool = dsl::users
            .filter(dsl::uuid.eq(user_uuid))
            .select(dsl::email_verified)
            .get_result(&mut conn)
            .await?;

        if !email_verified {
            return Err(Error::Forbidden(
                "server requires email verification".to_string(),
            ));
        }
    }

    Ok(())
}

pub async fn order_by_is_above<T>(mut items: Vec<T>) -> Result<Vec<T>, Error>
where
    T: HasUuid + HasIsAbove,
{
    let mut ordered = Vec::new();

    // Find head
    let head_pos = items
        .iter()
        .position(|item| !items.iter().any(|i| i.is_above() == Some(item.uuid())));

    if let Some(pos) = head_pos {
        ordered.push(items.swap_remove(pos));

        while let Some(next_pos) = items
            .iter()
            .position(|item| Some(item.uuid()) == ordered.last().unwrap().is_above())
        {
            ordered.push(items.swap_remove(next_pos));
        }
    }

    Ok(ordered)
}

impl AppState {
    pub async fn set_cache_key(
        &self,
        key: String,
        value: impl Serialize,
        expire: u32,
    ) -> Result<(), Error> {
        let mut conn = self.cache_pool.get_multiplexed_tokio_connection().await?;

        let key_encoded = encode(key);

        let value_json = serde_json::to_string(&value)?;

        redis::cmd("SET")
            .arg(&[key_encoded.clone(), value_json])
            .exec_async(&mut conn)
            .await?;

        redis::cmd("EXPIRE")
            .arg(&[key_encoded, expire.to_string()])
            .exec_async(&mut conn)
            .await?;

        Ok(())
    }

    pub async fn get_cache_key(&self, key: String) -> Result<String, RedisError> {
        let mut conn = self.cache_pool.get_multiplexed_tokio_connection().await?;

        let key_encoded = encode(key);

        redis::cmd("GET")
            .arg(key_encoded)
            .query_async(&mut conn)
            .await
    }

    pub async fn del_cache_key(&self, key: String) -> Result<(), RedisError> {
        let mut conn = self.cache_pool.get_multiplexed_tokio_connection().await?;

        let key_encoded = encode(key);

        redis::cmd("DEL")
            .arg(key_encoded)
            .query_async(&mut conn)
            .await
    }
}

pub fn generate_device_name() -> String {
    let mut rng = rand::rng();

    let adjective = ADJECTIVES.choose(&mut rng).unwrap();
    let animal = ANIMALS.choose(&mut rng).unwrap();

    return [*adjective, *animal].join(" ")
}
