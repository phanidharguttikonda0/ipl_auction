use std::sync::Arc;
use redis::{AsyncCommands, Commands, RedisResult};
use crate::auction::{bid_allowance_handler, broadcast_handler, send_message_to_participant};
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

    pub async fn check_room_existence(
        &mut self,
        room_id: String,
    ) -> Result<bool, redis::RedisError> {
        // Try to get the value for the given key
        let value: Option<String> = self.connection.get(room_id).await?;

        // If Redis returns `None`, it means the key doesn't exist
        Ok(value.is_some())
    }

    pub async fn update_current_bid(&mut self, room_id: String, bid: Bid, expiry: u8) -> Result<String, redis::RedisError> {
        let value: RedisResult<String> = self.connection.get(room_id.clone()).await ;
        let timer_key;
        if bid.is_rtm {
            timer_key = format!("auction:timer:rtms{}", room_id)
        }else {
            timer_key = format!("auction:timer:{}", room_id) ;
        }
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
                    room.current_bid = Some(Bid::new(participant_id, current_bid.player_id, next_bid, current_bid.base_price, false)) ;
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

    pub async fn last_player_id(&mut self, room_id: String) -> Result<i32, redis::RedisError> {
        tracing::info!("getting last player id redis function was called") ;
        let value: RedisResult<String> = self.connection.get(room_id.clone()).await ;
        match value {
            Ok(value) => {
                tracing::info!("let's get the value , whether it was null or it contains room") ;
                match serde_json::from_str::<AuctionRoom>(&value) {
                    Ok(val) => Ok(val.last_player_id),
                    Err(err) => {
                        tracing::warn!("no last value") ;
                        Ok(1)
                    }
                }
            },
            Err(e) => {
                tracing::warn!("error occurred while getting room details from last_player_id") ;
                Err(e)
            }
        }
    }
    
    pub async fn get_participants(&mut self, room_id: String) -> Result<Vec<AuctionParticipant>, redis::RedisError> {
        let mut value: RedisResult<String> = self.connection.get(room_id.clone()).await ;
        match value {
            Ok(mut value) => {
                let mut value: AuctionRoom = serde_json::from_str(&value).unwrap();
                Ok(value.participants)
            },
            Err(e) => {
                Err(e)
            }
        }
    }

    pub async fn update_last_player_id(&mut self, room_id: String, player_id: i32) -> Result<(), redis::RedisError> {
        tracing::info!("updating last player id redis function was called") ;
        let value: RedisResult<String> = self.connection.get(room_id.clone()).await ;
        match value {
            Ok(mut value) => {
                let mut room: AuctionRoom = serde_json::from_str(&value).unwrap();
                room.last_player_id = player_id ;
                let value = serde_json::to_string(&room).unwrap();
                self.connection.set(room_id, value).await
            },
            Err(e) => {
                tracing::error!("got error while updating last player id redis function was called") ;
                Err(e)
            }
        }
    }

    pub async fn remove_room(&mut self, room_id: String) -> Result<(), redis::RedisError> {
        let result:RedisResult<i32> = self.connection.del(room_id).await ;
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e)
        }
    }

    pub async fn set_state_to_pause(&mut self, room_id: String, value: bool) -> Result<(), redis::RedisError> {
        let result: RedisResult<String> = self.connection.get(&room_id).await ;

        match result {
            Ok(room) => {
                let mut room: AuctionRoom = serde_json::from_str(&room).unwrap();
                if room.paused == value {
                    return Ok(())
                }
                room.paused = value ;
                self.connection.set::<_, _, ()>(room_id, serde_json::to_string(&room).unwrap()).await.expect("unable to set the updated value in set_state_to_pause") ;
                Ok(())
            },
            Err(e) => {
                tracing::error!("got error while updating last player id redis function was called") ;
                tracing::error!("{}",e) ;
                Err(e)
            }
        }
    }

    pub async fn get_remaining_rtms(&mut self, room_id: String, participant_id: i32) -> Result<i16, redis::RedisError> {
        tracing::info!("getting remaining rtms redis function was called") ;
        let result: RedisResult<String> = self.connection.get(&room_id).await ;
        match result {
            Ok(value) => {
                let room: AuctionRoom = serde_json::from_str(&value).unwrap();
                let participant = get_participant_details(participant_id, &room.participants).unwrap().0;
                Ok(participant.remaining_rtms)
            },
            Err(e) => {
                tracing::error!("got error while get remaining_rtms redis function was called") ;
                tracing::error!("{}",e) ;
                Err(e)
            }
        }
    }

    pub async fn update_remaining_rtms(&mut self, room_id: String, participant_id: i32) -> Result<(), redis::RedisError> {
        tracing::info!("updating remaining rtms redis function was called") ;
        let result: RedisResult<String> = self.connection.get(&room_id).await ;
        match result {
            Ok(value) => {
                let mut room: AuctionRoom = serde_json::from_str(&value).unwrap();
                let participant = get_participant_details(participant_id, &room.participants).unwrap();
                room.participants[participant.1 as usize].remaining_rtms -= 1 ;
                // we are going to set the updated value in the redis
                self.connection.set::<_, _, ()>(room_id, serde_json::to_string(&room).unwrap()).await.expect("unable to set the updated value in update_remaining_rtms") ;
                Ok(())
            },
            Err(e) => {
                tracing::error!("got error while updating remaining rtms redis function was called") ;
                tracing::error!("{}",e) ;
                Err(e)
            }
        }
    }


    pub async fn check_key_exists(&mut self, key: &str) -> Result<bool, redis::RedisError> {
        tracing::info!("checking key exists redis function was called") ;
        let result: RedisResult<i32> = self.connection.exists(key).await ;
        match result {
            Ok(value) => Ok(value != 0),
            Err(e) => Err(e)
        }
    }

    pub async fn get_room_details(&mut self, room_id: String) -> Result<AuctionRoom, redis::RedisError> {
        let room: RedisResult<String> = self.connection.get(&room_id).await ;
        match room {
            Ok(room) => Ok(serde_json::from_str(&room).unwrap()),
            Err(e) => Err(e)
        }
    }

}



