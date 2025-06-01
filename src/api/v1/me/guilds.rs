//! `/api/v1/me/guilds` Contains endpoint related to guild memberships

use actix_web::{HttpRequest, HttpResponse, get, web};

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    objects::Me,
    utils::{get_auth_header, global_checks},
};

/// `GET /api/v1/me/guilds` Returns all guild memberships in a list
///
/// requires auth: yes
///
/// ### Example Response
/// ```
/// json!([
///     {
///         "uuid": "383d2afa-082f-4dd3-9050-ca6ed91487b6",
///         "name": "My new server!",
///         "description": null,
///         "icon": null,
///         "owner_uuid": "155d2291-fb23-46bd-a656-ae7c5d8218e6",
///         "roles": [],
///         "member_count": 1
///     },
///     {
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
///     }
/// ]);
/// ```
/// NOTE: UUIDs in this response are made using `uuidgen`, UUIDs made by the actual backend will be UUIDv7 and have extractable timestamps
#[get("/guilds")]
pub async fn get(req: HttpRequest, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    let me = Me::get(&mut conn, uuid).await?;

    let memberships = me.fetch_memberships(&mut conn).await?;

    Ok(HttpResponse::Ok().json(memberships))
}
