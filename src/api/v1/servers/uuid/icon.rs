use actix_web::{put, web, HttpRequest, HttpResponse};
use uuid::Uuid;
use futures_util::StreamExt as _;

use crate::{error::Error, api::v1::auth::check_access_token, structs::{Guild, Member}, utils::get_auth_header, Data};

#[put("{uuid}/icon")]
pub async fn upload(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    mut payload: web::Payload,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let guild_uuid = path.into_inner().0;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    Member::fetch_one(&mut conn, uuid, guild_uuid).await?;

    let mut guild = Guild::fetch_one(&mut conn, guild_uuid).await?;

    let mut bytes = web::BytesMut::new();
    while let Some(item) = payload.next().await {
        bytes.extend_from_slice(&item?);
    }

    guild.set_icon(&data.bunny_cdn, &mut conn, data.config.bunny.cdn_url.clone(), bytes).await?;

    Ok(HttpResponse::Ok().finish())
}
