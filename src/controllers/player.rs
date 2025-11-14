
use std::sync::Arc;
use axum::extract::{State, Path};
use axum::{Extension, Json};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use crate::models::app_state::{AppState};
use crate::models::authentication_models::Claims;
use crate::models::player_models::{PlayerBrought, PlayerDetails, TeamDetails};

pub async fn get_team_details(State(app_state): State<Arc<AppState>>, Extension(claims):Extension<Claims>,Path(participant_id): Path<i32>) -> Result<(StatusCode, Json<TeamDetails>), (StatusCode, Json<serde_json::Value>)>{
    tracing::info!("getting team details for participant {}", participant_id);
    /*
        we are going to return
        remaining_balance :
        total_players_brought:
        batsman's:
        bowlers:
        all_rounder:
        wicket_keepers:
    */

    // first we need to get the all  these counts and remaning balance ,we need to get from the participants
    let remaining_balance = app_state.database_connection.get_remaining_balance(participant_id).await ;
    match remaining_balance {
        Ok(remaining_balance) =>{
            let result = app_state.database_connection.get_team_details(participant_id).await;
            match result {
                Ok((total_players, batsmans, bowlers, all_rounder)) => {
                    Ok(
                        (
                            StatusCode::OK,
                            Json(TeamDetails {
                                remaining_balance,
                                total_players,
                                total_batsmans: batsmans,
                                total_bowlers: bowlers,
                                all_rounders: all_rounder
                            })
                        )
                    )
                },
                Err(err) => {
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"message" : "server error while fetching team details"}))
                    ))
                }
            }
        },
        Err(err) => {
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message" : "server error while fetching balance"}))
                ))
        }
    }
}


pub async fn get_team_players(State(app_state): State<Arc<AppState>>, Extension(claims):Extension<Claims>,Path(participant_id): Path<i32>) -> Result<(StatusCode, Json<Vec<PlayerDetails>>), (StatusCode, Json<serde_json::Value>)> {
    tracing::info!("getting team players for participant {}", participant_id);
    /*
        we are going to return each player name, role and their brought price of all players that are brought
    */
    match app_state.database_connection.get_team_players(participant_id).await {
        Ok(players) => {
            tracing::info!("got the players of the team");
            Ok((
                StatusCode::OK,
                Json(players)
            ))
        },
        Err(err) => {
            tracing::error!("error occurred while getting team_players") ;
            tracing::error!("{}", err) ;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message" : "server error while fetching team players"}))
            ))
        }
    }
}


/*
    ------------------- These 2 Api's are going to be used for getting in the profile and also after these we can 
    -------------------- use above api's for getting more details
    
    next we are going to get the auction rooms_ids along with dates played by each user
    next we are going to get the list of team-names along with participants-ids

*/