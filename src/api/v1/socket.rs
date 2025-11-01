use std::sync::Arc;

use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    http::HeaderMap,
    response::IntoResponse,
};
use bytes::Bytes;
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper, delete, dsl::insert_into, update};
use diesel_async::RunQueryDsl;
use futures_util::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tokio::{sync::{Mutex, mpsc::{self, error::TryRecvError}}, time::{self, Duration}};

use crate::{
    AppState, api::v1::auth::check_access_token, error::Error, objects::{self, message::MessageBuilder}, schema::messages, utils::global_checks
};

#[derive(Deserialize)]
#[serde(tag = "event")]
enum ReceiveEvent {
    MessageSend { entity: MessageSend },
    MessageEdit { entity: MessageEdit },
    MessageDelete { entity: MessageDelete },
    ChannelSubscribe { entity: Uuid },
    //ChannelUnsubscribe { entity: Uuid },
}

#[derive(Deserialize)]
struct MessageSend {
    channel_uuid: Uuid,
    text: String,
    reply_to: Option<Uuid>,
}

#[derive(Deserialize)]
struct MessageEdit {
    channel_uuid: Uuid,
    uuid: Uuid,
    text: String,
}

#[derive(Deserialize, Serialize)]
struct MessageDelete {
    channel_uuid: Uuid,
    uuid: Uuid,
}

#[derive(Serialize)]
#[serde(tag = "event")]
enum SendEvent {
    MessageSend { entity: objects::Message },
    MessageEdit { entity: objects::Message },
    MessageDelete { entity: MessageDelete },
    Error { entity: SendError },
}

impl TryInto<Message> for SendEvent {
    type Error = Error;

    fn try_into(self) -> Result<Message, Error> {
        let json = serde_json::to_string(&self)?;

        Ok(json.into())
    }
}

#[derive(Serialize)]
struct SendError {
    message: String,
}

pub async fn ws(
    ws: WebSocketUpgrade,
    State(app_state): State<&'static AppState>,
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

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let pubsub = Arc::new(Mutex::new(app_state
        .cache_pool
        .get_async_pubsub()
        .await
        .map_err(crate::error::Error::from)?));

    let mut res = ws.on_upgrade(async move |socket| {
        let (mut sender, receiver) = socket.split();
        let (sender_heartbeat, receiver_heartbeat) = mpsc::channel(5);
        let (sender_ws, mut receiver_ws) = mpsc::channel::<Message>(10);

        heartbeat(sender_ws.clone(), receiver_heartbeat).await;

        channel_sender(pubsub.clone(), sender_ws).await;

        websocket_receiver(app_state, uuid, sender_heartbeat, receiver, pubsub).await;
    });

    let headers = res.headers_mut();

    headers.append(
        axum::http::header::SEC_WEBSOCKET_PROTOCOL,
        "Authorization".parse()?,
    );

    // respond immediately with response connected to WS session
    Ok(res)
}

async fn heartbeat(sender: mpsc::Sender<Message>, mut receiver: mpsc::Receiver<&'static str>) -> tokio::task::JoinHandle<Result<(), Error>> {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60));

        'outer: loop {
            interval.tick().await;
            sender.send(Message::Ping(Bytes::new())).await?;
            let mut interval = time::interval(Duration::from_secs(10));
            for i in 0..5 {
                interval.tick().await;
                let msg = receiver.try_recv();

                if let Ok(_) = msg {
                    break;
                } else if msg == Err(TryRecvError::Disconnected) || msg == Err(TryRecvError::Empty) && i == 4 {
                    // Todo figure out how to tell the socket to close
                    sender.send(Message::Text("".into())).await?;
                    break 'outer;
                }
            }
        }

        Ok(())
    })
}

async fn channel_sender(pubsub: Arc<Mutex<redis::aio::PubSub>>, sender: mpsc::Sender<Message>) -> tokio::task::JoinHandle<Result<(), Error>> {
    tokio::spawn(async move {
        while let Some(msg) = pubsub.lock().await.on_message().next().await {
            let payload: String = msg.get_payload()?;
            sender.send(payload.into()).await?;
        }

        Ok(())
    })
}

