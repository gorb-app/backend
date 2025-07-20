use std::sync::Arc;

use log::info;
use rmpv::Value;
use socketioxide::extract::{AckSender, Data, SocketRef, State};

use crate::AppState;

pub async fn on_connect(
    State(_app_state): State<Arc<AppState>>,
    socket: SocketRef,
    Data(data): Data<Value>,
) {
    socket.emit("auth", &data).ok();

    socket.on("message", async |socket: SocketRef, Data::<Value>(data)| {
        info!("{data}");
        socket.emit("message-back", &data).ok();
    });

    socket.on(
        "message-with-ack",
        async |Data::<Value>(data), ack: AckSender| {
            info!("{data}");
            ack.send(&data).ok();
        },
    );
}
