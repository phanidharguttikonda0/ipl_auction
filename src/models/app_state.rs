use std::collections::HashMap;
use std::sync::{Arc};
use tokio::sync::RwLock;
use axum::extract::ws::Message;
use redis_derive::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use crate::services::auction::DatabaseAccess;
#[derive(Debug,Clone)]
pub struct AppState {
    pub rooms: Arc<RwLock<HashMap<String, Vec<(i32, tokio::sync::mpsc::UnboundedSender<Message>)>>>>, // i32 is participant id
    pub database_connection: Arc<DatabaseAccess>,
}

#[derive(Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize, sqlx::FromRow)]
pub struct Player {
    pub id: i32,
    pub name: String,
    pub base_price: f32,
    pub country: String,
    pub role: String,
}