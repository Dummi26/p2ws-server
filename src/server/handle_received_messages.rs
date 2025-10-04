use std::sync::Arc;

use futures_util::SinkExt;
use tokio::{sync::Mutex, time::Instant};

use crate::{
    data::{Area, Color, Coordinate},
    protocol::P2Decodable,
    server::{
        P2Read, P2Write, WebsocketServer,
        connection_data::ActiveConnectionData,
        connections::{ReadableWebsocketStream, WritableWebsocketStream},
        handle_connection::{Disconnected, HandleConnectionError},
    },
    users::UserId,
};

pub async fn handle_received_messages(
    server: WebsocketServer,
    user: UserId,
    active_connection_data: Arc<Mutex<ActiveConnectionData<WritableWebsocketStream>>>,
    connection: &mut ReadableWebsocketStream,
) -> Result<Disconnected, HandleConnectionError> {
    let mut ratelimit = server.ratelimit.ratelimiter();
    let mut valid = true;
    'receive_a_message: loop {
        if let Some(ping) = connection.2.take() {
            active_connection_data
                .lock()
                .await
                .write
                .0
                .send(tokio_tungstenite::tungstenite::Message::Pong(ping))
                .await
                .ok();
        }
        let mut first = [0u8];
        connection.read_exact(&mut first).await?;
        match first[0] {
            0x00 if valid => {
                // Message: Disconnect Request
                let mut lock = active_connection_data.lock().await;
                lock.write.close().await.ok();
                lock.replaced = true;
                drop(lock);
                let mut cons_lock = server.active_connections.lock().await;
                // if the connection hasn't been replaced yet, remove it from the server state
                if let Some(con) = cons_lock.remove(&user) {
                    if !con.lock().await.replaced {
                        cons_lock.insert(user.clone(), con);
                    }
                }
                drop(cons_lock);
                break 'receive_a_message Ok(Disconnected);
            }
            0xD0 if valid => {
                // Message: Put
                if ratelimit.should_drop_message().await {
                    valid = false;
                    continue 'receive_a_message;
                }
                let coord = if let Some(v) = Coordinate::read_p2encoded(connection).await? {
                    v
                } else {
                    valid = false;
                    continue 'receive_a_message;
                };
                let color = if let Some(v) = Color::read_p2encoded(connection).await? {
                    v
                } else {
                    valid = false;
                    continue 'receive_a_message;
                };
                server.put(coord, color).await;
            }
            0xAF if valid => {
                // Message: Sub
                ratelimit.wait_if_necessary_on_recv(Instant::now()).await;
                // for graphical clients: these messages never get dropped
                let (top_left, bottom_right) = if let (Some(a), Some(b)) = (
                    Coordinate::read_p2encoded(connection).await?,
                    Coordinate::read_p2encoded(connection).await?,
                ) {
                    (a, b)
                } else {
                    valid = false;
                    continue 'receive_a_message;
                };
                let mut lock = active_connection_data.lock().await;
                if lock.replaced {
                    break 'receive_a_message Ok(Disconnected);
                }
                lock.subscribed_area = Area::try_new(top_left, bottom_right);
                lock.has_acted();
            }
            0xFF => {
                valid = true;
                let mut lock = active_connection_data.lock().await;
                if lock.replaced {
                    break 'receive_a_message Ok(Disconnected);
                }
                lock.has_acted();
                continue 'receive_a_message;
            }
            _ => valid = false,
        }
    }
}
