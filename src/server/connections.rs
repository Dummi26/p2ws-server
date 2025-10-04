use std::{collections::VecDeque, convert::Infallible, sync::Arc, time::Duration};

use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::Mutex,
    task::{AbortHandle, JoinHandle},
    time::sleep,
};
use tokio_tungstenite::WebSocketStream;

use crate::{
    server::{
        DELAY_BETWEEN_WEBSOCKET_MESSAGES, P2Read, P2Write, Server,
        connection_data::ActiveConnectionData,
        handle_connection::{Disconnected, handle_connection},
    },
    users::Users,
};

#[derive(Debug)]
pub enum AcceptConnectionsError {
    CouldNotAcceptMoreConnections(tokio::io::Error),
    CouldNotAcceptAnyConnections(tokio::io::Error),
}

pub type WebsocketServer = Server<WritableWebsocketStream>;

impl WebsocketServer {
    pub async fn accept_connections(
        self,
        bind_addr: impl ToSocketAddrs,
        users: Users,
    ) -> Result<Infallible, AcceptConnectionsError> {
        let socket = TcpListener::bind(bind_addr).await.unwrap();
        let mut accepted_connections_counter: u128 = 0;
        loop {
            match socket.accept().await {
                Ok((connection, _)) => {
                    accepted_connections_counter = accepted_connections_counter.saturating_add(1);
                    let users = users.clone();
                    let server = self.clone();
                    tokio::task::spawn(handle_tcp_connection(connection, users, server));
                }
                Err(e) => {
                    return if accepted_connections_counter == 0 {
                        Err(AcceptConnectionsError::CouldNotAcceptAnyConnections(e))
                    } else {
                        Err(AcceptConnectionsError::CouldNotAcceptMoreConnections(e))
                    };
                }
            }
        }
    }
}

async fn handle_tcp_connection(connection: TcpStream, users: Users, server: WebsocketServer) {
    if let Ok(connection) = tokio_tungstenite::accept_async(connection).await {
        let (write, read) = connection.split();
        let (read, write) = (
            ReadableWebsocketStream(read, Default::default(), None),
            Arc::new(Mutex::new(ActiveConnectionData::new(
                WritableWebsocketStream(write, Default::default(), None, None),
            ))),
        );
        write.try_lock().unwrap().write.3 = Some(Arc::clone(&write));
        match handle_connection(users, server, read, write).await {
            Ok(Disconnected) => {}
            Err(_e) => {}
        }
    }
}

pub struct ReadableWebsocketStream(
    SplitStream<WebSocketStream<TcpStream>>,
    VecDeque<u8>,
    pub Option<tokio_tungstenite::tungstenite::Bytes>,
);
pub struct WritableWebsocketStream(
    pub SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>,
    VecDeque<u8>,
    Option<JoinHandle<tokio::io::Result<()>>>,
    Option<Arc<Mutex<ActiveConnectionData<Self>>>>,
);

impl P2Read for ReadableWebsocketStream {
    async fn read_exact(&mut self, mut buf: &mut [u8]) -> tokio::io::Result<()> {
        if !self.1.is_empty() {
            // take as many bytes as possible from `self.1` and put them into `buf` immediately
            let (queue1, queue2) = self.1.as_slices();
            if buf.len() >= queue1.len() {
                buf[0..queue1.len()].copy_from_slice(queue1);
                buf = &mut buf[queue1.len()..];
                let mut taken = queue2.len().min(buf.len());
                buf[0..taken].copy_from_slice(&queue2[0..taken]);
                buf = &mut buf[taken..];
                taken += queue1.len();
                self.1.drain(0..taken);
            } else {
                buf.copy_from_slice(&queue1[0..buf.len()]);
                self.1.drain(0..buf.len());
                buf = &mut [];
            }
        }
        while buf.len() > 0 {
            match self.0.next().await {
                Some(Ok(msg)) => {
                    if msg.is_ping() {
                        self.2 = Some(msg.into_data());
                        continue;
                    }
                    let bytes = msg.into_data();
                    if buf.len() < bytes.len() {
                        buf.copy_from_slice(&bytes[0..buf.len()]);
                        self.1.extend(&bytes[buf.len()..]);
                        buf = &mut [];
                    } else {
                        buf[0..bytes.len()].copy_from_slice(&bytes);
                        buf = &mut buf[bytes.len()..];
                    }
                }
                Some(Err(e)) => return Err(std::io::Error::other(e)),
                None => return Err(std::io::ErrorKind::UnexpectedEof.into()),
            }
        }
        Ok(())
    }
}

impl P2Write for WritableWebsocketStream {
    async fn write_all(&mut self, buf: &[u8]) -> tokio::io::Result<()> {
        self.1.extend(buf);
        Ok(())
    }

    async fn flush(&mut self) -> tokio::io::Result<()> {
        let result = if self.2.as_ref().is_some_and(|task| task.is_finished()) {
            self.2.take().unwrap().await.unwrap()
        } else {
            Ok(())
        };
        if self.2.is_none() {
            let con = self.3.as_ref().map(Arc::clone).unwrap();
            self.2 = Some(tokio::task::spawn(async move {
                sleep(DELAY_BETWEEN_WEBSOCKET_MESSAGES).await;
                let mut con = con.lock().await;
                if con.write.1.len() > 0 {
                    con.write.force_flush().await
                } else {
                    Ok(())
                }
            }));
        }
        result
    }

    async fn close(&mut self) -> tokio::io::Result<()> {
        if let Some(task) = self.2.take() {
            task.abort();
        }
        self.0.close().await.map_err(|e| std::io::Error::other(e))
    }
}
impl WritableWebsocketStream {
    pub async fn force_flush(&mut self) -> tokio::io::Result<()> {
        self.0
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                tokio_tungstenite::tungstenite::Bytes::from_iter(self.1.drain(..)),
            ))
            .await
            .map_err(|e| std::io::Error::other(e))?;
        self.0.flush().await.map_err(|e| std::io::Error::other(e))
    }
}
