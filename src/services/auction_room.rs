use redis::{AsyncCommands, Commands, RedisResult};
use crate::models::app_state::Player;
use crate::models::auction_models::{AuctionParticipant, AuctionRoom, Bid};

pub struct RedisConnection {
    connection: redis::aio::MultiplexedConnection,
}

impl RedisConnection {
    pub fn new() -> Self {
        let connection_url = std::env::var("REDIS_URL").unwrap();
        Self {
            connection: redis::Client::open(format!("redis://{}:6379/",connection_url)).unwrap().get_multiplexed_async_connection().unwrap(),
        }
    }

    pub async fn set_room(&mut self,room_id: String, room: AuctionRoom) -> Result<String, redis::RedisError> {
        self.connection.set(room_id, room).await
    }

    pub async fn update_current_bid(&mut self, room_id: String, bid: Bid) -> Result<String, redis::RedisError> {
        let value: RedisResult<AuctionRoom> = self.connection.get(room_id.clone()).await ;
        match value {
            Ok(mut value) => {
                value.current_bid  = Some(bid) ;
                self.connection.set(room_id, value).await
            },
            Err(e) => {
                Err(e)
            }
        }
    }

    pub async fn add_participant(&mut self, participant_id: i32, room_id: String, participant: AuctionParticipant) -> Result<String, redis::RedisError> {
        let mut value: RedisResult<AuctionRoom> = self.connection.get(room_id.clone()).await ;
        match value {
            Ok(mut value) => {
                value.add_participant(participant) ;
                self.connection.set(room_id, value).await
            },
            Err(e) => {
                Err(e)
            }
        }
    }

    pub fn check_participant(&mut self, participant_id: i32, room_id: String) -> Result<bool, redis::RedisError> {
        let value = self.connection.get(room_id.clone()) ;
        match value {
            Ok(value) => {
                for participant in value.participants {
                    if participant.id == participant_id {
                        return Ok(true) ;
                    }
                }
                Ok(false)
            },
            Err(e) => {

                tracing::warn!("room itself doesn't exists") ;
                Err(e)
            }
        }
    }

    pub async fn load_players_to_redis(&mut self, players: Vec<Player>) -> Result<(), redis::RedisError> {
        let value = self.connection.get("players").await ;
        match value {
            Ok(value) => {
                tracing::info!("players already exists in redis") ;
                Ok(())
            },
            Err(e) => {
                tracing::info!("players doesn't exists in redis") ;
                tracing::info!("adding players to redis") ;
                self.connection.set("players", players).await
            }
        }
    }
}