use actix_web::{Error, HttpRequest, HttpResponse, get, rt, web};
use actix_ws::AggregatedMessage;
use futures_util::StreamExt as _;
use uuid::Uuid;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    structs::{Channel, Member},
    utils::get_auth_header,
};

#[get("{uuid}/channels/{channel_uuid}/socket")]
pub async fn echo(
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    stream: web::Payload,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    // Get all headers
    let headers = req.headers();

    // Retrieve auth header
    let auth_header = get_auth_header(headers)?;

    // Get uuids from path
    let (guild_uuid, channel_uuid) = path.into_inner();

    let mut conn = data.pool.get().await.map_err(|e| crate::error::Error::from(e))?;

    // Authorize client using auth header
    let uuid = check_access_token(auth_header, &mut conn).await?;

    // Get server member from psql
    Member::fetch_one(&mut conn, uuid, guild_uuid).await?;

    let channel: Channel;

    // Return channel cache or result from psql as `channel` variable
    if let Ok(cache_hit) = data.get_cache_key(format!("{}", channel_uuid)).await {
        channel = serde_json::from_str(&cache_hit).unwrap()
    } else {
        channel = Channel::fetch_one(&mut conn, channel_uuid).await?;

        data
            .set_cache_key(format!("{}", channel_uuid), channel.clone(), 60)
            .await?;
    }

    let (res, mut session_1, stream) = actix_ws::handle(&req, stream)?;

    let mut stream = stream
        .aggregate_continuations()
        // aggregate continuation frames up to 1MiB
        .max_continuation_size(2_usize.pow(20));

    let mut pubsub = data.cache_pool.get_async_pubsub().await.map_err(|e| crate::error::Error::from(e))?;

    let mut session_2 = session_1.clone();

    rt::spawn(async move {
        pubsub.subscribe(channel_uuid.to_string()).await.unwrap();
        while let Some(msg) = pubsub.on_message().next().await {
            let payload: String = msg.get_payload().unwrap();
            session_1.text(payload).await.unwrap();
        }
    });

    // start task but don't wait for it
    rt::spawn(async move {
        let mut conn = data
            .cache_pool
            .get_multiplexed_tokio_connection()
            .await
            .unwrap();
        // receive messages from websocket
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(AggregatedMessage::Text(text)) => {
                    // echo text message
                    redis::cmd("PUBLISH")
                        .arg(&[channel_uuid.to_string(), text.to_string()])
                        .exec_async(&mut conn)
                        .await
                        .unwrap();
                    channel
                        .new_message(&mut data.pool.get().await.unwrap(), uuid, text.to_string())
                        .await
                        .unwrap();
                }

                Ok(AggregatedMessage::Binary(bin)) => {
                    // echo binary message
                    session_2.binary(bin).await.unwrap();
                }

                Ok(AggregatedMessage::Ping(msg)) => {
                    // respond to PING frame with PONG frame
                    session_2.pong(&msg).await.unwrap();
                }

                _ => {}
            }
        }
    });

    // respond immediately with response connected to WS session
    Ok(res)
}
