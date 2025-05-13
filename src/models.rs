use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct WLRegister {
    pub serial_number: String,
    pub email: String,
    pub account_created_at: Option<chrono::DateTime<Utc>>,

    pub username: String,
    pub password: String,

    pub device_name: String,
}

