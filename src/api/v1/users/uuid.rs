//! `/api/v1/users/{uuid}` Specific user endpoints

use actix_web::{HttpRequest, HttpResponse, get, web};
use uuid::Uuid;

use crate::{
    Data, api::v1::auth::check_access_token, error::Error, structs::User, utils::get_auth_header,
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

    let uuid = path.into_inner().0;

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    check_access_token(auth_header, &mut conn).await?;

    if let Ok(cache_hit) = data.get_cache_key(uuid.to_string()).await {
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(cache_hit));
    }

    let user = User::fetch_one(&mut conn, uuid).await?;

    data.set_cache_key(uuid.to_string(), user.clone(), 1800)
        .await?;

    Ok(HttpResponse::Ok().json(user))
}
