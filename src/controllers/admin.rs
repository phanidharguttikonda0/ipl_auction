use std::sync::Arc;
use axum::extract::{Path, State};
use axum::{Json, Router};
use http::StatusCode;
use crate::models::admin_models::AuctionCompletedTasksExecutionModel;
use crate::models::app_state::{AppState, Player};
use crate::services::auction_room::RedisConnection;
use crate::services::background_db_tasks_runner::auction_completed_tasks_executor;

pub async fn get_player(State(app_state): State<Arc<AppState>>, Path(player_id): Path<i32>) -> Result<(StatusCode, Json<Player>), (StatusCode, String)> {
    let redis_connection = RedisConnection::new().await;
    match redis_connection.get_player(player_id, "").await {
        Ok(player) => Ok((StatusCode::OK, Json(player))),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Error while fetching player from redis".to_string()))
    }
}

pub async fn auction_completed_tasks_execution(State(app_state): State<Arc<AppState>>, Json(details): Json<AuctionCompletedTasksExecutionModel>) -> Result<(StatusCode, String), (StatusCode, String)> {
    tracing::info!("getting admin password") ;
    let password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "".to_string());
    if password != details.password {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Invalid Password".to_string()))
    }
    tracing::info!("password was correct now execute the tasks") ;
    auction_completed_tasks_executor(&details.room_id, &app_state).await ;
    Ok((StatusCode::OK, "Successfully Executed".to_string()))
}