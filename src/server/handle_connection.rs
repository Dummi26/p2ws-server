use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    server::{
        P2Write, WebsocketServer,
        connection_data::ActiveConnectionData,
        connections::{ReadableWebsocketStream, WritableWebsocketStream},
        handle_authentication::{AuthenticationError, handle_authentication},
        handle_received_messages::handle_received_messages,
    },
    users::Users,
};

pub struct Disconnected;
pub enum HandleConnectionError {
    IoError(tokio::io::Error),
    AuthenticationError(AuthenticationError),
}

pub async fn handle_connection(
    users: Users,
    server: WebsocketServer,
    mut read: ReadableWebsocketStream,
    active_connection_data: Arc<Mutex<ActiveConnectionData<WritableWebsocketStream>>>,
) -> Result<Disconnected, HandleConnectionError> {
    match handle_authentication(users, &mut read).await {
        Ok(Ok(user)) => {
            let mut cons_lock = server.active_connections.lock().await;
            if let Some(previous_connection) =
                cons_lock.insert(user.clone(), Arc::clone(&active_connection_data))
            {
                drop(cons_lock);
                let mut previous_connection = previous_connection.lock().await;
                previous_connection.replaced = true;
                previous_connection.write.close().await.ok();
            } else {
                drop(cons_lock);
            }

            eprintln!("User {user:?} has joined.");
            handle_received_messages(server, user, active_connection_data, &mut read).await
        }
        Ok(Err(e)) => Err(HandleConnectionError::AuthenticationError(e)),
        Err(e) => Err(HandleConnectionError::IoError(e)),
    }
}

impl From<tokio::io::Error> for HandleConnectionError {
    fn from(value: tokio::io::Error) -> Self {
        Self::IoError(value)
    }
}
