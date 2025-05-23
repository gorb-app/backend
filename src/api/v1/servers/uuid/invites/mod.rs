use actix_web::{HttpRequest, HttpResponse, get, post, web};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::Error,
    Data,
    api::v1::auth::check_access_token,
    structs::{Guild, Member},
    utils::get_auth_header,
};

#[derive(Deserialize)]
struct InviteRequest {
    custom_id: String,
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

    Member::fetch_one(&mut conn, uuid, guild_uuid).await?;

    let guild = Guild::fetch_one(&mut conn, guild_uuid).await?;

    let invites = guild.get_invites(&mut conn).await?;

    Ok(HttpResponse::Ok().json(invites))
}

#[post("{uuid}/invites")]
pub async fn create(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    invite_request: web::Json<Option<InviteRequest>>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let guild_uuid = path.into_inner().0;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    let member = Member::fetch_one(&mut conn, uuid, guild_uuid).await?;

    let guild = Guild::fetch_one(&mut conn, guild_uuid).await?;

    let custom_id = invite_request.as_ref().map(|ir| ir.custom_id.clone());

    let invite = guild.create_invite(&mut conn, &member, custom_id).await?;

    Ok(HttpResponse::Ok().json(invite))
}
