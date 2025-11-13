use std::sync::Arc;
use redis::{AsyncCommands, Commands, RedisResult};
use crate::auction::{bid_allowance_handler, broadcast_handler};
use crate::models::app_state::{AppState, Player};
use crate::models::auction_models::{AuctionParticipant, AuctionRoom, Bid, SoldPlayer};

pub struct RedisConnection {
    connection: redis::aio::MultiplexedConnection,
}

impl RedisConnection {
    pub async fn new() -> Self {
        let connection_url = std::env::var("REDIS_URL").unwrap();
        Self {
            connection: redis::Client::open(format!("redis://{}:6379/",connection_url)).unwrap().get_multiplexed_async_connection().await.unwrap(),
        }
    }

    pub async fn set_room(&mut self,room_id: String, room: AuctionRoom) -> Result<String, redis::RedisError> {
        let serialized_room = serde_json::to_string(&room).unwrap();
        /*
            as redis doesn't able to understand rust structs , even though the struct implements traits like
            FromRedisArgs , ToRedisArgs , it cannot able to serialize or deserialize the values of vec or Options etc.
        */
        self.connection.set(room_id, serialized_room).await
    }

    pub async fn update_current_bid(&mut self, room_id: String, bid: Bid, expiry: u8) -> Result<String, redis::RedisError> {
        let value: RedisResult<String> = self.connection.get(room_id.clone()).await ;
        let timer_key = format!("auction:timer:{}", room_id);
        match value {
            Ok(mut value) => {
                let mut value: AuctionRoom = serde_json::from_str(&value).unwrap();
                value.current_bid  = Some(bid) ;
                let value = serde_json::to_string(&value).unwrap();
                self.connection.set::<_, _, ()>(room_id.clone(), value).await.expect("unable to set the updated value in new_bid");
                 self.connection.set_ex::<_,_,()>(&timer_key, "active", expiry as u64).await.expect("unable to set the expiry");
                Ok("Bid updated and TTL reset".to_string())
            },
            Err(e) => {
                Err(e)
            }
        }
    }

    pub async fn add_participant(&mut self, room_id: String, participant: AuctionParticipant) -> Result<String, redis::RedisError> {
        let mut value: RedisResult<String> = self.connection.get(room_id.clone()).await ;
        match value {
            Ok(mut value) => {
                let mut value: AuctionRoom = serde_json::from_str(&value).unwrap();
                value.add_participant(participant) ;
                let value = serde_json::to_string(&value).unwrap();
                self.connection.set(room_id, value).await
            },
            Err(e) => {
                Err(e)
            }
        }
    }

    pub async fn get_participant(&mut self, room_id: String, participant_id: i32) -> Result<Option<AuctionParticipant>, redis::RedisError> {
        let value: RedisResult<String> = self.connection.get(room_id.clone()).await ;
        match value {
            Ok(value) => {
                let mut value: AuctionRoom = serde_json::from_str(&value).unwrap();
                for participant in value.participants.iter() {
                    if participant.id == participant_id {
                        return Ok(Some(participant.clone()));
                    }
                }
                Ok(None)
            },
            Err(err) => {
                return Err(err);
            }
        }
    }

    pub async fn check_participant(&mut self, participant_id: i32, room_id: String) -> Result<bool, redis::RedisError> {
        let value: RedisResult<String> = self.connection.get(room_id.clone()).await ;
        match value {
            Ok(value) => {
                let mut value: AuctionRoom = serde_json::from_str(&value).unwrap();
                for participant in value.participants {
                    if participant.id == participant_id {
                        return Ok(true) ;
                    }
                }
                Ok(false)
            },
            Err(err) => {

                // Handling "key not found" or empty data gracefully
                if err.kind() == redis::ErrorKind::TypeError || err.to_string().contains("Response was nil") {
                    tracing::info!("Room '{}' does not exist in Redis yet", room_id);
                    return Ok(false);
                }

                tracing::error!("Redis error in check_participant for room {}: {}", room_id, err);
                Err(err)
            }
        }
    }

