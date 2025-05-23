use actix_web::{HttpRequest, HttpResponse, get, web};
use uuid::Uuid;

use crate::{error::Error, api::v1::auth::check_access_token, structs::User, utils::get_auth_header, Data};


#[get("/{uuid}")]
pub async fn res(
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

    data
        .set_cache_key(uuid.to_string(), user.clone(), 1800)
        .await?;

    Ok(HttpResponse::Ok().json(user))
}
