use actix_web::{get, web, Error, HttpRequest, HttpResponse, Scope};
use uuid::Uuid;

mod channels;
mod roles;

use crate::{api::v1::auth::check_access_token, structs::{Guild, Member}, utils::get_auth_header, Data};

pub fn web() -> Scope {
    web::scope("")
        .service(res)
        .service(channels::response)
        .service(channels::response_post)
        .service(channels::uuid::res)
        .service(channels::uuid::messages::res)
        .service(roles::response)
        .service(roles::response_post)
        .service(roles::uuid::res)
}

#[get("/{uuid}")]
pub async fn res(req: HttpRequest, path: web::Path<(Uuid,)>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error)
    }

    let guild_uuid = path.into_inner().0;

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error)
    }

    let uuid = authorized.unwrap();

    let member = Member::fetch_one(&data.pool, uuid, guild_uuid).await;

    if let Err(error) = member {
        return Ok(error);
    }

    let guild = Guild::fetch_one(&data.pool, guild_uuid).await;

    if let Err(error) = guild {
        return Ok(error);
    }

    Ok(HttpResponse::Ok().json(guild.unwrap()))
}

