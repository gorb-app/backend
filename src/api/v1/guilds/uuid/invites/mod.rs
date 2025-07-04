use actix_web::{HttpRequest, HttpResponse, get, post, web};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    objects::{Guild, Member, Permissions},
    utils::{get_auth_header, global_checks},
};

#[derive(Deserialize)]
struct InviteRequest {
    custom_id: Option<String>,
}

#[get("{uuid}/invites")]
pub async fn get(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let guild_uuid = path.into_inner().0;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    let guild = Guild::fetch_one(&mut conn, guild_uuid).await?;

    let invites = guild.get_invites(&mut conn).await?;

    Ok(HttpResponse::Ok().json(invites))
}

#[post("{uuid}/invites")]
pub async fn create(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    invite_request: web::Json<InviteRequest>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let guild_uuid = path.into_inner().0;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    let member = Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    member
        .check_permission(&data, Permissions::CreateInvite)
        .await?;

    let guild = Guild::fetch_one(&mut conn, guild_uuid).await?;

    let invite = guild
        .create_invite(&mut conn, uuid, invite_request.custom_id.clone())
        .await?;

    Ok(HttpResponse::Ok().json(invite))
}
