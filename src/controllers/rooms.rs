use std::sync::Arc;
use axum::extract::{Path, State};
use axum::{Extension, Json};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use crate::models::app_state::AppState;
use crate::models::authentication_models::Claims;
use crate::models::room_models::{Participant, ParticipantResponse, ParticipantsWithTeam, Rooms};
use crate::models::player_models::Teams;

pub async fn create_room(State(app_state): State<Arc<AppState>>, Extension(user): Extension<Claims>, Path((team_name, is_strict_mode)) : Path<(String,bool)>) -> impl IntoResponse  {
    /*
        we are going to create a new room and then returning participant_id, and then in front-end, it will immediately
        create a websocket connection with the server, and the server will send all the details to the room if any
        new team has joined everything.
    */
    let team_name_check = Teams::check_team(&team_name);
    if !team_name_check {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid Team Name" })),
        ) ;
    }
    // first creating a room
    match app_state.database_connection.create_room(user.user_id, is_strict_mode).await {
        Ok(room_id) => {
            let participant_id = app_state.database_connection.add_participant(user.user_id, room_id.clone(), team_name.clone()).await.expect("Unable to add participant to the room");
            tracing::info!("created participant_id {} for the room_id {} and team_name {} ", participant_id, room_id, team_name);
            (
                StatusCode::OK,
                Json(json!({
                    "room_id": room_id,
                    "team_name": team_name,
                    "participant_id": participant_id,
                    "message": "Room Created Successfully"
                })),
            )
        },
        Err(err) => {
            tracing::error!("error occurred while creating room");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Internal Server Error" })),
            )
        }
    }
}

pub async fn get_remaining_teams(State(app_state): State<Arc<AppState>>, Extension(user): Extension<Claims>, Path(room_id) : Path<String>) -> impl IntoResponse {
    /*
        once user entered the room-id or he clicks the link, then this api call will executed and if the participant was already
        exists in the room, then it will return the participant_id directly,else returns the list of teams, based on this
        the front-end will render and route accordingly.
    */

    // before returning remaining teams, let's first check whether the user already in the room or not
    let is_already_participant = app_state.database_connection.is_already_participant(user.user_id, room_id.clone()).await;
    let room_status = app_state.database_connection.get_room_status(room_id.clone()).await.expect("Invalid room_id");
    match is_already_participant {
        Ok((participant_id, team_name)) => {
            tracing::info!("participant_id {} is already in the room_id {} and team_name {} ", participant_id, room_id, team_name);

            if room_status == "completed" {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "message": "Room Closed"
                    })),
                ) ;
            }

            (
                StatusCode::OK,
                Json(json!({
                    "room_id": room_id,
                    "team_name": team_name,
                    "participant_id": participant_id,
                    "message": "Already a participant"
                })),
            )
        },
        Err(err) => {
            tracing::info!("a new participant") ;
            let remaining_teams = app_state.database_connection.get_remaining_teams(room_id.clone()).await.expect("Unable to get remaining teams");
            if room_status == "completed" {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "message": "Room Closed"
                    })),
                ) ;
            }else if room_status == "in_progress" {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "message": "Room Closed, Auction Started"
                    })),
                ) ;
            }

            let teams = vec![
                "Mumbai Indians",
                "Chennai Super Kings",
                "Sun Risers Hyderabad",
                "Punjab Kings",
                "Rajasthan Royals",
                "Royal Challengers Bangalore",
                "Kolkata Knight Riders",
                "Delhi Capitals",
                "Lucknow Super Gaints",
                "Gujarat Titans",
            ];

            // let's get remaining teams
            let mut real_remaining_teams = vec![] ;
            for team in teams {
                tracing::info!("the team name was {}", team) ;
                for existed_team in remaining_teams.iter() {
                    if team != existed_team {
                        real_remaining_teams.push(team) ;
                    }
                }
            }
            tracing::info!("remaining teams are {:?}", real_remaining_teams);
            (
                StatusCode::OK,
                Json(json!({
                    "remaining_teams": remaining_teams,
                    "message": "Join with the remaining teams"
                })),
            ) // these are used teams, so we need to make sure return the teams that are not there
        }
    }
}

pub async fn join_room(State(app_state): State<Arc<AppState>>, Extension(user): Extension<Claims>, Path((room_id, team_name)) : Path<(String, String)>) -> impl IntoResponse {
    /*
        it will add the participant to the room and then returns the participant_id, and then front-end will create a websocket
        connection with the server, and the server will send all the details to the room if any new team has joined everything.
    */
    let team_name_check = Teams::check_team(&team_name);
    if !team_name_check {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid Team Name" })),
        ) ;
    }
    // need to add a db check such that no 2 teams will have the same name and also make sure doesn't exceed more than 10 teams
    match app_state.database_connection.add_participant(user.user_id, room_id.clone(), team_name.clone()).await {
        Ok(participant_id) => {
            tracing::info!("created participant_id {} for the room_id {} and team_name {} ", participant_id, room_id, team_name);
            (
                StatusCode::OK,
                Json(json!({
                    "room_id": room_id,
                    "team_name": team_name,
                    "participant_id": participant_id,
                    "message": "Room Created Successfully"
                })),
            )
        },
        /*
            Error Logic need to be clear, where if the same user tries to join we need to say, that you're already
            a participant so we should not insert we need to return the participant_id , so we need to make sure
            this gonna work. where we need to do a change in the db call such that if user already a participant,
            we need to return the participant_id.
        */
        Err(err) => {
            tracing::error!("error occurred while creating room");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Internal Server Error" })),
            )
        }
    }
}

pub async fn get_rooms_played(State(app_state): State<Arc<AppState>>, Extension(user): Extension<Claims>, Path((per_page,room_id,last_record_time_stamp)): Path<(i32,String,String)>) ->  Result<(StatusCode, Json<Vec<Rooms>>),(StatusCode, Json<serde_json::Value>)> {
    // returns the room-ids along with dates
    tracing::info!("getting rooms played by user-id {}", user.user_id);
    // we need to get from the participants not from the rooms table
    let decoded = base64::decode(last_record_time_stamp).expect("decoding the timestamp");
    let timestamp = String::from_utf8(decoded).expect("decoding the timestamp");

    match app_state.database_connection.get_rooms(user.user_id, &timestamp, per_page, &room_id).await {
        Ok(rooms) => {
            tracing::info!("got the rooms played by user-id {}", user.user_id);
            Ok((
                StatusCode::OK,
                Json(rooms)
            ))
        },
        Err(err) => {
            tracing::error!("error occurred while getting rooms played by user-id {}", user.user_id);
            Err((

                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    json!({
                        "message" : "error in getting rooms"
                    })
                )
                ))
        }
    }

}

pub async fn get_participants_room(State(app_state): State<Arc<AppState>>, Extension(user): Extension<Claims>, Path(room_id): Path<String>) -> Result<(StatusCode, Json<Vec<ParticipantResponse>>),(StatusCode, Json<serde_json::Value>)> {
    match app_state.database_connection.get_participants_in_room(room_id.clone()).await {
        Ok(participants) => {
            tracing::info!("got the participants in room {}", room_id);
            Ok((
                StatusCode::OK,
                Json(participants)
            ))
        },
        Err(err) => {
            tracing::error!("error occurred while getting participants in room {}", room_id);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    json!({
                        "message" : "error in getting rooms"
                    })
                )
                ))
        }
    }
}