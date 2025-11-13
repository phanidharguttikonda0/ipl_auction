use std::sync::Arc;
use axum::{middleware, Router};
use axum::routing::get;
use crate::controllers::player::{get_team_details, get_team_players};
use crate::models::app_state::AppState;

pub fn players_routes() -> Router<Arc<AppState>>{
    Router::new()
        .route("/get-team-details/{participant_id}", get(get_team_details))
        .route("/get-team-players/{participant_id}", get(get_team_players))
        .layer(middleware::from_fn(crate::middlewares::authentication::auth_check))
}