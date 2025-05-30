//! `/api/v1/guilds` Guild related endpoints

use actix_web::{HttpRequest, HttpResponse, Scope, get, post, web};
use serde::Deserialize;

mod uuid;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    structs::{Guild, StartAmountQuery},
    utils::{get_auth_header, global_checks},
};

#[derive(Deserialize)]
struct GuildInfo {
    name: String,
}

pub fn web() -> Scope {
    web::scope("/guilds")
        .service(post)
        .service(get)
        .service(uuid::web())
}

/// `POST /api/v1/guilds` Creates a new guild
///
/// requires auth: yes
///
/// ### Request Example
/// ```
/// json!({
///     "name": "My new server!"
/// });
/// ```
///
/// ### Response Example
/// ```
/// json!({
///     "uuid": "383d2afa-082f-4dd3-9050-ca6ed91487b6",
///     "name": "My new server!",
///     "description": null,
///     "icon": null,
///     "owner_uuid": "155d2291-fb23-46bd-a656-ae7c5d8218e6",
///     "roles": [],
///     "member_count": 1
/// });
/// ```
/// NOTE: UUIDs in this response are made using `uuidgen`, UUIDs made by the actual backend will be UUIDv7 and have extractable timestamps
#[post("")]
pub async fn post(
    req: HttpRequest,
    guild_info: web::Json<GuildInfo>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    let guild = Guild::new(&mut conn, guild_info.name.clone(), uuid).await?;

    Ok(HttpResponse::Ok().json(guild))
}

/// `GET /api/v1/servers` Fetches all guilds
///
/// requires auth: yes
///
/// requires admin: yes
///
/// ### Response Example
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
#[get("")]
pub async fn get(
    req: HttpRequest,
    request_query: web::Query<StartAmountQuery>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let start = request_query.start.unwrap_or(0);

    let amount = request_query.amount.unwrap_or(10);

    let uuid = check_access_token(auth_header, &mut data.pool.get().await?).await?;

    global_checks(&data, uuid).await?;

    let guilds = Guild::fetch_amount(&data.pool, start, amount).await?;

    Ok(HttpResponse::Ok().json(guilds))
}
