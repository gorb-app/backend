//! `/api/v1/users/{uuid}` Specific user endpoints

use actix_web::{HttpRequest, HttpResponse, get, web};
use uuid::Uuid;

use crate::{
    api::v1::auth::check_access_token, error::Error, objects::{Me, User}, utils::{get_auth_header, global_checks}, Data
};

/// `GET /api/v1/users/{uuid}` Returns user with the given UUID
///
/// requires auth: yes
///
/// requires relation: yes
///
/// ### Response Example
/// ```
/// json!({
///         "uuid": "155d2291-fb23-46bd-a656-ae7c5d8218e6",
///         "username": "user1",
///         "display_name": "Nullable Name",
///         "avatar": "https://nullable-url.com/path/to/image.png"
/// });
/// ```
/// NOTE: UUIDs in this response are made using `uuidgen`, UUIDs made by the actual backend will be UUIDv7 and have extractable timestamps
#[get("/{uuid}")]
pub async fn get(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let user_uuid = path.into_inner().0;

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    let me = Me::get(&mut conn, uuid).await?;

    let user = User::fetch_one_with_friendship(&data, &me, user_uuid).await?;

    Ok(HttpResponse::Ok().json(user))
}
