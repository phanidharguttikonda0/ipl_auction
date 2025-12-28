use std::sync::Arc;
use axum::Router;
use axum::routing::{get, post};
use crate::controllers::admin::{auction_completed_tasks_execution, get_player};
use crate::models::app_state::AppState;

pub fn admin_routes() -> Router<Arc<AppState>>{
    Router::new()
        .route("/get-redis-player/{player_id}", get(get_player))
        .route("/auction_completed_tasks_execution", post(auction_completed_tasks_execution))
}