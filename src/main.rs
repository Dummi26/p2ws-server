use std::{process::ExitCode, time::Duration};

use crate::{ratelimit::RatelimitSettings, server::WebsocketServer, users::Users};

mod data;
mod one_time_password;
mod protocol;
mod ratelimit;
mod server;
mod users;

#[tokio::main]
async fn main() -> ExitCode {
    let time_per_message = Duration::from_millis(1);
    let ratelimit = RatelimitSettings::new(time_per_message)
        .allow_bursts(10)
        .drop_instead_of_blocking();

    let users = Users::from_toml(&tokio::fs::read_to_string("users.toml").await.unwrap()).unwrap();

    let Err(e) = WebsocketServer::new(ratelimit)
        .accept_connections("127.0.0.1:8080", users)
        .await;
    eprintln!("Error accepting connections: {e:?}");
    ExitCode::FAILURE
}
