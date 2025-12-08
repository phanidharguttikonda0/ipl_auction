use std::collections::HashMap;
use std::sync::{Arc};
use tokio::sync::RwLock;
use axum::extract::ws::Message;
use redis_derive::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use crate::models::background_db_tasks::{DBCommandsAuction, DBCommandsAuctionRoom};
use crate::services::auction::DatabaseAccess;
use crate::services::auction_room::RedisConnection;

#[derive(Debug,Clone)]
pub struct AppState {
    pub rooms: Arc<RwLock<HashMap<String, Vec<(i32, tokio::sync::mpsc::UnboundedSender<Message>)>>>>, // i32 is participant id
    pub database_connection: Arc<DatabaseAccess>,
    pub auction_room_database_task_executor: tokio::sync::mpsc::UnboundedSender<DBCommandsAuctionRoom>,
    pub database_task_executor: tokio::sync::mpsc::UnboundedSender<DBCommandsAuction>,
    pub redis_connection: Arc<RedisConnection>
}

#[derive(Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize, sqlx::FromRow)]
pub struct Player {
    pub id: i32,
    pub name: String,
    pub base_price: f32,
    pub country: String,
    pub role: String,
    pub previous_team: String,
    pub is_indian: bool,
}