async fn websocket_receiver(app_state: &'static AppState, uuid: Uuid, sender_heartbeat: mpsc::Sender<&'static str>, mut receiver: SplitStream<WebSocket>, pubsub: Arc<Mutex<redis::aio::PubSub>>) -> tokio::task::JoinHandle<Result<(), Error>> {
    tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg? {
                Message::Pong(_) => {
                    sender_heartbeat.send("").await?;
                },
                Message::Text(text) => {
                    let message_body: ReceiveEvent = serde_json::from_str(&text)?;


                    match message_body {
                        ReceiveEvent::MessageSend { entity } => {
                            // FIXME: We literally dont even check if the user has access anymore pls send help
                            let message_uuid = Uuid::now_v7();

                            let message = MessageBuilder {
                                uuid: message_uuid,
                                channel_uuid: entity.channel_uuid,
                                user_uuid: uuid,
                                message: entity.text,
                                reply_to: entity.reply_to,
                            };

                            insert_into(messages::table)
                                .values(message.clone())
                                .execute(&mut app_state.pool.get().await?)
                                .await?;

                            let message = message.build(&mut app_state.pool.get().await?, &app_state.cache_pool).await?;

                            redis::cmd("PUBLISH")
                                .arg(&[
                                    entity.channel_uuid.to_string(),
                                    serde_json::to_string(&SendEvent::MessageSend {
                                        entity: message,
                                    })?,
                                ])
                                .exec_async(
                                    &mut app_state
                                        .cache_pool
                                        .get_multiplexed_tokio_connection()
                                        .await?,
                                )
                                .await?;
                        }
                        ReceiveEvent::MessageEdit { entity } => {
                            use messages::dsl;
                            let mut message: MessageBuilder = dsl::messages
                                .filter(dsl::uuid.eq(entity.uuid))
                                .select(MessageBuilder::as_select())
                                .get_result(&mut app_state.pool.get().await?)
                                .await?;

                            if uuid != message.user_uuid {
                                redis::cmd("PUBLISH")
                                    .arg(&[
                                        entity.channel_uuid.to_string(),
                                        serde_json::to_string(&SendEvent::Error {
                                            entity: SendError {
                                                message: "Not allowed".to_string(),
                                            },
                                        })?,
                                    ])
                                    .exec_async(
                                        &mut app_state
                                            .cache_pool
                                            .get_multiplexed_tokio_connection()
                                            .await?,
                                    )
                                    .await?;

                                continue;
                            }

                            update(messages::table)
                                .filter(dsl::uuid.eq(entity.uuid))
                                .set(dsl::message.eq(&entity.text))
                                .execute(&mut app_state.pool.get().await?)
                                .await?;

                            message.message = entity.text;

                            redis::cmd("PUBLISH")
                                .arg(&[
                                    entity.channel_uuid.to_string(),
                                    serde_json::to_string(&SendEvent::MessageEdit {
                                        entity: message
                                            .build(
                                                &mut app_state.pool.get().await?,
                                                &app_state.cache_pool,
                                            )
                                            .await?,
                                    })?,
                                ])
                                .exec_async(
                                    &mut app_state
                                        .cache_pool
                                        .get_multiplexed_tokio_connection()
                                        .await?,
                                )
                                .await?;
                        }
                        ReceiveEvent::MessageDelete { entity } => {
                            use messages::dsl;
                            let message: MessageBuilder = dsl::messages
                                .filter(dsl::uuid.eq(entity.uuid))
                                .select(MessageBuilder::as_select())
                                .get_result(&mut app_state.pool.get().await?)
                                .await?;

                            if uuid != message.user_uuid {
                                redis::cmd("PUBLISH")
                                    .arg(&[
                                        entity.channel_uuid.to_string(),
                                        serde_json::to_string(&SendEvent::Error {
                                            entity: SendError {
                                                message: "Not allowed".to_string(),
                                            },
                                        })?,
                                    ])
                                    .exec_async(
                                        &mut app_state
                                            .cache_pool
                                            .get_multiplexed_tokio_connection()
                                            .await?,
                                    )
                                    .await?;

                                continue;
                            }

                            delete(messages::table)
                                .filter(dsl::uuid.eq(entity.uuid))
                                .execute(&mut app_state.pool.get().await?)
                                .await?;

                            redis::cmd("PUBLISH")
                                .arg(&[
                                    entity.channel_uuid.to_string(),
                                    serde_json::to_string(&SendEvent::MessageDelete { entity })?,
                                ])
                                .exec_async(
                                    &mut app_state
                                        .cache_pool
                                        .get_multiplexed_tokio_connection()
                                        .await?,
                                )
                                .await?;
                        },
                        ReceiveEvent::ChannelSubscribe { entity } => {
                            pubsub.lock().await.subscribe(entity.to_string()).await?;
                        },
                    }
                },
                _ => {},
            }
        }

        Ok(())
    })
}

async fn websocket_sender(sender: SplitSink<WebSocket, Message>) -> tokio::task::JoinHandle<Result<(), Error>> {
    tokio::spawn(async move {
        while let Some(msg) = sender.on_message().next().await {
            let payload: String = msg.get_payload()?;
            sender.send(payload.into()).await?;
        }

        Ok(())
    })
}
