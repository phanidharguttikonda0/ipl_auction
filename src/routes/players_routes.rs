use std::sync::Arc;
use axum::Router;
use axum::routing::get;
use crate::models::app_state::AppState;

pub fn players_routes() -> Router<Arc<AppState>>{
    Router::new()
        .route("get-players-brought/{room-id}/{participant_id}", get("get-players-brought"))
}