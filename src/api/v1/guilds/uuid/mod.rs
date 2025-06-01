//! `/api/v1/guilds/{uuid}` Specific server endpoints

use actix_web::{HttpRequest, HttpResponse, Scope, get, web};
use uuid::Uuid;

mod channels;
mod icon;
mod invites;
mod members;
mod roles;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    objects::{Guild, Member},
    utils::{get_auth_header, global_checks},
};

pub fn web() -> Scope {
    web::scope("")
        // Servers
        .service(get)
        // Channels
        .service(channels::get)
        .service(channels::create)
        // Roles
        .service(roles::get)
        .service(roles::create)
        .service(roles::uuid::get)
        // Invites
        .service(invites::get)
        .service(invites::create)
        // Icon
        .service(icon::upload)
        // Members
        .service(members::get)
}

/// `GET /api/v1/guilds/{uuid}` DESCRIPTION
///
/// requires auth: yes
///
/// ### Response Example
/// ```
/// json!({
///         "uuid": "5ba61ec7-5f97-43e1-89a5-d4693c155612",
///         "name": "My first server!",
///         "description": "This is a cool and nullable description!",
///         "icon": "https://nullable-url/path/to/icon.png",
///         "owner_uuid": "155d2291-fb23-46bd-a656-ae7c5d8218e6",
///         "roles": [
///             {
///                 "uuid": "be0e4da4-cf73-4f45-98f8-bb1c73d1ab8b",
///                 "guild_uuid": "5ba61ec7-5f97-43e1-89a5-d4693c155612",
///                 "name": "Cool people",
///                 "color": 15650773,
///                 "is_above": c7432f1c-f4ad-4ad3-8216-51388b6abb5b,
///                 "permissions": 0
///             }
///             {
///                 "uuid": "c7432f1c-f4ad-4ad3-8216-51388b6abb5b",
///                 "guild_uuid": "5ba61ec7-5f97-43e1-89a5-d4693c155612",
///                 "name": "Equally cool people",
///                 "color": 16777215,
///                 "is_above": null,
///                 "permissions": 0
///             }
///         ],
///         "member_count": 20
/// });
/// ```
#[get("/{uuid}")]
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

    Ok(HttpResponse::Ok().json(guild))
}
