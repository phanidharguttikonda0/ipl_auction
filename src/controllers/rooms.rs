use std::sync::Arc;
use axum::extract::{Path, State};
use axum::{Extension, Json};
use crate::models::app_state::AppState;
use crate::models::authentication_models::Claims;
use crate::models::room_models::{Participant, Teams};

pub async fn create_room(State(app_state): State<Arc<AppState>>, Path(team_name) : Path<String>, Extension(user): Extension<Claims>) -> Json<Result<Participant, String>>  {
    /*
        we are going to create a new room and then returning participant_id, and then in front-end, it will immediately 
        create a websocket connection with the server, and the server will send all the details to the room if any
        new team has joined everything.
    */
    Json(Err("Not implemented".to_string()))
}

pub async fn get_remaining_teams(State(app_state): State<Arc<AppState>>, Path(team_name) : Path<String>, Extension(user): Extension<Claims>) -> Json<Result<Teams, String>> {
    /*
        once user entered the room-id or he clicks the link , then this api call will executed and if the participant was already
        exists in the room, then it will return the participant_id directly,else returns the list of teams, based on this
        the front-end will render and route accordingly.
    */
    Json(Err("Not implemented".to_string()))
}

pub async fn join_room(State(app_state): State<Arc<AppState>>, Path((room_id, team_name)) : Path<(String, String)>, Extension(user): Extension<Claims>) -> Json<Result<Participant, String>> {
    /*
        it will add the participant to the room and then returns the participant_id, and then front-end will create a websocket
        connection with the server, and the server will send all the details to the room if any new team has joined everything.
    */
    Json(Err("Not implemented".to_string()))
}