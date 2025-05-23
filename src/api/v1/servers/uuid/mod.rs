use actix_web::{HttpRequest, HttpResponse, Scope, get, web};
use uuid::Uuid;

mod channels;
mod invites;
mod roles;
mod icon;

use crate::{
    error::Error,
    Data,
    api::v1::auth::check_access_token,
    structs::{Guild, Member},
    utils::get_auth_header,
};

pub fn web() -> Scope {
    web::scope("")
        // Servers
        .service(res)
        // Channels
        .service(channels::get)
        .service(channels::create)
        .service(channels::uuid::get)
        .service(channels::uuid::delete)
        .service(channels::uuid::messages::get)
        .service(channels::uuid::socket::echo)
        // Roles
        .service(roles::get)
        .service(roles::create)
        .service(roles::uuid::get)
        // Invites
        .service(invites::get)
        .service(invites::create)
        // Icon
        .service(icon::upload)
}

#[get("/{uuid}")]
pub async fn res(
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

    Ok(HttpResponse::Ok().json(guild))
}
