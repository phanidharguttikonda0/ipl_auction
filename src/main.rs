use std::sync::{Arc};
use tokio::sync::RwLock;
use axum::Router;
use axum::routing::get;
use dotenv::dotenv;
use tokio::task;
use crate::auction::ws_handler;
use crate::models::app_state::AppState;
use crate::services::auction::DatabaseAccess;
use crate::services::auction_room::listen_for_expiry_events;
use crate::services::other::load_players_to_redis;

mod models;
mod auction;
mod services;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenv().ok();
    let port = std::env::var("PORT").unwrap_or("4545".to_string());
    tracing::info!("Starting server on port {}", port);
    tracing::info!("creating TCP listener") ;
    let tcp_listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();
    let app = routes().await;
    axum::serve(tcp_listener, app).await.unwrap();

}


async fn routes() -> Router {
    let state = Arc::new(
        AppState {
            rooms: Arc::new(RwLock::new(std::collections::HashMap::new())),
            database_connection: Arc::from(DatabaseAccess::new().await),
        }
    ) ;
    let redis_url = std::env::var("REDIS_URL").unwrap();
    let state_ = state.clone();
    task::spawn(async move {
        if let Err(e) = listen_for_expiry_events(&format!("redis://{}:6379/", redis_url), state_).await {
            tracing::error!("Redis expiry listener failed: {:?}", e);
        }
    });


    // here we are going to load all the players from the database to the redis
    load_players_to_redis(&state.database_connection).await ;
    Router::new()
        .route("/ws/{room_id}/{participant_id}", get(ws_handler)) // for the initial handshake it's just a GET request, after handshake the client and server exchange the data via websocket not any more http
        .with_state(state)
}