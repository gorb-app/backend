use std::sync::LazyLock;

use actix_web::{
    cookie::{Cookie, SameSite, time::Duration},
    http::header::HeaderMap,
    web::BytesMut,
};
use bindet::FileType;
use getrandom::fill;
use hex::encode;
use redis::RedisError;
use regex::Regex;
use serde::Serialize;

use crate::{
    Data,
    error::Error,
    structs::{HasIsAbove, HasUuid},
};

pub static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+(?:\.[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+)*@(?:[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?\.)+[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?").unwrap()
});

pub static USERNAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9_.-]+$").unwrap());

// Password is expected to be hashed using SHA3-384
pub static PASSWORD_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[0-9a-f]{96}").unwrap());

pub fn get_auth_header(headers: &HeaderMap) -> Result<&str, Error> {
    let auth_token = headers.get(actix_web::http::header::AUTHORIZATION);

    if auth_token.is_none() {
        return Err(Error::Unauthorized(
            "No authorization header provided".to_string(),
        ));
    }

    let auth_raw = auth_token.unwrap().to_str()?;

    let mut auth = auth_raw.split_whitespace();

    let auth_type = auth.next();

    let auth_value = auth.next();

    if auth_type.is_none() {
        return Err(Error::BadRequest(
            "Authorization header is empty".to_string(),
        ));
    } else if auth_type.is_some_and(|at| at != "Bearer") {
        return Err(Error::BadRequest(
            "Only token auth is supported".to_string(),
        ));
    }

    if auth_value.is_none() {
        return Err(Error::BadRequest("No token provided".to_string()));
    }

    Ok(auth_value.unwrap())
}

pub fn get_ws_protocol_header(headers: &HeaderMap) -> Result<&str, Error> {
    let auth_token = headers.get(actix_web::http::header::SEC_WEBSOCKET_PROTOCOL);

    if auth_token.is_none() {
        return Err(Error::Unauthorized(
            "No authorization header provided".to_string(),
        ));
    }

    let auth_raw = auth_token.unwrap().to_str()?;

    let mut auth = auth_raw.split_whitespace();

    let response_proto = auth.next();

    let auth_value = auth.next();

    if response_proto.is_none() {
        return Err(Error::BadRequest(
            "Sec-WebSocket-Protocol header is empty".to_string(),
        ));
    } else if response_proto.is_some_and(|rp| rp != "Authorization,") {
        return Err(Error::BadRequest(
            "First protocol should be Authorization".to_string(),
        ));
    }

    if auth_value.is_none() {
        return Err(Error::BadRequest("No token provided".to_string()));
    }

    Ok(auth_value.unwrap())
}

pub fn refresh_token_cookie(refresh_token: String) -> Cookie<'static> {
    Cookie::build("refresh_token", refresh_token)
        .http_only(true)
        .secure(true)
        .same_site(SameSite::None)
        .path("/api")
        .max_age(Duration::days(30))
        .finish()
}

pub fn generate_access_token() -> Result<String, getrandom::Error> {
    let mut buf = [0u8; 16];
    fill(&mut buf)?;
    Ok(encode(buf))
}

pub fn generate_refresh_token() -> Result<String, getrandom::Error> {
    let mut buf = [0u8; 32];
    fill(&mut buf)?;
    Ok(encode(buf))
}

pub fn image_check(icon: BytesMut) -> Result<String, Error> {
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

impl Data {
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
