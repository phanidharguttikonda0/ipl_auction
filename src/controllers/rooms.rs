use std::sync::Arc;
use axum::extract::{Path, State};
use axum::{Extension, Json};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use crate::models::app_state::AppState;
use crate::models::authentication_models::Claims;
use crate::models::room_models::{Participant, ParticipantsWithTeam, Teams};

pub async fn create_room(State(app_state): State<Arc<AppState>>, Extension(user): Extension<Claims>, Path(team_name) : Path<String>) -> impl IntoResponse  {
    /*
        we are going to create a new room and then returning participant_id, and then in front-end, it will immediately
        create a websocket connection with the server, and the server will send all the details to the room if any
        new team has joined everything.
    */

    // first creating a room
    match app_state.database_connection.create_room(user.user_id).await {
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
        once user entered the room-id or he clicks the link , then this api call will executed and if the participant was already
        exists in the room, then it will return the participant_id directly,else returns the list of teams, based on this
        the front-end will render and route accordingly.
    */

    // get remaining teams only if the status was in not_started and also he was not a participant that he already joined
    match app_state.database_connection.get_room_status(room_id.clone()).await {
        Ok(status) => {
            if status == "not_started" {

            }
         },
        Err(err) => {
            tracing::error!("error occurred while getting room status");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Internal Server Error" })),
            ) // we need to check whether the room-id doesn't exists is that throwing the error or anything else
        }
    }

}

pub async fn join_room(State(app_state): State<Arc<AppState>>, Extension(user): Extension<Claims>, Path((room_id, team_name)) : Path<(String, String)>) -> Json<Result<ParticipantsWithTeam, String>> {
    /*
        it will add the participant to the room and then returns the participant_id, and then front-end will create a websocket
        connection with the server, and the server will send all the details to the room if any new team has joined everything.
    */
    Json(Err("Not implemented".to_string()))
}