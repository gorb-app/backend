use actix_web::{get, rt, web, Error, HttpRequest, HttpResponse};
use actix_ws::AggregatedMessage;
use futures_util::StreamExt as _;
use uuid::Uuid;
use log::error;

use crate::{api::v1::auth::check_access_token, structs::{Channel, Member}, utils::get_auth_header, Data};

#[get("{uuid}/channels/{channel_uuid}/socket")]
pub async fn echo(req: HttpRequest, path: web::Path<(Uuid, Uuid)>, stream: web::Payload, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    // Get all headers
    let headers = req.headers();

    // Retrieve auth header
    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error)
    }

    // Get uuids from path
    let (guild_uuid, channel_uuid) = path.into_inner();

    // Authorize client using auth header
    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error)
    }

    // Unwrap user uuid from authorization
    let uuid = authorized.unwrap();

    // Get server member from psql
    let member = Member::fetch_one(&data.pool, uuid, guild_uuid).await;

    if let Err(error) = member {
        return Ok(error);
    }

    // Get cache for channel
    let cache_result = data.get_cache_key(format!("{}", channel_uuid)).await;

    let channel: Channel;

    // Return channel cache or result from psql as `channel` variable
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

    let (res, mut session, stream) = actix_ws::handle(&req, stream)?;

    let mut stream = stream
        .aggregate_continuations()
        // aggregate continuation frames up to 1MiB
        .max_continuation_size(2_usize.pow(20));

    // start task but don't wait for it
    rt::spawn(async move {
        // receive messages from websocket
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(AggregatedMessage::Text(text)) => {
                    // echo text message
                    session.text(text).await.unwrap();
                }

                Ok(AggregatedMessage::Binary(bin)) => {
                    // echo binary message
                    session.binary(bin).await.unwrap();
                }

                Ok(AggregatedMessage::Ping(msg)) => {
                    // respond to PING frame with PONG frame
                    session.pong(&msg).await.unwrap();
                }

                _ => {}
            }
        }
    });

    // respond immediately with response connected to WS session
    Ok(res)
}
