use actix_web::{cookie::{time::Duration, Cookie, SameSite}, http::header::HeaderMap, HttpResponse};
use getrandom::fill;
use hex::encode;
use redis::RedisError;
use serde::Serialize;

use crate::Data;

pub fn get_auth_header(headers: &HeaderMap) -> Result<&str, HttpResponse> {
    let auth_token = headers.get(actix_web::http::header::AUTHORIZATION);

    if let None = auth_token {
        return Err(HttpResponse::Unauthorized().finish());
    }

    let auth = auth_token.unwrap().to_str();

    if let Err(error) = auth {
        return Err(HttpResponse::Unauthorized().json(format!(r#" {{ "error": "{}" }} "#, error)));
    }

    let auth_value = auth.unwrap().split_whitespace().nth(1);

    if let None = auth_value {
        return Err(HttpResponse::BadRequest().finish());
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

impl Data {
    pub async fn set_cache_key(&self, key: String, value: impl Serialize, expire: u32) -> Result<(), RedisError> {
        let mut conn = self.cache_pool.get_multiplexed_tokio_connection().await?;

        let key_encoded = encode(key);

        let value_json = serde_json::to_string(&value).unwrap();

        redis::cmd("SET",).arg(&[key_encoded.clone(), value_json]).exec_async(&mut conn).await?;

        redis::cmd("EXPIRE").arg(&[key_encoded, expire.to_string()]).exec_async(&mut conn).await
    }

    pub async fn get_cache_key(&self, key: String) -> Result<String, RedisError> {
        let mut conn = self.cache_pool.get_multiplexed_tokio_connection().await?;

        let key_encoded = encode(key);

        redis::cmd("GET").arg(key_encoded).query_async(&mut conn).await
    }

    pub async fn del_cache_key(&self, key: String) -> Result<(), RedisError> {
        let mut conn = self.cache_pool.get_multiplexed_tokio_connection().await?;

        let key_encoded = encode(key);

        redis::cmd("DEL").arg(key_encoded).query_async(&mut conn).await
    }
}