    pub async fn load_players_to_redis(&mut self, players: Vec<Player>) -> Result<(), String> {
        let value: RedisResult<String> = self.connection.get("players").await ;
        match value {
            Ok(value) => {
                tracing::info!("players already exists in redis") ;
                let players: Result<Vec<Player>, Error> = serde_json::from_str(&value) ;
                match players {
                    Ok(players) =>{
                        tracing::info!("we players from redis , let's check first player {:?}", players[0]) ;
                        Ok(())
                    },
                    Err(err) => {
                        tracing::info!("no players found in redis") ;
                        Err("players not exists in redis".to_string())
                    }
                }
            },
            Err(e) => {
                tracing::info!("players doesn't exists in redis") ;
                tracing::info!("adding players to redis") ;
                let _:() = self.connection.set("players", serde_json::to_string(&players).unwrap()).await.expect("unable to add players to redis");
                Err("getting error while getting players key from redis".to_string())
            }
        }
    }


    pub async fn get_player(&mut self, player_id: i32) -> Result<Player, redis::RedisError> {
        // Get raw JSON string from Redis
        let data: RedisResult<String> = self.connection.get("players").await;

        match data {
            Ok(json) => {
                // Deserializing JSON → Vec<Player>
                let players: Vec<Player> = serde_json::from_str(&json)
                    .map_err(|_| redis::RedisError::from((redis::ErrorKind::TypeError, "Invalid JSON")))?;

                // Find the player by ID
                if let Some(player) = players.into_iter().find(|p| p.id == player_id) {
                    Ok(player)
                } else {
                    Err(redis::RedisError::from((redis::ErrorKind::TypeError, "Player not found")))
                }
            }
            Err(e) => {
                tracing::warn!("Error while fetching players from Redis: {:?}", e);
                Err(e)
            }
        }
    }


