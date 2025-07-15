use actix_web::{HttpRequest, HttpResponse, post, web};
use diesel::{ExpressionMethods, QueryDsl, delete, update};
use diesel_async::RunQueryDsl;
use log::error;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    error::Error, schema::{
        access_tokens::{self, dsl},
        refresh_tokens::{self, dsl as rdsl},
    }, utils::{generate_token, new_refresh_token_cookie}, Data
};

use super::Response;

#[post("/refresh")]
pub async fn res(req: HttpRequest, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let mut refresh_token_cookie = req.cookie("refresh_token").ok_or(Error::Unauthorized(
        "request has no refresh token".to_string(),
    ))?;

    let mut refresh_token = String::from(refresh_token_cookie.value());

    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    let mut conn = data.pool.get().await?;

    if let Ok(created_at) = rdsl::refresh_tokens
        .filter(rdsl::token.eq(&refresh_token))
        .select(rdsl::created_at)
        .get_result::<i64>(&mut conn)
        .await
    {
        let lifetime = current_time - created_at;

        if lifetime > 2592000 {
            if let Err(error) = delete(refresh_tokens::table)
                .filter(rdsl::token.eq(&refresh_token))
                .execute(&mut conn)
                .await
            {
                error!("{error}");
            }

            refresh_token_cookie.make_removal();

            return Ok(HttpResponse::Unauthorized()
                .cookie(refresh_token_cookie)
                .finish());
        }

        let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let mut device_name: String = String::new();

        if lifetime > 1987200 {
            let new_refresh_token = generate_token::<32>()?;

            match update(refresh_tokens::table)
                .filter(rdsl::token.eq(&refresh_token))
                .set((
                    rdsl::token.eq(&new_refresh_token),
                    rdsl::created_at.eq(current_time),
                ))
                .returning(rdsl::device_name)
                .get_result::<String>(&mut conn)
                .await
            {
                Ok(existing_device_name) => {
                    refresh_token = new_refresh_token;
                    device_name = existing_device_name;
                }
                Err(error) => {
                    error!("{error}");
                }
            }
        }

        let access_token = generate_token::<16>()?;

        update(access_tokens::table)
            .filter(dsl::refresh_token.eq(&refresh_token))
            .set((
                dsl::token.eq(&access_token),
                dsl::created_at.eq(current_time),
            ))
            .execute(&mut conn)
            .await?;

        return Ok(HttpResponse::Ok()
            .cookie(new_refresh_token_cookie(&data.config, refresh_token))
            .json(Response { access_token, device_name }));
    }

    refresh_token_cookie.make_removal();

    Ok(HttpResponse::Unauthorized()
        .cookie(refresh_token_cookie)
        .finish())
}
