pub mod messages;
pub mod socket;

use crate::{
    api::v1::auth::check_access_token, error::Error, structs::{Channel, Member}, utils::{get_auth_header, global_checks}, Data
};
use actix_web::{HttpRequest, HttpResponse, delete, get, web};
use uuid::Uuid;

#[get("/{uuid}")]
pub async fn get(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let channel_uuid = path.into_inner().0;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    let channel = Channel::fetch_one(&data, channel_uuid).await?;

    Member::fetch_one(&mut conn, uuid, channel.guild_uuid).await?;

    Ok(HttpResponse::Ok().json(channel))
}

#[delete("/{uuid}")]
pub async fn delete(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let channel_uuid = path.into_inner().0;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    let channel = Channel::fetch_one(&data, channel_uuid).await?;

    Member::fetch_one(&mut conn, uuid, channel.guild_uuid).await?;

    channel.delete(&data).await?;

    Ok(HttpResponse::Ok().finish())
}
