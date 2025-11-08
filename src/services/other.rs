use sqlx::{Pool, Postgres};
use crate::services::auction::DatabaseAccess;
use crate::services::auction_room::RedisConnection;

pub async fn load_players_to_redis(conn: &DatabaseAccess) {
    let mut redis_connection = RedisConnection::new();
    // need to load players from the postgres database
    let players = conn.get_players().await.unwrap();
    redis_connection.load_players_to_redis(vec![]).await.unwrap();
    tracing::info!("loading players to redis successful") ;
}