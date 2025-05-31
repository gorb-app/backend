use actix_web::{HttpRequest, HttpResponse, post, web};
use diesel::{ExpressionMethods, delete};
use diesel_async::RunQueryDsl;

use crate::{
    Data,
    error::Error,
    schema::refresh_tokens::{self, dsl},
};

// TODO: Should maybe be a delete request?
#[post("/logout")]
pub async fn res(
    req: HttpRequest,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let mut refresh_token_cookie = req.cookie("refresh_token").ok_or(Error::Unauthorized("request has no refresh token".to_string()))?;

    let refresh_token = String::from(refresh_token_cookie.value());

    let mut conn = data.pool.get().await?;

    delete(refresh_tokens::table)
        .filter(dsl::token.eq(refresh_token))
        .execute(&mut conn)
        .await?;

    refresh_token_cookie.make_removal();

    Ok(HttpResponse::Ok().cookie(refresh_token_cookie).finish())
}
