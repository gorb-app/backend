use actix_web::{get, post, web, Error, HttpRequest, HttpResponse};

use crate::{api::v1::auth::check_access_token, structs::{Guild, Invite, Member}, utils::get_auth_header, Data};

#[get("{id}")]
pub async fn get(req: HttpRequest, path: web::Path<(String,)>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error)
    }

    let invite_id = path.into_inner().0;

    let result = Invite::fetch_one(&data.pool, invite_id).await;

    if let Err(error) = result {
        return Ok(error)
    }

    let invite = result.unwrap();

    let guild_result = Guild::fetch_one(&data.pool, invite.guild_uuid).await;

    if let Err(error) = guild_result {
        return Ok(error);
    }

    let guild = guild_result.unwrap();

    Ok(HttpResponse::Ok().json(guild))
}

#[post("{id}")]
pub async fn join(req: HttpRequest, path: web::Path<(String,)>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error)
    }

    let invite_id = path.into_inner().0;

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error)
    }

    let uuid = authorized.unwrap();

    let result = Invite::fetch_one(&data.pool, invite_id).await;

    if let Err(error) = result {
        return Ok(error)
    }

    let invite = result.unwrap();

    let guild_result = Guild::fetch_one(&data.pool, invite.guild_uuid).await;

    if let Err(error) = guild_result {
        return Ok(error);
    }

    let guild = guild_result.unwrap();

    let member = Member::new(&data.pool, uuid, guild.uuid).await;
    
    if let Err(error) = member {
        return Ok(error);
    }

    Ok(HttpResponse::Ok().json(guild))
}
