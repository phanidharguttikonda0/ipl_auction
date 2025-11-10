use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AuthenticationModel {
    gmail: String,
    sid: String,
    favorite_team: Option<String>,
}
