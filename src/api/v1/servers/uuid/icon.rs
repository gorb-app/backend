use actix_web::{put, web, Error, HttpRequest, HttpResponse};
use uuid::Uuid;
use futures_util::StreamExt as _;

use crate::{api::v1::auth::check_access_token, structs::{Guild, Member}, utils::get_auth_header, Data};

#[put("{uuid}/icon")]
pub async fn upload(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    mut payload: web::Payload,
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

    let guild_result = Guild::fetch_one(&data.pool, guild_uuid).await;

    if let Err(error) = guild_result {
        return Ok(error);
    }

    let mut guild = guild_result.unwrap();

    let mut bytes = web::BytesMut::new();
    while let Some(item) = payload.next().await {
        bytes.extend_from_slice(&item?);
    }

    if let Err(error) = guild.set_icon(&data.bunny_cdn, &data.pool, data.config.bunny.cdn_url.clone(), bytes).await {
        return Ok(error)
    }

    Ok(HttpResponse::Ok().finish())
}
