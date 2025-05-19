use actix_web::{get, post, web, Error, HttpRequest, HttpResponse};
use serde::Deserialize;
use uuid::Uuid;

use crate::{api::v1::auth::check_access_token, structs::{Guild, Member}, utils::get_auth_header, Data};

#[derive(Deserialize)]
struct InviteRequest {
    custom_id: String,
}

#[get("{uuid}/invites")]
pub async fn get(req: HttpRequest, path: web::Path<(Uuid,)>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
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

    let guild_result = Guild::fetch_one(&data.pool, guild_uuid).await;

    if let Err(error) = guild_result {
        return Ok(error);
    }

    let guild = guild_result.unwrap();

    let invites = guild.get_invites(&data.pool).await;

    if let Err(error) = invites {
        return Ok(error);
    }    

    Ok(HttpResponse::Ok().json(invites.unwrap()))
}

#[post("{uuid}/invites")]
pub async fn create(req: HttpRequest, path: web::Path<(Uuid,)>, invite_request: web::Json<Option<InviteRequest>>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
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

    let member_result = Member::fetch_one(&data.pool, uuid, guild_uuid).await;

    if let Err(error) = member_result {
        return Ok(error);
    }

    let member = member_result.unwrap();

    let guild_result = Guild::fetch_one(&data.pool, guild_uuid).await;

    if let Err(error) = guild_result {
        return Ok(error);
    }

    let guild = guild_result.unwrap();

    let custom_id =  invite_request.as_ref().map(|ir| ir.custom_id.clone());

    let invite = guild.create_invite(&data.pool, &member, custom_id).await;

    if let Err(error) = invite {
        return Ok(error);
    }

    Ok(HttpResponse::Ok().json(invite.unwrap()))
}
