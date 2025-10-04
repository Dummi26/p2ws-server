mod connection_data;
mod connection_traits;
mod connections;
mod handle_authentication;
mod handle_connection;
mod handle_received_messages;

pub use connection_traits::*;
pub use connections::WebsocketServer;

use tokio::sync::Mutex;

use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    data::{Color, Coordinate},
    protocol::P2Encodable,
    ratelimit::RatelimitSettings,
    server::connection_data::ActiveConnectionData,
    users::UserId,
};

pub use handle_authentication::AuthenticationError;

const DELAY_BETWEEN_WEBSOCKET_MESSAGES: Duration = Duration::from_millis(10);

/// Shared state, can be shared using `.clone()`.
pub struct Server<W: P2Write + Unpin> {
    ratelimit: RatelimitSettings,
    /// NOTE: You may not wait for a lock on this Mutex while holding a lock to a Mutex
    /// which is (or was) contained in the HashMap, as this may result in a deadlock.
    /// Always lock this Mutex before you lock an inner Mutex, if you have to hold two locks at the same time.
    active_connections: Arc<Mutex<HashMap<UserId, Arc<Mutex<ActiveConnectionData<W>>>>>>,
}

impl<W: P2Write + Unpin> Server<W> {
    pub fn new(ratelimit: RatelimitSettings) -> Self {
        Self {
            ratelimit,
            active_connections: Default::default(),
        }
    }

    async fn put(&self, coord: Coordinate, color: Color) {
        for con in self.active_connections.lock().await.values() {
            let mut con = con.lock().await;
            if !con.replaced && con.subscribed_area.is_some_and(|area| area.contains(coord)) {
                if !(con.write.write_all(&[0xFF, 0x01]).await.is_ok()
                    && coord.write_p2encoded(&mut con.write).await.is_ok()
                    && color.write_p2encoded(&mut con.write).await.is_ok()
                    && con.write.flush().await.is_ok())
                {
                    con.replaced = true;
                }
            }
        }
    }
}

impl<W: P2Write + Unpin> Clone for Server<W> {
    fn clone(&self) -> Self {
        Self {
            ratelimit: self.ratelimit,
            active_connections: Arc::clone(&self.active_connections),
        }
    }
}
