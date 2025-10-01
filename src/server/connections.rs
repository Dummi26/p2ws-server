use std::{collections::VecDeque, convert::Infallible};

use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio_tungstenite::WebSocketStream;

use crate::{
    server::{
        P2Read, P2Write, Server,
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
            ReadableWebsocketStream(read, Default::default()),
            WritableWebsocketStream(write, Default::default()),
        );
        match handle_connection(users, server, read, write).await {
            Ok(Disconnected) => {}
            Err(_e) => {}
        }
    }
}

pub struct ReadableWebsocketStream(SplitStream<WebSocketStream<TcpStream>>, VecDeque<u8>);
pub struct WritableWebsocketStream(
    SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>,
    VecDeque<u8>,
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
        if self.1.len() > 1024 {
            self.flush().await
        } else {
            Ok(())
        }
    }

    async fn flush(&mut self) -> tokio::io::Result<()> {
        self.0
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                tokio_tungstenite::tungstenite::Bytes::from_iter(self.1.drain(..)),
            ))
            .await
            .map_err(|e| std::io::Error::other(e))?;
        self.0.flush().await.map_err(|e| std::io::Error::other(e))
    }

    async fn close(&mut self) -> tokio::io::Result<()> {
        self.0.close().await.map_err(|e| std::io::Error::other(e))
    }
}
