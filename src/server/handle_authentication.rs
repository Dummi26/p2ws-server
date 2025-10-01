use crate::{
    server::P2Read,
    users::{UserId, Users},
};

pub enum AuthenticationError {
    UsernameNotUtf8,
    NoSuchUser(String),
    InvalidOneTimePassword,
}

pub async fn handle_authentication(
    users: Users,
    connection: &mut (impl P2Read + Unpin),
) -> tokio::io::Result<Result<UserId, AuthenticationError>> {
    let mut buf_len = [0u8; 3];
    connection.read_exact(&mut buf_len).await?;
    let username_len = buf_len[2] as usize + 1;
    let mut buf_message = vec![0u8; username_len + 4];
    connection.read_exact(&mut buf_message).await?;
    Ok(handle_authentication_message(users, username_len, buf_message).await)
}

async fn handle_authentication_message(
    users: Users,
    username_len: usize,
    mut buf_message: Vec<u8>,
) -> Result<UserId, AuthenticationError> {
    let provided_one_time_password = 0u32
        + byte_to_digits(buf_message[username_len + 0]) * 1000000
        + byte_to_digits(buf_message[username_len + 1]) * 10000
        + byte_to_digits(buf_message[username_len + 2]) * 100
        + byte_to_digits(buf_message[username_len + 3]);
    buf_message.truncate(username_len);
    let username =
        String::from_utf8(buf_message).map_err(|_| AuthenticationError::UsernameNotUtf8)?;

    users
        .verify_one_time_password(username, provided_one_time_password)
        .await
}

//given a byte `0xAB`, returns `A * 10 + B`.
// `A` and `B` are capped at `9`, meaning they will always be in the range `0..=9`
fn byte_to_digits(byte: u8) -> u32 {
    9.min((byte & 0xF0) >> 4) as u32 * 10 + 9.min(byte & 0xF) as u32
}

#[test]
fn test_byte_to_digits() {
    assert_eq!(byte_to_digits(0x04), 04);
    assert_eq!(byte_to_digits(0x70), 70);
    assert_eq!(byte_to_digits(0x89), 89);
    assert_eq!(byte_to_digits(0xC3), 93);
}
