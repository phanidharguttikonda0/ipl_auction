use std::sync::Arc;
use axum::Router;
use axum::routing::get;
use crate::controllers::rooms::{create_room, get_remaining_teams, join_room};
use crate::models::app_state::AppState;

pub fn rooms_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/create-room/{team_name}", get(create_room)) // from the authorization header we can get user-id
        .route("/join-room-get-teams/{room-id}", get(get_remaining_teams))// it returns the remaining teams
        .route("/join-room/{room-id}/{team_name}", get(join_room)) // it returns the participant_id
}