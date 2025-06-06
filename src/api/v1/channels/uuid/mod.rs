//! `/api/v1/channels/{uuid}` Channel specific endpoints

pub mod messages;
pub mod socket;

use crate::{
    api::v1::auth::check_access_token, error::Error, objects::{Channel, Member, Permissions}, utils::{get_auth_header, global_checks}, Data
};
use actix_web::{HttpRequest, HttpResponse, delete, get, patch, web};
use serde::Deserialize;
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

    Member::check_membership(&mut conn, uuid, channel.guild_uuid).await?;

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

    let member = Member::check_membership(&mut conn, uuid, channel.guild_uuid).await?;

    member.check_permission(&data, Permissions::DeleteChannel).await?;

    channel.delete(&data).await?;

    Ok(HttpResponse::Ok().finish())
}

#[derive(Deserialize)]
struct NewInfo {
    name: Option<String>,
    description: Option<String>,
    is_above: Option<String>,
}

/// `PATCH /api/v1/channels/{uuid}` Returns user with the given UUID
///
/// requires auth: yes
///
/// requires relation: yes
///
/// ### Request Example
/// All fields are optional and can be nulled/dropped if only changing 1 value
/// ```
/// json!({
///     "name": "gaming-chat",
///     "description": "Gaming related topics.",
///     "is_above": "398f6d7b-752c-4348-9771-fe6024adbfb1"
/// });
/// ```
///
/// ### Response Example
/// ```
/// json!({
///     uuid: "cdcac171-5add-4f88-9559-3a247c8bba2c",
///     guild_uuid: "383d2afa-082f-4dd3-9050-ca6ed91487b6",
///     name: "gaming-chat",
///     description: "Gaming related topics.",
///     is_above: "398f6d7b-752c-4348-9771-fe6024adbfb1",
///     permissions: {
///         role_uuid: "79cc0806-0f37-4a06-a468-6639c4311a2d",
///         permissions: 0
///     }
/// });
/// ```
/// NOTE: UUIDs in this response are made using `uuidgen`, UUIDs made by the actual backend will be UUIDv7 and have extractable timestamps
#[patch("/{uuid}")]
pub async fn patch(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    new_info: web::Json<NewInfo>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let channel_uuid = path.into_inner().0;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    let mut channel = Channel::fetch_one(&data, channel_uuid).await?;

    let member = Member::check_membership(&mut conn, uuid, channel.guild_uuid).await?;

    member.check_permission(&data, Permissions::ManageChannel).await?;

    if let Some(new_name) = &new_info.name {
        channel.set_name(&data, new_name.to_string()).await?;
    }

    if let Some(new_description) = &new_info.description {
        channel
            .set_description(&data, new_description.to_string())
            .await?;
    }

    if let Some(new_is_above) = &new_info.is_above {
        channel
            .set_description(&data, new_is_above.to_string())
            .await?;
    }

    Ok(HttpResponse::Ok().json(channel))
}
