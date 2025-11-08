use std::sync::{Arc, RwLock};
use axum::Router;
use axum::routing::get;
use crate::auction::ws_handler;
use crate::models::app_state::AppState;
use crate::services::auction::DatabaseAccess;
use crate::services::other::load_players_to_redis;

mod models;
mod auction;
mod services;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let port = std::env::var("PORT").unwrap_or("4545".to_string());
    tracing::info!("Starting server on port {}", port);
    tracing::info!("creating TCP listener") ;
    let tcp_listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();
    let app = routes().await;
    axum::serve(tcp_listener, app).await.unwrap();

}


async fn routes() -> Router {
    let state = AppState {
        rooms: Arc::new(RwLock::new(std::collections::HashMap::new())),
        database_connection: DatabaseAccess::new(),
    } ;
    // here we are going to load all the players from the database to the redis
    load_players_to_redis(&state.database_connection).await ;
    Router::new()
        .route("/ws/{room_id}/{participant_id}", get(ws_handler)) // for the initial handshake it's just a GET request, after handshake the client and server exchange the data via websocket not any more http
        .with_state(Arc::new(state))
}