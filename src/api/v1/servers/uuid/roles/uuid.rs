use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use crate::{api::v1::auth::check_access_token, structs::{Member, Role}, utils::get_auth_header, Data};
use ::uuid::Uuid;
use log::error;

#[get("{uuid}/roles/{role_uuid}")]
pub async fn res(req: HttpRequest, path: web::Path<(Uuid, Uuid)>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error)
    }

    let (guild_uuid, role_uuid) = path.into_inner();

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error)
    }

    let uuid = authorized.unwrap();

    let member = Member::fetch_one(&data.pool, uuid, guild_uuid).await;

    if let Err(error) = member {
        return Ok(error);
    }

    let cache_result = data.get_cache_key(format!("{}", role_uuid)).await;

    if let Ok(cache_hit) = cache_result {
        return Ok(HttpResponse::Ok().content_type("application/json").body(cache_hit))
    }

    let role_result = Role::fetch_one(&data.pool, guild_uuid, role_uuid).await;

    if let Err(error) = role_result {
        return Ok(error)
    }

    let role = role_result.unwrap();

    let cache_result = data.set_cache_key(format!("{}", role_uuid), role.clone(), 60).await;

    if let Err(error) = cache_result {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().finish());
    }

    Ok(HttpResponse::Ok().json(role))
}
