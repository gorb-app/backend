//! `/api/v1/me/guilds` Contains endpoint related to guild memberships

use actix_web::{get, web, HttpRequest, HttpResponse};

use crate::{api::v1::auth::check_access_token, error::Error, structs::Me, utils::{get_auth_header, global_checks}, Data};


/// `GET /api/v1/me/guilds` Returns all guild memberships in a list
/// 
/// requires auth: yes
/// 
/// ### Example Response
/// ```
/// json!([
///     {
///         "uuid": "22006503-fb01-46e6-8e0e-70336dac6c63",
///         "nickname": "This field is nullable",
///         "user_uuid": "522bca17-de63-4706-9d18-0971867ad1e0",
///         "guild_uuid": "0911e468-3e9e-47bf-8381-59b30e8b68a8"
///     },
///     {
///         "uuid": "bf95361e-3b64-4704-969c-3c5a80d10514",
///         "nickname": null,
///         "user_uuid": "522bca17-de63-4706-9d18-0971867ad1e0",
///         "guild_uuid": "69ec2ce5-3d8b-4451-b644-c2d969905458"
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

    let memberships = me.fetch_memberships(&data).await?;

    Ok(HttpResponse::Ok().json(memberships))
}