use std::fmt::format;
use std::net::SocketAddr;
use std::sync::{Arc};
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use axum::{http, middleware, Router};
use axum::extract::ws::Message;
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
use crate::controllers::others::feed_back;
use crate::controllers::profile::update_favorite_team;
use crate::models::background_db_tasks::{DBCommandsAuction, DBCommandsAuctionRoom};
use crate::routes::admin_routes::admin_routes;
use crate::services::background_db_tasks_runner::{background_task_executor_outside_auction_db_calls, background_tasks_executor};
use tracing_appender::non_blocking;
use crate::observability::http_tracing::http_trace_layer;
use crate::observability::metrics::init_metrics;

mod models;
mod auction;
mod services;
mod routes;
mod controllers;
mod middlewares;
mod observability;

#[tokio::main]
async fn main() {
    // creating a writer to make all logs async and non blocking
    let tracing_gaurd = observability::tracing::init_tracing();
    // this tracing_gaurd should be alive until the server is live.
    tracing::info!("Initializing Metrics");
    init_metrics();
    
    dotenv().ok();
    let port = std::env::var("PORT").unwrap_or("4545".to_string());
    tracing::info!("Starting server on port {}", port);
    tracing::info!("creating TCP listener") ;
    let is_prod = std::env::var("PROD").unwrap_or("false".to_string());
    let tcp_listener ;
    if is_prod == "true" {
        tcp_listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();
    }else{
        tcp_listener = tokio::net::TcpListener::bind(format!("[::]:{}", port)).await.unwrap();
    }
    let app = routes().await;
    axum::serve(tcp_listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();

}


async fn routes() -> Router {
    let (tx, mut rx) = mpsc::unbounded_channel::<DBCommandsAuctionRoom>();
    let (tx_outside_auction_d, mut rx_outside_auction_d) = mpsc::unbounded_channel::<DBCommandsAuction>();
    let state = Arc::new(
        AppState {
            rooms: Arc::new(RwLock::new(std::collections::HashMap::new())),
            database_connection: Arc::from(DatabaseAccess::new().await),
            auction_room_database_task_executor: tx,
            database_task_executor: tx_outside_auction_d,
            redis_connection: Arc::new(services::auction_room::RedisConnection::new().await)
        }
    ) ;
    let redis_url = std::env::var("REDIS_URL").unwrap();
    let state_ = state.clone();
    task::spawn(async move {
        let state = state_;
        loop {
            if let Err(e) = listen_for_expiry_events(&format!("redis://{}:6379/", redis_url), &state).await {
                tracing::error!("Redis expiry listener failed: {:?}", e);
            }
            tracing::warn!("🔁 Restarting listener in 2 seconds...");
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
    
    tracing::info!("tracing of the background tasks executor for inside auction room db tasks was called") ;
    let state_ = state.clone() ;
    tokio::spawn(async move {
        background_tasks_executor(state_, rx).await ;
    }) ;
    
    tracing::info!("tracing of the background tasks executor was called") ;
    let state_ = state.clone() ;
    tokio::spawn(async move {
       background_task_executor_outside_auction_db_calls(state_, rx_outside_auction_d).await ; 
    });

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin([
            "https://ipl-auction.phani.services".parse().unwrap(),
            "http://localhost:3000".parse().unwrap(),
            "http://127.0.0.1:3000".parse().unwrap(),
            "https://v0-ipl-auction-1ma5hg2bh-phanidharguttikonda0s-projects.vercel.app".parse().unwrap()
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
        .route("/health", get(|| {
            tracing::info!("Health check passed") ;
            async { Ok::<_, std::convert::Infallible>("Health check passed") }
        }))
        .nest("/rooms", rooms_routes())
        .nest("/players", players_routes())
        .route("/feedback", post(feed_back).layer(middleware::from_fn(middlewares::authentication::auth_check)))
        .route("/update-favorite-team/{new_team}", get(update_favorite_team)).layer(middleware::from_fn(middlewares::authentication::auth_check))
        .route("/continue-with-google", post(controllers::authentication::authentication_handler))
        .layer(cors) // <-- apply globally
        .route("/ws/{room_id}/{participant_id}", get(ws_handler))
        .nest("/admin", admin_routes())
        .layer(http_trace_layer())
        .with_state(state);

    app

}