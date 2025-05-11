use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use serde::Deserialize;
use crate::{api::v1::auth::check_access_token, structs::{Channel, Member}, utils::get_auth_header, Data};
use ::uuid::Uuid;
use log::error;

#[derive(Deserialize)]
struct MessageRequest {
    amount: i64,
    offset: i64,
}

#[get("{uuid}/channels/{channel_uuid}/messages")]
pub async fn res(req: HttpRequest, path: web::Path<(Uuid, Uuid)>, message_request: web::Json<MessageRequest>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error)
    }

    let (guild_uuid, channel_uuid) = path.into_inner();

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error)
    }

    let uuid = authorized.unwrap();

    let member = Member::fetch_one(&data.pool, uuid, guild_uuid).await;

    if let Err(error) = member {
        return Ok(error);
    }

    let cache_result = data.get_cache_key(format!("{}", channel_uuid)).await;

    let channel: Channel;

    if let Ok(cache_hit) = cache_result {
        channel = serde_json::from_str(&cache_hit).unwrap()
    } else {
        let channel_result = Channel::fetch_one(&data.pool, guild_uuid, channel_uuid).await;

        if let Err(error) = channel_result {
            return Ok(error)
        }
    
        channel = channel_result.unwrap();
    
        let cache_result = data.set_cache_key(format!("{}", channel_uuid), channel.clone(), 60).await;
    
        if let Err(error) = cache_result {
            error!("{}", error);
            return Ok(HttpResponse::InternalServerError().finish());
        }
    }

    let messages = channel.fetch_messages(&data.pool, message_request.amount, message_request.offset).await;

    if let Err(error) = messages {
        return Ok(error)
    }

    Ok(HttpResponse::Ok().json(messages.unwrap()))
}
