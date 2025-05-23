use actix_web::{HttpRequest, HttpResponse, get, post, web};

use crate::{
    error::Error,
    Data,
    api::v1::auth::check_access_token,
    structs::{Guild, Invite, Member},
    utils::get_auth_header,
};

#[get("{id}")]
pub async fn get(
    req: HttpRequest,
    path: web::Path<(String,)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    check_access_token(auth_header, &mut conn).await?;

    let invite_id = path.into_inner().0;

    let invite = Invite::fetch_one(&mut conn, invite_id).await?;

    let guild = Guild::fetch_one(&mut conn, invite.guild_uuid).await?;

    Ok(HttpResponse::Ok().json(guild))
}

#[post("{id}")]
pub async fn join(
    req: HttpRequest,
    path: web::Path<(String,)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let invite_id = path.into_inner().0;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    let invite = Invite::fetch_one(&mut conn, invite_id).await?;

    let guild = Guild::fetch_one(&mut conn, invite.guild_uuid).await?;

    Member::new(&mut conn, uuid, guild.uuid).await?;

    Ok(HttpResponse::Ok().json(guild))
}
