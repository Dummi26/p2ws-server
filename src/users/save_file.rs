use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use tokio::sync::Mutex;

use crate::{
    one_time_password::OneTimePasswordGenerator,
    users::{UserData, UserId, Users},
};

pub fn parse(file_content: &str) -> Result<Users, toml::de::Error> {
    #[derive(Deserialize)]
    struct DeUsersFile {
        otp: DeOtpMode,
    }
    #[derive(Deserialize)]
    enum DeOtpMode {
        Static(u32),
    }

    let de = toml::from_str::<HashMap<String, DeUsersFile>>(file_content)?;

    Ok(Users {
        users: Arc::new(Mutex::new(
            de.into_iter()
                .map(|(user, data)| {
                    (
                        UserId(user),
                        UserData {
                            one_time_password: match data.otp {
                                DeOtpMode::Static(pin) => OneTimePasswordGenerator::Static(pin),
                            },
                        },
                    )
                })
                .collect(),
        )),
    })
}
