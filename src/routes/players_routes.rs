use std::sync::Arc;
use axum::{middleware, Router};
use axum::routing::get;
use crate::controllers::player::{get_sold_players, get_team_details, get_team_players, get_unsold_players};
use crate::models::app_state::AppState;

pub fn players_routes() -> Router<Arc<AppState>>{
    Router::new()
        .route("/get-team-details/{participant_id}", get(get_team_details))
        .route("/get-team-players/{participant_id}", get(get_team_players))
        .route("/get-unsold-players/{room_id}/{page_no}/{offset}", get(get_unsold_players))
        .route("/get-sold-players/{room_id}/{page_no}/{offset}", get(get_sold_players))
        .layer(middleware::from_fn(crate::middlewares::authentication::auth_check))
}

/*
    * here for sold and unsold players we are going to use page_no, for each page we are going to get last 10 players only
    * so we are going to use offset and limit in the query

    for unsold players we are going to return base_price, name, id and role.
    for sold players we are going to return the team_brought, sold_price, team_name, player_name, player_id, role
*/