
use std::sync::Arc;
use axum::extract::{State, Path};
use axum::Json;
use crate::models::app_state::{AppState};
use crate::models::player_models::PlayerBrought;

pub async fn get_players_brought(State(app_state): State<Arc<AppState>, Path((room_id, participant_id)):Path<(String, i32)> ) -> Json<Result<Vec<PlayerBrought>, String>>{}