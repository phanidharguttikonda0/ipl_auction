use std::sync::Arc;
use axum::extract::{Path, State};
use axum::{Json, Router};
use http::StatusCode;
use crate::models::app_state::{AppState, Player};
use crate::services::auction_room::RedisConnection;

pub async fn get_player(State(app_state): State<Arc<AppState>>, Path(player_id): Path<i32>) -> Result<(StatusCode, Json<Player>), (StatusCode, String)> {
    let mut redis_connection = RedisConnection::new().await;
    match redis_connection.get_player(player_id).await {
        Ok(player) => Ok((StatusCode::OK, Json(player))),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Error while fetching player from redis".to_string()))
    }
}