use actix_web::{get, post, web, HttpRequest, HttpResponse, Scope};
use serde::Deserialize;

mod uuid;

use crate::{error::Error, api::v1::auth::check_access_token, structs::{Guild, StartAmountQuery}, utils::get_auth_header, Data};

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

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    let guild = Guild::new(
        &mut conn,
        guild_info.name.clone(),
        guild_info.description.clone(),
        uuid,
    )
    .await?;

    Ok(HttpResponse::Ok().json(guild))
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    request_query: web::Query<StartAmountQuery>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let start = request_query.start.unwrap_or(0);

    let amount = request_query.amount.unwrap_or(10);

    check_access_token(auth_header, &mut data.pool.get().await.unwrap()).await?;

    let guilds = Guild::fetch_amount(&data.pool, start, amount).await?;

    Ok(HttpResponse::Ok().json(guilds))
}

