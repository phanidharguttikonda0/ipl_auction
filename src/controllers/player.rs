
use std::sync::Arc;
use axum::extract::{State, Path};
use axum::{Extension, Json};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use crate::models::app_state::{AppState, PoolPlayer};
use crate::models::authentication_models::Claims;
use crate::models::player_models::{PlayerBrought, PlayerDetails, SoldPlayerOutput, TeamDetails, UnSoldPlayerOutput};

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
    let result = app_state.database_connection.get_remaining_balance_and_room_status(participant_id).await ;
    match result {
        Ok((remaining_balance, room_status)) =>{
            let result = app_state.database_connection.get_team_details(participant_id, &room_status).await;
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


pub async fn get_team_players(State(app_state): State<Arc<AppState>>, Extension(claims):Extension<Claims>,Path((participant_id, status)): Path<(i32, String)>) -> Result<(StatusCode, Json<Vec<PlayerDetails>>), (StatusCode, Json<serde_json::Value>)> {
    tracing::info!("getting team players for participant {}", participant_id);
    /*
        we are going to return each player name, role and their brought price of all players that are brought
        -> here we are going to need to get status , based on the status we are going to call from the
        completed_rooms_sold_players or from the just sold_players
    */
    tracing::info!("the status of the room was {}", status) ;
    match app_state.database_connection.get_team_players(participant_id, &status).await {
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

pub async fn get_sold_players(State(app_state): State<Arc<AppState>>,Path((room_id, page_no, offset)): Path<(String, i32, i32)>) -> Result<(StatusCode, Json<Vec<SoldPlayerOutput>>), (StatusCode, Json<serde_json::Value>)> {
    tracing::info!("getting sold players for room {}", room_id);
    
    match app_state.database_connection.get_sold_players(room_id,page_no,offset).await { 
        Ok(result) => {
            Ok((
                StatusCode::OK,
                Json(result)
                ))
        },
        Err(err) => {
            tracing::error!("error occurred while getting sold players") ;
            tracing::error!("{}", err) ;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message" : "server error while fetching sold players"}))
            ))
        }
    }
}

pub async fn get_unsold_players(State(app_state): State<Arc<AppState>>,Path((room_id, page_no, offset)): Path<(String, i32, i32)>) -> Result<(StatusCode, Json<Vec<UnSoldPlayerOutput>>), (StatusCode, Json<serde_json::Value>)> {
    tracing::info!("getting unsold players for room {}", room_id);
    
    match app_state.database_connection.get_unsold_players(room_id, page_no, offset).await { 
        Ok(result ) => {
            Ok((
                StatusCode::OK,
                Json(result)
                ))
        },
        Err(err) => {
            tracing::error!("error occurred while getting unsold players") ;
            tracing::error!("{}", err) ;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message" : "server error while fetching unsold players"}))
            ))
        }
    }
}


pub async fn get_players_from_pool(State(app_state): State<Arc<AppState>>, Path(pool_no): Path<i16>) -> Result<(StatusCode, Json<Vec<PoolPlayer>>), (StatusCode, Json<serde_json::Value>)> {
    tracing::info!("get players from the pool api was called") ;
    match app_state.redis_connection.get_players_by_pool(pool_no).await { 
        Ok(players) => {
            Ok((
                StatusCode::OK,
                Json(players)
                ))
        },
        Err(err) => {
            tracing::error!("error occurred while getting players from pool") ;
            tracing::error!("{}", err) ;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"message" : "server error while fetching players from pool"}))
                ))
        }
    }
}