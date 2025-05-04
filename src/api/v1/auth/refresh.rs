use actix_web::{post, web, Error, HttpRequest, HttpResponse};
use log::error;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    crypto::{generate_access_token, generate_refresh_token}, utils::refresh_token_cookie, Data
};

use super::Response;

#[post("/refresh")]
pub async fn res(req: HttpRequest, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let recv_refresh_token_cookie = req.cookie("refresh_token");

    if let None = recv_refresh_token_cookie {
        return Ok(HttpResponse::Unauthorized().finish())
    }

    let mut refresh_token = String::from(recv_refresh_token_cookie.unwrap().value());

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    if let Ok(row) =
        sqlx::query_scalar("SELECT created_at FROM refresh_tokens WHERE token = $1")
            .bind(&refresh_token)
            .fetch_one(&data.pool)
            .await
    {
        let created_at: i64 = row;

        let lifetime = current_time - created_at;

        if lifetime > 2592000 {
            if let Err(error) = sqlx::query("DELETE FROM refresh_tokens WHERE token = $1")
                .bind(&refresh_token)
                .execute(&data.pool)
                .await
            {
                error!("{}", error);
            }

            return Ok(HttpResponse::Unauthorized().finish());
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        if lifetime > 1987200 {
            let new_refresh_token = generate_refresh_token();

            if new_refresh_token.is_err() {
                error!("{}", new_refresh_token.unwrap_err());
                return Ok(HttpResponse::InternalServerError().finish());
            }

            let new_refresh_token = new_refresh_token.unwrap();

            match sqlx::query("UPDATE refresh_tokens SET token = $1, created_at = $2 WHERE token = $3")
                .bind(&new_refresh_token)
                .bind(current_time)
                .bind(&refresh_token)
                .execute(&data.pool)
                .await
            {
                Ok(_) => {
                    refresh_token = new_refresh_token;
                }
                Err(error) => {
                    error!("{}", error);
                }
            }
        }

        let access_token = generate_access_token();

        if access_token.is_err() {
            error!("{}", access_token.unwrap_err());
            return Ok(HttpResponse::InternalServerError().finish());
        }

        let access_token = access_token.unwrap();

        if let Err(error) = sqlx::query("UPDATE access_tokens SET token = $1, created_at = $2 WHERE refresh_token = $3")
            .bind(&access_token)
            .bind(current_time)
            .bind(&refresh_token)
            .execute(&data.pool)
            .await {
            error!("{}", error);
            return Ok(HttpResponse::InternalServerError().finish())
        }

        return Ok(HttpResponse::Ok().cookie(refresh_token_cookie(refresh_token)).json(Response {
            access_token,
        }));
    }

    Ok(HttpResponse::Unauthorized().finish())
}
