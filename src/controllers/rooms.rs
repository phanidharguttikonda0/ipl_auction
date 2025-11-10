use std::sync::Arc;
use axum::extract::{Path, State};
use axum::{Extension, Json};
use crate::models::app_state::AppState;
use crate::models::room_models::{Participant, Teams};

pub async fn create_room(State(app_state): State<Arc<AppState>>, Path(team_name) : Path<String>, Extension(user_id): Extension<i32>) -> Json<Result<Participant, String>>  {
    
}

pub async fn get_remaining_teams(State(app_state): State<Arc<AppState>>, Path(team_name) : Path<String>, Extension(user_id): Extension<i32>) -> Json<Result<Teams, String>> {}

pub async fn join_room(State(app_state): State<Arc<AppState>>, Path((room_id, team_name)) : Path<(String, String)>, Extension(user_id): Extension<i32>) -> Json<Result<Participant, String>> {}