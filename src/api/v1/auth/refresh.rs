use actix_web::{HttpRequest, HttpResponse, post, web};
use diesel::{ExpressionMethods, QueryDsl, delete, update};
use diesel_async::RunQueryDsl;
use log::error;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    Data,
    error::Error,
    schema::{
        access_tokens::{self, dsl},
        refresh_tokens::{self, dsl as rdsl},
    },
    utils::{generate_access_token, generate_refresh_token, refresh_token_cookie},
};

use super::Response;

#[post("/refresh")]
pub async fn res(req: HttpRequest, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let recv_refresh_token_cookie = req.cookie("refresh_token");

    if recv_refresh_token_cookie.is_none() {
        return Ok(HttpResponse::Unauthorized().finish());
    }

    let mut refresh_token = String::from(recv_refresh_token_cookie.unwrap().value());

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
                error!("{}", error);
            }

            let mut refresh_token_cookie = refresh_token_cookie(refresh_token);

            refresh_token_cookie.make_removal();

            return Ok(HttpResponse::Unauthorized()
                .cookie(refresh_token_cookie)
                .finish());
        }

        let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        if lifetime > 1987200 {
            let new_refresh_token = generate_refresh_token()?;

            match update(refresh_tokens::table)
                .filter(rdsl::token.eq(&refresh_token))
                .set((
                    rdsl::token.eq(&new_refresh_token),
                    rdsl::created_at.eq(current_time),
                ))
                .execute(&mut conn)
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

        let access_token = generate_access_token()?;

        update(access_tokens::table)
            .filter(dsl::refresh_token.eq(&refresh_token))
            .set((
                dsl::token.eq(&access_token),
                dsl::created_at.eq(current_time),
            ))
            .execute(&mut conn)
            .await?;

        return Ok(HttpResponse::Ok()
            .cookie(refresh_token_cookie(refresh_token))
            .json(Response { access_token }));
    }

    let mut refresh_token_cookie = refresh_token_cookie(refresh_token);

    refresh_token_cookie.make_removal();

    Ok(HttpResponse::Unauthorized()
        .cookie(refresh_token_cookie)
        .finish())
}
