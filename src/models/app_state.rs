use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use redis_derive::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use crate::services::auction::DatabaseAccess;
#[derive(Debug,Clone)]
pub struct AppState {
    pub rooms: Arc<RwLock<HashMap<String, Vec<(i32, broadcast::Sender<String>)>>>>, // i32 is participant id
    pub database_connection: Arc<DatabaseAccess>,
}

#[derive(Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize)]
pub struct Player {
    pub id: i32,
    pub name: String,
    pub base_price: i32,
    pub country: String,
    pub role: String,
}