use actix_web::{delete, web, Error, HttpRequest, HttpResponse};
use uuid::Uuid;

use crate::{api::v1::auth::check_access_token, structs::{Invite, Member}, utils::get_auth_header, Data};

#[delete("{uuid}/invites/{id}")]
pub async fn delete(req: HttpRequest, path: web::Path<(Uuid, String)>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error)
    }

    let (guild_uuid, invite_id) = path.into_inner();

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error)
    }

    let uuid = authorized.unwrap();

    if let Err(error) = Member::fetch_one(&data.pool, uuid, guild_uuid).await {
        return Ok(error)
    }

    let result = Invite::fetch_one(&data.pool, invite_id).await;

    if let Err(error) = result {
        return Ok(error)
    }

    let invite = result.unwrap();

    if let Err(error) = invite.delete(&data.pool).await {
        return Ok(error)
    }

    Ok(HttpResponse::Ok().finish())
}
