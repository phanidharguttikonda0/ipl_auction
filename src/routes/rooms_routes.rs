use std::sync::Arc;
use axum::Router;
use axum::routing::get;
use crate::controllers::rooms::{create_room, get_participants_room, get_remaining_teams, get_rooms_played, join_room};
use crate::models::app_state::AppState;
use axum::middleware ;

pub fn rooms_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/create-room/{team_name}", get(create_room)) // from the authorization header we can get user-id
        .route("/join-room-get-teams/{room-id}", get(get_remaining_teams))// it returns the remaining teams
        .route("/join-room/{room-id}/{team_name}", get(join_room)) // it returns the participant_id
        .route("/get-auctions-played", get(get_rooms_played)) // it going to return the list of room-ids participated by the user and along with date
        .route("/get-participants/{room_id}", get(get_participants_room)) // it going to return the list of participants-id along with the team-name, using these participant_ids to get the team details and player details
        .layer(middleware::from_fn(crate::middlewares::authentication::auth_check))

}