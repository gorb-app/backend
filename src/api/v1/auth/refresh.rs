use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::{error, post, web, Error, HttpResponse};
use serde::{Deserialize, Serialize};
use futures::StreamExt;

use crate::{crypto::{generate_access_token, generate_refresh_token}, Data};

#[derive(Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Serialize)]
struct Response {
    refresh_token: String,
    access_token: String,
}

const MAX_SIZE: usize = 262_144;

#[post("/refresh")]
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

    let refresh_request = serde_json::from_slice::<RefreshRequest>(&body)?;

    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

    if let Ok(row) = sqlx::query_as("SELECT CAST(uuid as VARCHAR), created FROM refresh_tokens WHERE token = $1").bind(&refresh_request.refresh_token).fetch_one(&data.pool).await {
        let (uuid, created): (String, i64) = row;

        if let Err(error) = sqlx::query("DELETE FROM access_tokens WHERE refresh_token = $1")
            .bind(&refresh_request.refresh_token)
            .execute(&data.pool)
            .await {
            eprintln!("{}", error);
        }
    
        let lifetime = current_time - created;
    
        if lifetime > 2592000 {
            if let Err(error) = sqlx::query("DELETE FROM refresh_tokens WHERE token = $1")
                .bind(&refresh_request.refresh_token)
                .execute(&data.pool)
                .await {
                eprintln!("{}", error);
            }
    
            return Ok(HttpResponse::Unauthorized().finish())
        }

        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

        let mut refresh_token = refresh_request.refresh_token;

        if lifetime > 1987200 {
            let new_refresh_token = generate_refresh_token();

            if new_refresh_token.is_err() {
                eprintln!("{}", new_refresh_token.unwrap_err());
                return Ok(HttpResponse::InternalServerError().finish())
            }

            let new_refresh_token = new_refresh_token.unwrap();

            match sqlx::query(&format!("UPDATE refresh_tokens SET token = $1, uuid = {}, created = $2 WHERE token = $3", uuid))
                .bind(&new_refresh_token)
                .bind(&current_time)
                .bind(&refresh_token)
                .execute(&data.pool)
                .await {
                Ok(_) => {
                    refresh_token = new_refresh_token;
                },
                Err(error) => {
                    eprintln!("{}", error);
                },
            }
        }

        let access_token = generate_access_token();

        if access_token.is_err() {
            eprintln!("{}", access_token.unwrap_err());
            return Ok(HttpResponse::InternalServerError().finish())
        }

        let access_token = access_token.unwrap();

        if let Err(error) = sqlx::query(&format!("INSERT INTO access_tokens (token, refresh_token, uuid, created) VALUES ($1, $2, '{}', $3 )", uuid))
            .bind(&access_token)
            .bind(&refresh_token)
            .bind(current_time)
            .execute(&data.pool)
            .await {
            eprintln!("{}", error);
            return Ok(HttpResponse::InternalServerError().finish())
        }
    
        return Ok(HttpResponse::Ok().json(Response {
            refresh_token,
            access_token
        }))
    }

    Ok(HttpResponse::Unauthorized().finish())
}
