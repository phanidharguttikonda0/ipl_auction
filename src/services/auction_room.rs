use std::sync::Arc;
use redis::{AsyncCommands, Commands, RedisResult};
use crate::auction::{bid_allowance_handler, broadcast_handler};
use crate::models::app_state::{AppState, Player};
use crate::models::auction_models::{AuctionParticipant, AuctionRoom, Bid, SoldPlayer};

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

    pub async fn update_current_bid(&mut self, room_id: String, bid: Bid, expiry: u8) -> Result<String, redis::RedisError> {
        let value: RedisResult<AuctionRoom> = self.connection.get(room_id.clone()).await ;
        let timer_key = format!("auction:timer:{}", room_id);
        match value {
            Ok(mut value) => {
                value.current_bid  = Some(bid) ;
                self.connection.set(room_id.clone(), value).await.expect("unable to set the updated value in new_bid");
                let res = self.connection.set_ex(&timer_key, "active", expiry as u64).await;
                Ok("Bid updated and TTL reset".to_string())
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
    
    pub async fn new_bid(&mut self, participant_id: i32, room_id: String, expiry_time: u8) -> Result<f32, String> {
       let mut room: RedisResult<AuctionRoom>= self.connection.get(room_id.clone()).await ;
        let timer_key = format!("auction:timer:{}", room_id);
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
                let participant = get_participant_details(participant_id, &room.participants).unwrap().0;
                let balance = participant.balance;
                let players_brought = participant.total_players_brought;
                // calculating bid allowance for the participant
                let is_allowed = bid_allowance_handler(room_id.clone(), participant_id, previous_bid + next_bid_increment,
                balance, players_brought).await;
                
                if is_allowed {
                    room.current_bid = Some(Bid::new(participant_id, current_bid.player_id, previous_bid + next_bid_increment, current_bid.base_price)) ;

                    self.connection.set(room_id.clone(), room).await.expect("unable to set the updated value in new_bid");
                    // it gets restarted if any bid comes before 20 seconds
                    let res = self.connection.set_ex(&timer_key, "active", expiry_time as u64).await;

                    Ok(previous_bid + next_bid_increment)
                }else { 
                    Err("Amount is InSufficient to buy the player".to_string()) 
                }
            },
            Err(e) => {
                tracing::warn!("error occurred while getting room details") ;
                Err("error occurred while getting room details".to_string())
            }
        }
    }
}



// -------------------------- Spawning the task for expiry bid logic ------------------------------------
use tokio_stream::StreamExt;
use redis::{Client, aio::PubSub};
use axum::extract::ws::{Message};

pub async fn listen_for_expiry_events(redis_url: &str, mut app_state: Arc<AppState>) -> redis::RedisResult<()> {
    let client = Client::open(redis_url)?;
    let mut pubsub = client.get_async_pubsub().await?;
    let mut conn = client.get_multiplexed_async_connection().await?;
    // Subscribe to key event notifications for expired keys
    pubsub.subscribe("__keyevent@0__:expired").await?;

    let mut stream = pubsub.on_message();
    while let Some(msg) = stream.next().await {
        let expired_key: String = msg.get_payload()?;
        tracing::info!("Key expired: {}", expired_key);
        let room_id = expired_key
            .split(':')
            .last()
            .unwrap_or_default()
            .to_string();
        tracing::info!("room_id from that expiry key was {}", room_id);
        tracing::info!("we are going to use the key to broadcast") ;
        let mut res: AuctionRoom = conn.get(&room_id).await? ;
        let message;
        let participant_id = res.current_bid.clone().unwrap().participant_id ;
        let player_id = res.current_bid.clone().unwrap().player_id ;
        if res.current_bid.clone().unwrap().bid_amount != 0.0 {
            message= Message::from(
                serde_json::to_string(&SoldPlayer {
                    team_name: app_state.database_connection.get_team_name(participant_id).await.unwrap(),
                    sold_price: res.current_bid.clone().unwrap().bid_amount
                }).unwrap()
            );
        }else{
            message = Message::text("UnSold") ;
        }
        tracing::info!("we are going to update the balance of the participant") ;
        let details = get_participant_details(participant_id, &res.participants).unwrap() ;
        res.participants[details.1].balance = res.participants[details.1].balance -  res.current_bid.clone().unwrap().bid_amount;
        // need to update that particular participant purse or balance before storing the updated value in redis
        res.current_bid = Some(Bid::new(0, 0,0.0,0.0)) ;
        conn.set(&room_id, res).await?;
        tracing::info!("we are going to broadcast the message to the room participant") ;
        broadcast_handler(message,room_id.clone(), &mut app_state ).await ;
        // we are going to get the next player and broadcasting the next player
        let next_player = player_id + 1 ;
        let players = conn.get("players").await;
        let message;
        match players {
            Ok(players) => {
                if ((next_player) as usize ) < players.len(){
                    message = Message::from(serde_json::to_string(&players[next_player]).unwrap()) ;
                }else{
                    message = Message::text("Auction Completed")
                }
                broadcast_handler(message,room_id, &mut app_state ).await ;
            },
            Err(err) => {
                tracing::warn!("error occurred while getting players") ;
                message = Message::text("retry later")
            }
        };
    }

    Ok(())
}


pub fn get_participant_details(participant_id: i32, participants: &Vec<AuctionParticipant>) -> Option<(AuctionParticipant, u8)> {
    let mut index: u8 = 0 ;
    for participant in participants.iter() {
        if participant.id == participant_id {
            return Some((participant.clone(), index))
        }
        index += 1 ;
    }
    None
}