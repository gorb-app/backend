use actix_web::{get, post, web, Error, HttpRequest, HttpResponse, Scope};
use serde::Deserialize;

mod uuid;

use crate::{api::v1::auth::check_access_token, structs::{Guild, StartAmountQuery}, utils::get_auth_header, Data};

#[derive(Deserialize)]
struct GuildInfo {
    name: String,
    description: Option<String>,
}

pub fn web() -> Scope {
    web::scope("/servers")
        .service(create)
        .service(get)
        .service(uuid::web())
}

#[post("")]
pub async fn create(
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

#[get("")]
pub async fn get(
    req: HttpRequest,
    request_query: web::Query<StartAmountQuery>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    let start = request_query.start.unwrap_or(0);

    let amount = request_query.amount.unwrap_or(10);

    if let Err(error) = auth_header {
        return Ok(error);
    }

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let guilds = Guild::fetch_amount(&data.pool, start, amount).await;

    if let Err(error) = guilds {
        return Ok(error);
    }

    Ok(HttpResponse::Ok().json(guilds.unwrap()))
}

