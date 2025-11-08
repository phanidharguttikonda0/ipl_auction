use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use crate::services::auction::DatabaseAccess;
#[derive(Debug,Clone)]
pub struct AppState {
    pub rooms: Arc<RwLock<HashMap<String, Vec<broadcast::Sender<String>>>>>,
    pub database_connection: Arc<DatabaseAccess>,
}