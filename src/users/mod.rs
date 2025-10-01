mod save_file;

use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::{one_time_password::OneTimePasswordGenerator, server::AuthenticationError};

/// Contains the users who are able to authenticate.
///
/// Can be shared using `.clone()`, as its contains `Arc<Mutex<_>>`.
#[derive(Clone)]
pub struct Users {
    users: Arc<Mutex<HashMap<UserId, UserData>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(String);

pub struct UserData {
    one_time_password: OneTimePasswordGenerator,
}

impl Users {
    pub fn new() {}

    pub async fn verify_one_time_password(
        &self,
        username: String,
        provided_one_time_password: u32,
    ) -> Result<UserId, AuthenticationError> {
        let user_id = UserId(username);
        match self.users.lock().await.get_mut(&user_id) {
            Some(user) => {
                if user.one_time_password.get_current_otp().is_some_and(
                    |expected_one_time_password| {
                        expected_one_time_password == provided_one_time_password
                    },
                ) {
                    Ok(user_id)
                } else {
                    Err(AuthenticationError::InvalidOneTimePassword)
                }
            }
            None => Err(AuthenticationError::NoSuchUser(user_id.0)),
        }
    }

    pub fn from_toml(toml: &str) -> Result<Users, toml::de::Error> {
        save_file::parse(toml)
    }
}
