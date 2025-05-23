use crate::{
    error::Error,
    Data,
    api::v1::auth::check_access_token,
    structs::{Member, Role},
    utils::get_auth_header,
};
use ::uuid::Uuid;
use actix_web::{HttpRequest, HttpResponse, get, web};

#[get("{uuid}/roles/{role_uuid}")]
pub async fn get(
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let (guild_uuid, role_uuid) = path.into_inner();

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    Member::fetch_one(&mut conn, uuid, guild_uuid).await?;

    if let Ok(cache_hit) = data.get_cache_key(format!("{}", role_uuid)).await {
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(cache_hit));
    }

    let role = Role::fetch_one(&mut conn, role_uuid).await?;

    data
        .set_cache_key(format!("{}", role_uuid), role.clone(), 60)
        .await?;

    Ok(HttpResponse::Ok().json(role))
}