// -------------------------- Spawning the task for expiry bid logic ------------------------------------
use tokio_stream::StreamExt;
use redis::{Client, aio::PubSub};
use axum::extract::ws::{Message};
use serde_json::Error;
use crate::models::room_models::Participant;
use crate::services::other::get_previous_team_full_name;

pub async fn listen_for_expiry_events(redis_url: &str, app_state: Arc<AppState>) -> redis::RedisResult<()> {
    tracing::info!("🔔 Redis expiry listener started");
    let client = Client::open(redis_url)?;
    let mut pubsub = client.get_async_pubsub().await?;
    let mut conn = client.get_multiplexed_async_connection().await?;
    // Subscribe to key event notifications for expired keys
    pubsub.subscribe("__keyevent@0__:expired").await?;

    let mut redis_connection = RedisConnection::new().await;

    let bid_expiry = std::env::var("BID_EXPIRY").unwrap().parse::<u8>().unwrap();

    let mut stream = pubsub.on_message();
    while let Some(msg) = stream.next().await {
        let expired_key: String = msg.get_payload()?;
        tracing::info!("Key expired: {}", expired_key);
        let parts: Vec<&str> = expired_key.split(':').collect();

        let room_id = parts.get(parts.len() - 1).unwrap_or(&"").to_string();
        let is_rtm = parts.get(parts.len() - 2).unwrap_or(&"").to_string();

        tracing::info!("room_id from that expiry key was {}", room_id);
        tracing::info!("we are going to use the key to broadcast") ;
        match conn.get::<_, Option<String>>(&room_id).await {
            Ok(Some(room)) => {
                let mut res: AuctionRoom = serde_json::from_str(&room).unwrap();
                let message;
                let participant_id = res.current_bid.clone().unwrap().participant_id ;
                let player_id = res.current_bid.clone().unwrap().player_id ;

                // here we are going to check whether the specific player having the previous team or not
                let previous_player = redis_connection.get_player(player_id).await? ;
                let remaining_rtms = redis_connection.get_remaining_rtms(room_id.clone(), participant_id).await?;

                if is_rtm == "rtms" {
                    tracing::info!("the expired key was the RTM one") ;
                    // as it is expired, we are going to sell the player to the last bidded person
                    let bid = res.current_bid.clone().unwrap() ;
                    if bid.is_rtm {
                        // we are going to update the rtms of the user
                        redis_connection.update_remaining_rtms(room_id.clone(), participant_id).await?;
                        // we are going to update in the sql as well.
                        app_state.database_connection.update_remaining_rtms(participant_id).await.unwrap();
                    }
                    // we are going to sell the player to the person,
                    tracing::info!("player was a sold player") ;
                    app_state.database_connection.add_sold_player(room_id.clone(), bid.player_id, bid.participant_id, bid.bid_amount).await.unwrap();
                    let details = get_participant_details(participant_id, &res.participants).unwrap() ;
                    res.participants[details.1 as usize].balance = res.participants[details.1 as usize].balance -  res.current_bid.clone().unwrap().bid_amount;
                    res.participants[details.1 as usize].total_players_brought += 1 ;
                    let remaining_balance = res.participants[details.1 as usize].balance ;
                    // updating the participant balance in the participant table
                    app_state.database_connection.update_balance(room_id.clone(), bid.participant_id, remaining_balance).await.unwrap() ;
                    tracing::info!("successfully updated the balance in the psql") ;
                    tracing::info!("updating in the redis along with the balance and bid") ;
                    res.current_bid = Some(Bid::new(0, 0,0.0,0.0, false)) ;
                    let res = serde_json::to_string(&res).unwrap();
                    conn.set::<_,_,()>(&room_id, res).await?;
                    broadcast_handler(Message::from(
                        serde_json::to_string(&SoldPlayer {
                            team_name: app_state.database_connection.get_team_name(participant_id).await.unwrap(),
                            sold_price: bid.bid_amount,
                            remaining_balance
                        }).unwrap()
                    ), room_id.clone(), &app_state).await ;

                }else{
                    tracing::info!("****************************") ;
                    let full_team_name = get_previous_team_full_name(&previous_player.previous_team) ;
                    let mut previous_team_participant_id= 0 ;
                    for participant in res.participants.iter() {
                        tracing::info!("participant.teamname {} and previous_player.previous_team {}", participant.team_name, full_team_name) ;
                        if participant.team_name == full_team_name {
                            tracing::info!("previous team participant {}", participant.id) ;
                            previous_team_participant_id = participant.id ;
                            break
                        }
                    }
                    if (!previous_player.previous_team.contains("-"))  && remaining_rtms > 0 {
                        let previous_team = get_previous_team_full_name(&previous_player.previous_team) ;

                        // so we are going to create a new expiry key, and for that key there will be another subscriber
                        // now we are going to send the notification to the previous team to use the RTM, if he not uses it
                        // then this will expiry in 20 seconds.

                        send_message_to_participant(previous_team_participant_id, String::from("Use RTM"), room_id.clone(), &app_state).await ;

                        // setting the new timer
                        let timer_key = format!("auction:timer:rtms:{}", room_id); // if this key exists in the redis then no bids takes place
                        redis_connection.connection.set_ex::<_, _, ()>(&timer_key, "rtm", bid_expiry as u64).await.expect("unable to set the updated value in new_bid");
                        tracing::info!("we have successfully sent the message to the previous team, regarding RTM") ;

                    }else {
                        tracing::info!("we are going to update the balance of the participant") ;
                        let current_bid= res.current_bid.clone().unwrap() ;
                        let length ;
                        {
                            let rooms = &app_state.rooms;
                            let mut rooms_lock = rooms.read().await; // acquire mutable write lock
                            length = rooms_lock.get(&room_id).unwrap().len() ;
                        }
                        tracing::info!("length of room {}", length) ;
                        if res.paused {
                            tracing::info!("Auction was Stopped no more bids, takes place") ;
                            // we are going to make the last bid invalid, and last player_id will be same, and bid will be all zeros

                            res.current_bid = Some(Bid::new(0, 0,0.0,0.0, false)) ;
                            // now when people joined the room creator can click on start and from the last player it will continue
                            let _: () =redis_connection.connection.set(&room_id, serde_json::to_string(&res).unwrap()).await?;
                            let message = Message::text("Auction was Paused Temporarily") ;
                            broadcast_handler(message,room_id.clone(), &app_state).await ;
                        }else{
                            let mut remaining_balance: f32 = 0.0 ;
                            if participant_id != 0 {
                                let details = get_participant_details(participant_id, &res.participants).unwrap() ;
                                res.participants[details.1 as usize].balance = res.participants[details.1 as usize].balance -  res.current_bid.clone().unwrap().bid_amount;
                                res.participants[details.1 as usize].total_players_brought += 1 ;
                                remaining_balance = res.participants[details.1 as usize].balance ;
                                res.current_bid = Some(Bid::new(0, 0,0.0,0.0, false)) ;
                            }

                            if current_bid.clone().bid_amount != 0.0 {
                                message= Message::from(
                                    serde_json::to_string(&SoldPlayer {
                                        team_name: app_state.database_connection.get_team_name(participant_id).await.unwrap(),
                                        sold_price: current_bid.clone().bid_amount,
                                        remaining_balance
                                    }).unwrap()
                                );
                            }else{
                                message = Message::text("UnSold") ;
                            }
                            let res = serde_json::to_string(&res).unwrap();
                            conn.set::<_,_,()>(&room_id, res).await?;
                            tracing::info!("we are going to broadcast the message to the room participant") ;
                            broadcast_handler(message,room_id.clone(), &app_state ).await ;

                            // -------------------- over here we need to add the player to the sold player list with room-id and player-id
                            if current_bid.bid_amount != 0.0 {
                                tracing::info!("player was a sold player") ;
                                app_state.database_connection.add_sold_player(room_id.clone(), current_bid.player_id, current_bid.participant_id, current_bid.bid_amount).await.unwrap();
                                // updating the participant balance in the participant table
                                app_state.database_connection.update_balance(room_id.clone(), current_bid.participant_id, remaining_balance).await.unwrap() ;
                                tracing::info!("successfully updated the balance in the psql") ;
                            }else {
                                tracing::info!("player was an unsold player") ;
                                app_state.database_connection.add_unsold_player(room_id.clone(), current_bid.player_id).await.unwrap();
                            }
                        }
                    }
                }

                if !(!previous_player.previous_team.contains("-")  && remaining_rtms > 0) {
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
                                redis_connection.update_current_bid(room_id.clone(), Bid::new(0, next_player, 0.0, players[next_player as usize].base_price, false), bid_expiry).await?;
                                tracing::info!("now updating last player id") ;
                                redis_connection.update_last_player_id(room_id.clone(), next_player).await?;
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
                    broadcast_handler(message,room_id.clone(), &app_state ).await ;
                }

            },
            Ok(None) => {
                tracing::warn!("room does not exist in redis") ;
                tracing::error!("As some one was already clicked the end button") ;
            } ,
            Err(err) => {
                tracing::error!("error occurred while getting room details from expiry listener") ;
                tracing::error!("{}",err) ;
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