    pub async fn new_bid(&mut self, participant_id: i32, room_id: String, expiry_time: u8) -> Result<f32, String> {
       let mut room: RedisResult<String>= self.connection.get(room_id.clone()).await ;
        let timer_key = format!("auction:timer:{}", room_id);
        tracing::info!("timer key was {}", timer_key) ;
        match room {
            Ok(mut room) => {
                let mut room: AuctionRoom = serde_json::from_str(&room).unwrap();
                let current_bid = room.current_bid.unwrap();
                let mut previous_bid = current_bid.bid_amount ;
                if current_bid.participant_id == participant_id {
                    return Err("highest".to_string())
                }
                let next_bid_increment ;
                if previous_bid == 0.0 {
                    next_bid_increment = current_bid.base_price ;
                } else if previous_bid < 1.0 {
                    // we are going to increment by 0.05
                    next_bid_increment = 0.05 ;
                }else if previous_bid < 10.0 {
                    next_bid_increment = 0.10 ;
                }else {
                    next_bid_increment = 0.25 ;
                }
                tracing::info!("next bid increment was {}", next_bid_increment) ;
                let participant = get_participant_details(participant_id, &room.participants).unwrap().0;
                let balance = participant.balance;
                let players_brought = participant.total_players_brought;
                let next_bid = ((previous_bid + next_bid_increment) * 100.0).round() / 100.0;

                // calculating bid allowance for the participant
                let is_allowed = bid_allowance_handler(room_id.clone(),  next_bid,
                balance, players_brought).await;
                tracing::info!("is bid allowed {}", is_allowed) ;
                if is_allowed {
                    room.current_bid = Some(Bid::new(participant_id, current_bid.player_id, next_bid, current_bid.base_price)) ;
                    let room = serde_json::to_string(&room).unwrap();
                    tracing::info!("room was serialized ") ;
                    self.connection.set::<_, _, ()>(room_id.clone(), room).await.expect("unable to set the updated value in new_bid");
                    // it gets restarted if any bid comes before 20 seconds
                    let res = self.connection.set_ex::<_,_, ()>(&timer_key, "active", expiry_time as u64).await;

                    Ok(next_bid)
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

    pub async fn check_end_auction(&mut self, room_id: String) -> Result<bool, redis::RedisError> {
        let room: RedisResult<String> = self.connection.get(&room_id).await ;
        match room {
            Ok(room) => {
                tracing::info!("checking all the participants no of player brought") ;
                let room: AuctionRoom = serde_json::from_str(&room).unwrap();
                let mut fail = true ;
                for participant in room.participants.iter() {
                    if participant.total_players_brought < 15 {
                        fail = false ;
                    }
                }
                if fail {
                    Ok(false)
                }else {
                    Ok(true)
                }
            },
            Err(e) => {
                tracing::warn!("error occurred while getting room details from check_end_auction") ;
                Err(e)
            }
        }
    }
}



// -------------------------- Spawning the task for expiry bid logic ------------------------------------
use tokio_stream::StreamExt;
use redis::{Client, aio::PubSub};
use axum::extract::ws::{Message};
use serde_json::Error;

pub async fn listen_for_expiry_events(redis_url: &str, app_state: Arc<AppState>) -> redis::RedisResult<()> {
    tracing::info!("🔔 Redis expiry listener started");
    let client = Client::open(redis_url)?;
    let mut pubsub = client.get_async_pubsub().await?;
    let mut conn = client.get_multiplexed_async_connection().await?;
    // Subscribe to key event notifications for expired keys
    pubsub.subscribe("__keyevent@0__:expired").await?;
    let mut redis_connection = RedisConnection::new().await;
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
        let mut res: String = conn.get(&room_id).await? ;
        let mut res: AuctionRoom = serde_json::from_str(&res).unwrap();
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
        res.participants[details.1 as usize].balance = res.participants[details.1 as usize].balance -  res.current_bid.clone().unwrap().bid_amount;
        // need to update that particular participant purse or balance before storing the updated value in redis
        let current_bid = res.current_bid.clone().unwrap() ;
        res.current_bid = Some(Bid::new(0, 0,0.0,0.0)) ;
        let res = serde_json::to_string(&res).unwrap();
        conn.set::<_,_,()>(&room_id, res).await?;
        tracing::info!("we are going to broadcast the message to the room participant") ;
        broadcast_handler(message,room_id.clone(), &app_state ).await ;

        // -------------------- over here we need to add the player to the sold player list with room-id and player-id
        if current_bid.bid_amount != 0.0 {
            tracing::info!("player was a sold player") ;
            app_state.database_connection.add_sold_player(room_id.clone(), current_bid.player_id, current_bid.participant_id, current_bid.bid_amount).await.unwrap();
        }else {
            tracing::info!("player was an unsold player") ;
            app_state.database_connection.add_unsold_player(room_id.clone(), current_bid.player_id).await.unwrap();
        }
        // we are going to get the next player and broadcasting the next player
        let next_player = player_id + 1 ;
        let players: RedisResult<String> = conn.get("players").await;
        let message;
        match players {
            Ok(players) => {
                let players: Vec<Player> = serde_json::from_str(&players).unwrap() ;
                if ((next_player) as usize ) < players.len(){
                    message = Message::from(serde_json::to_string(&players[next_player as usize]).unwrap()) ;
                    // we are going to update the current bid
                    redis_connection.update_current_bid(room_id.clone(), Bid::new(0, next_player, 0.0, players[next_player as usize].base_price), 20).await?;
                    tracing::info!("we are going to broadcast the next player, completed with updating current bid with new player") ;
                }else{
                    message = Message::text("Auction Completed")
                }
            },
            Err(err) => {
                tracing::warn!("error occurred while getting players") ;
                tracing::error!("error was {}", err) ;
                message = Message::text("Error Occurred while getting players from redis") ;
            }
        };
        broadcast_handler(message,room_id, &app_state ).await ;
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