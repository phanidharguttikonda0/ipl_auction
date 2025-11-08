use redis::{AsyncCommands, Commands, RedisResult};
use crate::auction::bid_allowance_handler;
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

    pub async fn get_player(&mut self, player_id: i32) -> Result<Player, redis::RedisError> {
        let value: RedisResult<Vec<Player>> = self.connection.get(player_id.clone()).await ;
        match value {
            Ok(value) => {
                let player = value[player_id] ;
                Ok(player)
            },
            Err(e) => {
                tracing::warn!("error occured while getting player") ;
                Err(e)
            }
        }
    }
    
    pub async fn new_bid(&mut self, participant_id: i32, room_id: String) -> Result<f32, String> {
       let mut room: RedisResult<AuctionRoom>= self.connection.get(room_id.clone()).await ;
        match room { 
            Ok(mut room) => {
                let current_bid = room.current_bid.unwrap();
                let previous_bid = current_bid.bid_amount ;
                let next_bid_increment ;
                if previous_bid < 1.0 {
                    // we are going to increment by 0.05
                    next_bid_increment = 0.05 ;
                }else if previous_bid < 10.0 {
                    next_bid_increment = 0.10 ;
                }else {
                    next_bid_increment = 0.25 ;
                }
                let balance ;
                let players_brought ;
                for participant in room.participants.iter() {
                    if participant.id == participant_id {
                        balance = participant.balance;
                        players_brought = participant.total_players_brought ;
                    }
                }
                // calculating bid allowance for the participant
                let is_allowed = bid_allowance_handler(room_id.clone(), participant_id, previous_bid + next_bid_increment,
                balance, players_brought).await;
                
                if is_allowed {
                    // we are going to update the
                    room.current_bid = Some(Bid::new(participant_id, current_bid.player_id, previous_bid + next_bid_increment, current_bid.base_price)) ;
                    let res = self.connection.set(room_id.clone(), room).await ;
                    match res {
                        Ok(_) => {
                            Ok(previous_bid + next_bid_increment)
                        }, 
                        Err(err) => {
                            Err("unable to update the bid".to_string())
                        }
                    }
                }else { 
                    Err("Amount is InSufficient to buy the player".to_string()) 
                }
            },
            Err(e) => {
                tracing::warn!("error occured while getting room details") ;
                Err("error occurred while getting room details".to_string())
            }
        }
    }
}