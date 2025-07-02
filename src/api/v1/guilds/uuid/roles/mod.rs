use ::uuid::Uuid;
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use serde::Deserialize;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    objects::{Member, Permissions, Role},
    utils::{get_auth_header, global_checks, order_by_is_above},
};

pub mod uuid;

#[derive(Deserialize)]
struct RoleInfo {
    name: String,
}

#[get("{uuid}/roles")]
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

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    if let Ok(cache_hit) = data.get_cache_key(format!("{guild_uuid}_roles")).await {
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(cache_hit));
    }

    let roles = Role::fetch_all(&mut conn, guild_uuid).await?;

    let roles_ordered = order_by_is_above(roles).await?;

    data.set_cache_key(format!("{guild_uuid}_roles"), roles_ordered.clone(), 1800)
        .await?;

    Ok(HttpResponse::Ok().json(roles_ordered))
}

#[post("{uuid}/roles")]
pub async fn create(
    req: HttpRequest,
    role_info: web::Json<RoleInfo>,
    path: web::Path<(Uuid,)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let guild_uuid = path.into_inner().0;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    let member = Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    member
        .check_permission(&data, Permissions::CreateRole)
        .await?;

    let role = Role::new(&mut conn, guild_uuid, role_info.name.clone()).await?;

    Ok(HttpResponse::Ok().json(role))
}
