use actix_web::{Error, HttpRequest, HttpResponse, Scope, post, web};
use serde::Deserialize;

mod uuid;

use crate::{Data, api::v1::auth::check_access_token, structs::Guild, utils::get_auth_header};

#[derive(Deserialize)]
struct GuildInfo {
    name: String,
    description: Option<String>,
}

pub fn web() -> Scope {
    web::scope("/servers").service(res).service(uuid::web())
}

#[post("")]
pub async fn res(
    req: HttpRequest,
    guild_info: web::Json<GuildInfo>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error);
    }

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let uuid = authorized.unwrap();

    let guild = Guild::new(
        &data.pool,
        guild_info.name.clone(),
        guild_info.description.clone(),
        uuid,
    )
    .await;

    if let Err(error) = guild {
        return Ok(error);
    }

    Ok(HttpResponse::Ok().json(guild.unwrap()))
}
