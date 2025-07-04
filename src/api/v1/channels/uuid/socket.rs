use actix_web::{
    Error, HttpRequest, HttpResponse, get,
    http::header::{HeaderValue, SEC_WEBSOCKET_PROTOCOL},
    rt, web,
};
use actix_ws::AggregatedMessage;
use futures_util::StreamExt as _;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    objects::{Channel, Member},
    utils::{get_ws_protocol_header, global_checks},
};

#[derive(Deserialize)]
struct MessageBody {
    message: String,
    reply_to: Option<Uuid>,
}

#[get("/{uuid}/socket")]
pub async fn ws(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    stream: web::Payload,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    // Get all headers
    let headers = req.headers();

    // Retrieve auth header
    let auth_header = get_ws_protocol_header(headers)?;

    // Get uuid from path
    let channel_uuid = path.into_inner().0;

    let mut conn = data.pool.get().await.map_err(crate::error::Error::from)?;

    // Authorize client using auth header
    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    let channel = Channel::fetch_one(&data, channel_uuid).await?;

    Member::check_membership(&mut conn, uuid, channel.guild_uuid).await?;

    let (mut res, mut session_1, stream) = actix_ws::handle(&req, stream)?;

    let mut stream = stream
        .aggregate_continuations()
        // aggregate continuation frames up to 1MiB
        .max_continuation_size(2_usize.pow(20));

    let mut pubsub = data
        .cache_pool
        .get_async_pubsub()
        .await
        .map_err(crate::error::Error::from)?;

    let mut session_2 = session_1.clone();

    rt::spawn(async move {
        pubsub.subscribe(channel_uuid.to_string()).await?;
        while let Some(msg) = pubsub.on_message().next().await {
            let payload: String = msg.get_payload()?;
            session_1.text(payload).await?;
        }

        Ok::<(), crate::error::Error>(())
    });

    // start task but don't wait for it
    rt::spawn(async move {
        // receive messages from websocket
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(AggregatedMessage::Text(text)) => {
                    let mut conn = data.cache_pool.get_multiplexed_tokio_connection().await?;

                    let message_body: MessageBody = serde_json::from_str(&text)?;

                    let message = channel.new_message(&data, uuid, message_body.message, message_body.reply_to).await?;

                    redis::cmd("PUBLISH")
                        .arg(&[channel_uuid.to_string(), serde_json::to_string(&message)?])
                        .exec_async(&mut conn)
                        .await?;
                }

                Ok(AggregatedMessage::Binary(bin)) => {
                    // echo binary message
                    session_2.binary(bin).await?;
                }

                Ok(AggregatedMessage::Ping(msg)) => {
                    // respond to PING frame with PONG frame
                    session_2.pong(&msg).await?;
                }

                _ => {}
            }
        }

        Ok::<(), crate::error::Error>(())
    });

    let headers = res.headers_mut();

    headers.append(
        SEC_WEBSOCKET_PROTOCOL,
        HeaderValue::from_str("Authorization")?,
    );

    // respond immediately with response connected to WS session
    Ok(res)
}
