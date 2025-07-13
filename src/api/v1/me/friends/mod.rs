use actix_web::{HttpRequest, HttpResponse, get, post, web};
use serde::Deserialize;

pub mod uuid;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    objects::Me,
    utils::{get_auth_header, global_checks, user_uuid_from_identifier},
};

/// Returns a list of users that are your friends
#[get("/friends")]
pub async fn get(req: HttpRequest, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    let me = Me::get(&mut conn, uuid).await?;

    let friends = me.get_friends(&data).await?;

    Ok(HttpResponse::Ok().json(friends))
}

#[derive(Deserialize)]
struct UserReq {
    username: String,
}

/// `POST /api/v1/me/friends` Send friend request
///
/// requires auth? yes
///
/// ### Request Example:
/// ```
/// json!({
///     "uuid": "155d2291-fb23-46bd-a656-ae7c5d8218e6",
/// });
/// ```
/// NOTE: UUIDs in this response are made using `uuidgen`, UUIDs made by the actual backend will be UUIDv7 and have extractable timestamps
///
/// ### Responses
/// 200 Success
///
/// 404 Not Found
///
/// 400 Bad Request (usually means users are already friends)
///
#[post("/friends")]
pub async fn post(
    req: HttpRequest,
    json: web::Json<UserReq>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    let me = Me::get(&mut conn, uuid).await?;

    let target_uuid = user_uuid_from_identifier(&mut conn, &json.username).await?;
    me.add_friend(&mut conn, target_uuid).await?;

    Ok(HttpResponse::Ok().finish())
}
