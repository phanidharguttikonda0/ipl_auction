use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};

#[derive(Debug, Deserialize)]
pub struct AuthenticationModel {
    pub gmail: String,
    pub google_sid: String,
    pub favorite_team: Option<String>
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub user_id: i32,     // user id or mail
    pub username: String,
    pub gmail: String,
    pub favorite_team: String,
    pub exp: usize,      // expiration time (as UTC timestamp)
}
