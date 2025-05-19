use crate::{
    Data,
    api::v1::auth::check_access_token,
    structs::{Member, Role},
    utils::get_auth_header,
};
use ::uuid::Uuid;
use actix_web::{Error, HttpRequest, HttpResponse, get, post, web};
use log::error;
use serde::Deserialize;

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

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error);
    }

    let guild_uuid = path.into_inner().0;

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let uuid = authorized.unwrap();

    let member = Member::fetch_one(&data.pool, uuid, guild_uuid).await;

    if let Err(error) = member {
        return Ok(error);
    }

    let cache_result = data.get_cache_key(format!("{}_roles", guild_uuid)).await;

    if let Ok(cache_hit) = cache_result {
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(cache_hit));
    }

    let roles_result = Role::fetch_all(&data.pool, guild_uuid).await;

    if let Err(error) = roles_result {
        return Ok(error);
    }

    let roles = roles_result.unwrap();

    let cache_result = data
        .set_cache_key(format!("{}_roles", guild_uuid), roles.clone(), 1800)
        .await;

    if let Err(error) = cache_result {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().finish());
    }

    Ok(HttpResponse::Ok().json(roles))
}

#[post("{uuid}/roles")]
pub async fn create(
    req: HttpRequest,
    role_info: web::Json<RoleInfo>,
    path: web::Path<(Uuid,)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error);
    }

    let guild_uuid = path.into_inner().0;

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let uuid = authorized.unwrap();

    let member = Member::fetch_one(&data.pool, uuid, guild_uuid).await;

    if let Err(error) = member {
        return Ok(error);
    }

    // FIXME: Logic to check permissions, should probably be done in utils.rs

    let role = Role::new(&data.pool, guild_uuid, role_info.name.clone()).await;

    if let Err(error) = role {
        return Ok(error);
    }

    Ok(HttpResponse::Ok().json(role.unwrap()))
}
