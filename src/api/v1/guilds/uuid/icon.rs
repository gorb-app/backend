//! `/api/v1/guilds/{uuid}/icon` icon related endpoints, will probably be replaced by a multipart post to above endpoint

use actix_web::{HttpRequest, HttpResponse, put, web};
use futures_util::StreamExt as _;
use uuid::Uuid;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    objects::{Guild, Member, Permissions},
    utils::{get_auth_header, global_checks},
};

/// `PUT /api/v1/guilds/{uuid}/icon` Icon upload
///
/// requires auth: no
///
/// put request expects a file and nothing else
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

    global_checks(&data, uuid).await?;

    let member = Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    member
        .check_permission(&data, Permissions::ManageGuild)
        .await?;

    let mut guild = Guild::fetch_one(&mut conn, guild_uuid).await?;

    let mut bytes = web::BytesMut::new();
    while let Some(item) = payload.next().await {
        bytes.extend_from_slice(&item?);
    }

    guild
        .set_icon(
            &data.bunny_storage,
            &mut conn,
            data.config.bunny.cdn_url.clone(),
            bytes,
        )
        .await?;

    Ok(HttpResponse::Ok().finish())
}
