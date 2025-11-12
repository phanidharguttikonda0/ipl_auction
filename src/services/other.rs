use chrono::{Duration, Utc};
use sqlx::{Pool, Postgres};
use crate::services::auction::DatabaseAccess;
use crate::services::auction_room::RedisConnection;
use jsonwebtoken::{encode, Header, EncodingKey};
use crate::models::authentication_models::Claims;

pub async fn load_players_to_redis(conn: &DatabaseAccess) {
    let mut redis_connection = RedisConnection::new();
    // need to load players from the postgres database
    let players = conn.get_players().await.unwrap();
    redis_connection.await.load_players_to_redis(players).await.unwrap();
    tracing::info!("loading players to redis successful") ;
}


pub fn create_authorization_header(user_id: i32, username: String, gmail: String, favorite_team: String) -> Result<String, jsonwebtoken::errors::Error> {
    let secret_key : String = std::env::var("JWT_SECRET").unwrap();
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(7))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        user_id: user_id.to_owned(),
        username,
        gmail, favorite_team,
        exp: expiration,
    };

    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret_key.as_ref()))
}