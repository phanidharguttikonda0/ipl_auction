use std::sync::{Arc};
use tokio::sync::RwLock;
use axum::{http, middleware, Router};
use axum::http::{header, Method};
use axum::routing::{get, post};
use dotenv::dotenv;
use tokio::task;
use crate::auction::ws_handler;
use crate::middlewares::authentication::auth_check;
use crate::models::app_state::AppState;
use crate::routes::players_routes::players_routes;
use crate::routes::rooms_routes::rooms_routes;
use crate::services::auction::DatabaseAccess;
use crate::services::auction_room::listen_for_expiry_events;
use crate::services::other::load_players_to_redis;
use tower_http::cors::{CorsLayer, Any};
use tower::ServiceBuilder;
mod models;
mod auction;
mod services;
mod routes;
mod controllers;
mod middlewares;

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
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin([
            "https://ipl-auction.phani.services".parse().unwrap(),
            "http://localhost:3000".parse().unwrap(),
            "http://127.0.0.1:3000".parse().unwrap(),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,   // IMPORTANT
        ])
        .allow_credentials(true)
        .allow_headers([
            header::ACCEPT,
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::HeaderName::from_static("x-requested-with"),
            header::HeaderName::from_static("sec-fetch-site"),
            header::HeaderName::from_static("sec-fetch-mode"),
            header::HeaderName::from_static("sec-fetch-dest"),
        ]).expose_headers([http::header::AUTHORIZATION])
        ;



    // here we are going to load all the players from the database to the redis
    load_players_to_redis(&state.database_connection).await ;
    let app = Router::new()
        .route("/ws/:room_id/:participant_id", get(ws_handler))  // keep it clean
        .merge(
            Router::new()
                .nest("/rooms", rooms_routes())
                .nest("/players", players_routes())
                .route("/continue-with-google", post(controllers::authentication::authentication_handler))
                .layer(cors) // only REST API has CORS
        )
        .with_state(state) ;
    app

}