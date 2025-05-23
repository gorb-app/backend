use actix_web::{HttpRequest, HttpResponse, post, web};
use argon2::{PasswordHash, PasswordVerifier};
use diesel::{delete, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::{api::v1::auth::check_access_token, error::Error, schema::users::dsl as udsl, schema::refresh_tokens::{self, dsl as rdsl}, utils::get_auth_header, Data};

#[derive(Deserialize)]
struct RevokeRequest {
    password: String,
    device_name: String,
}

// TODO: Should maybe be a delete request?
#[post("/revoke")]
pub async fn res(
    req: HttpRequest,
    revoke_request: web::Json<RevokeRequest>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    let database_password: String = udsl::users
        .filter(udsl::uuid.eq(uuid))
        .select(udsl::password)
        .get_result(&mut conn)
        .await?;

    let hashed_password = PasswordHash::new(&database_password).map_err(|e| Error::PasswordHashError(e.to_string()))?;

    if data
        .argon2
        .verify_password(revoke_request.password.as_bytes(), &hashed_password)
        .is_err()
    {
        return Err(Error::Unauthorized("Wrong username or password".to_string()));
    }

    delete(refresh_tokens::table)
        .filter(rdsl::uuid.eq(uuid))
        .filter(rdsl::device_name.eq(&revoke_request.device_name))
        .execute(&mut conn)
        .await?;

    Ok(HttpResponse::Ok().finish())
}
