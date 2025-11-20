
use axum::extract::{Path, State};
use axum::Router;
use http::StatusCode;
use crate::models::app_state::Player;
use crate::services::auction_room::RedisConnection;

pub async fn get_player(Path(player_id): Path<i32>) -> Result<(StatusCode, Player), (StatusCode, String)> {
    let mut redis_connection = RedisConnection::new().await;
    match redis_connection.get_player(player_id) { 
        Ok(player) => Ok((StatusCode::OK, player)),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Error while fetching player from redis".to_string()))
    }
}