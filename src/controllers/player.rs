
use std::sync::Arc;
use axum::extract::{State, Path};
use axum::{Extension, Json};
use axum::response::IntoResponse;
use crate::models::app_state::{AppState};
use crate::models::authentication_models::Claims;
use crate::models::player_models::PlayerBrought;

pub async fn get_team_details(State(app_state): State<Arc<AppState>>, Extension(claims):Extension<Claims>,Path(participant_id): Path<i32>) -> impl IntoResponse {
    tracing::info!("getting team details for participant {}", participant_id);
    /*
        we are going to return
        remaining_balance :
        total_players_brought:
        batsmans:
        bowlers:
        all_rounder:
        wicket_keepers:
    */
}


pub async fn get_team_players(State(app_state): State<Arc<AppState>>, Extension(claims):Extension<Claims>,Path(participant_id): Path<i32>) -> impl IntoResponse {
    tracing::info!("getting team players for participant {}", participant_id);
    /*
        we are going to return each player name, role and their brought price of all players that are brought
    */
}


/*
    ------------------- These 2 Api's are going to be used for getting in the profile and also after these we can 
    -------------------- use above api's for getting more details
    
    next we are going to get the auction rooms_ids along with dates played by each user
    next we are going to get the list of team-names along with participants-ids

*/