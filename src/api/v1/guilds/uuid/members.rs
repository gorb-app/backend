use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    objects::Member,
    utils::{get_auth_header, global_checks},
};
use ::uuid::Uuid;
use actix_web::{HttpRequest, HttpResponse, get, web};

#[get("{uuid}/members")]
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

    global_checks(&data, uuid).await?;

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    let members = Member::fetch_all(&data, guild_uuid).await?;

    Ok(HttpResponse::Ok().json(members))
}
