use std::sync::Arc;

use axum::{
    extract::{Path, State, WebSocketUpgrade, ws::Message},
    http::HeaderMap,
    response::IntoResponse,
};
use futures::SinkExt;
use futures_util::StreamExt as _;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    api::v1::auth::check_access_token,
    error::Error,
    objects::{Channel, Member},
    utils::global_checks,
};

#[derive(Deserialize)]
struct MessageBody {
    message: String,
    reply_to: Option<Uuid>,
}

pub async fn ws(
    ws: WebSocketUpgrade,
    State(app_state): State<Arc<AppState>>,
    Path(channel_uuid): Path<Uuid>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, Error> {
    // Retrieve auth header
    let auth_token = headers.get(axum::http::header::SEC_WEBSOCKET_PROTOCOL);

    if auth_token.is_none() {
        return Err(Error::Unauthorized(
            "No authorization header provided".to_string(),
        ));
    }

    let auth_raw = auth_token.unwrap().to_str()?;

    let mut auth = auth_raw.split_whitespace();

    let response_proto = auth.next();

    let auth_value = auth.next();

    if response_proto.is_none() {
        return Err(Error::BadRequest(
            "Sec-WebSocket-Protocol header is empty".to_string(),
        ));
    } else if response_proto.is_some_and(|rp| rp != "Authorization,") {
        return Err(Error::BadRequest(
            "First protocol should be Authorization".to_string(),
        ));
    }

    if auth_value.is_none() {
        return Err(Error::BadRequest("No token provided".to_string()));
    }

    let auth_header = auth_value.unwrap();

    let mut conn = app_state
        .pool
        .get()
        .await
        .map_err(crate::error::Error::from)?;

    // Authorize client using auth header
    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&app_state, uuid).await?;

    let channel = Channel::fetch_one(&app_state, channel_uuid).await?;

    Member::check_membership(&mut conn, uuid, channel.guild_uuid).await?;

    let mut pubsub = app_state
        .cache_pool
        .get_async_pubsub()
        .await
        .map_err(crate::error::Error::from)?;

    let mut res = ws.on_upgrade(async move |socket| {
        let (mut sender, mut receiver) = socket.split();

        tokio::spawn(async move {
            pubsub.subscribe(channel_uuid.to_string()).await?;
            while let Some(msg) = pubsub.on_message().next().await {
                let payload: String = msg.get_payload()?;
                sender.send(payload.into()).await?;
            }

            Ok::<(), crate::error::Error>(())
        });

        tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                if let Ok(Message::Text(text)) = msg {
                    let message_body: MessageBody = serde_json::from_str(&text)?;

                    let message = channel
                        .new_message(
                            &app_state,
                            uuid,
                            message_body.message,
                            message_body.reply_to,
                        )
                        .await?;

                    redis::cmd("PUBLISH")
                        .arg(&[channel_uuid.to_string(), serde_json::to_string(&message)?])
                        .exec_async(
                            &mut app_state
                                .cache_pool
                                .get_multiplexed_tokio_connection()
                                .await?,
                        )
                        .await?;
                }
            }

            Ok::<(), crate::error::Error>(())
        });
    });

    let headers = res.headers_mut();

    headers.append(
        axum::http::header::SEC_WEBSOCKET_PROTOCOL,
        "Authorization".parse()?,
    );

    // respond immediately with response connected to WS session
    Ok(res)
}
