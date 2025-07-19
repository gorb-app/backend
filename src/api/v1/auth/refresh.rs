use axum::{
    extract::State,
    http::{HeaderValue, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use diesel::{ExpressionMethods, QueryDsl, delete, update};
use diesel_async::RunQueryDsl;
use log::error;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    AppState,
    error::Error,
    schema::{
        access_tokens::{self, dsl},
        refresh_tokens::{self, dsl as rdsl},
    },
    utils::{generate_token, new_access_token_cookie, new_refresh_token_cookie},
};

pub async fn post(
    State(app_state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<impl IntoResponse, Error> {
    let mut refresh_token_cookie = jar
        .get("refresh_token")
        .ok_or(Error::Unauthorized(
            "request has no refresh token".to_string(),
        ))?
        .to_owned();

    let access_token_cookie = jar.get("access_token");

    let refresh_token = String::from(refresh_token_cookie.value_trimmed());

    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    let mut conn = app_state.pool.get().await?;

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

            let mut response = StatusCode::UNAUTHORIZED.into_response();

            refresh_token_cookie.make_removal();
            response.headers_mut().append(
                "Set-Cookie",
                HeaderValue::from_str(&refresh_token_cookie.to_string())?,
            );

            if let Some(cookie) = access_token_cookie {
                let mut cookie = cookie.clone();
                cookie.make_removal();
                response
                    .headers_mut()
                    .append("Set-Cookie", HeaderValue::from_str(&cookie.to_string())?);
            }

            return Ok(response);
        }

        let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        let mut response = StatusCode::OK.into_response();

        if lifetime > 1987200 {
            let new_refresh_token = generate_token::<32>()?;

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
                    response.headers_mut().append(
                        "Set-Cookie",
                        HeaderValue::from_str(
                            &new_refresh_token_cookie(&app_state.config, new_refresh_token)
                                .to_string(),
                        )?,
                    );
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

        
        response.headers_mut().append(
            "Set-Cookie",
            HeaderValue::from_str(
                &new_access_token_cookie(access_token).to_string(),
            )?,
        );

        return Ok(response);
    }

    let mut response = StatusCode::UNAUTHORIZED.into_response();

    refresh_token_cookie.make_removal();
    response.headers_mut().append(
        "Set-Cookie",
        HeaderValue::from_str(&refresh_token_cookie.to_string())?,
    );

    if let Some(cookie) = access_token_cookie {
        let mut cookie = cookie.clone();
        cookie.make_removal();
        response
            .headers_mut()
            .append("Set-Cookie", HeaderValue::from_str(&cookie.to_string())?);
    }

    Ok(response)
}
