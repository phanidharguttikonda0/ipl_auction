use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use redis::{AsyncCommands, Commands, RedisResult};
use crate::auction::{bid_allowance_handler, broadcast_handler, is_foreigner_allowed, send_message_to_participant};
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

    pub async fn update_current_bid(&mut self, room_id: &str, bid: Bid, expiry: u8) -> Result<String, redis::RedisError> {
        let value: RedisResult<String> = self.connection.get(room_id.clone()).await ;
        let timer_key;
        if bid.is_rtm {
            timer_key = format!("auction:timer:rtms:{}", room_id)
        }else {
            timer_key = format!("auction:timer:{}", room_id) ;
        }
        match value {
            Ok(mut value) => {
                let mut value: AuctionRoom = serde_json::from_str(&value).unwrap();
                value.current_bid  = Some(bid) ;
                let value = serde_json::to_string(&value).unwrap();
                self.connection.set::<_, _, ()>(room_id.clone(), value).await.expect("unable to set the updated value in new_bid");
                 if expiry != 0 {
                     self.connection.set_ex::<_,_,()>(&timer_key, "active", expiry as u64).await.expect("unable to set the expiry");
                 }// we can pass expiry as 0  if we want to just update the bid with out any timer
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


    pub async fn get_player(&mut self, player_id: i32, room_id: &str) -> Result<Player, redis::RedisError> {
        if room_id.len() != 0 {
            let room = self.get_room_details(room_id).await?;

            match room.current_player {
                Some(current_player) => {
                    tracing::info!("current-player exists, so returning current-player, with out looping") ;
                    if player_id == current_player.id {
                        return Ok(current_player)
                    }
                },
                None => {
                    tracing::info!("no current player, so getting the player from redis players key")
                }
            } ;
        }
        // Get raw JSON string from Redis
        let data: RedisResult<String> = self.connection.get("players").await;

        match data {
            Ok(json) => {
                // Deserializing JSON → Vec<Player>
                let players: Vec<Player> = serde_json::from_str(&json)
                    .map_err(|_| redis::RedisError::from((redis::ErrorKind::TypeError, "Invalid JSON")))?;

                tracing::info!("***************************") ;
                tracing::info!("total players length in redis {}", players.len()) ;
                tracing::info!("*******************************") ;
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
                    room.current_bid = Some(Bid::new(participant_id, current_bid.player_id, next_bid, current_bid.base_price, false, false)) ;
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

    pub async fn current_player_id(&mut self, room_id: String) -> Result<i32, redis::RedisError> {
        tracing::info!("getting current player id redis function was called") ;
        let value: RedisResult<String> = self.connection.get(room_id.clone()).await ;
        match value {
            Ok(value) => {
                tracing::info!("let's get the value , whether it was null or it contains room") ;
                match serde_json::from_str::<AuctionRoom>(&value) {
                    Ok(val) => {
                        match val.current_player {
                            Some(val) => Ok(val.id),
                            None => {
                                tracing::warn!("no current player") ;
                                Ok(1)
                            }
                        }
                    },
                    Err(err) => {
                        tracing::warn!("no current player") ;
                        Ok(1)
                    }
                }
            },
            Err(e) => {
                tracing::warn!("error occurred while getting room details from current-player") ;
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

    pub async fn update_current_player(&mut self, room_id: String, player: Player) -> Result<(), redis::RedisError> {
        tracing::info!("updating last player id redis function was called") ;
        let value: RedisResult<String> = self.connection.get(room_id.clone()).await ;
        match value {
            Ok(mut value) => {
                let mut room: AuctionRoom = serde_json::from_str(&value).unwrap();
                room.current_player = Some(player) ;
                let value = serde_json::to_string(&room).unwrap();
                self.connection.set(room_id, value).await
            },
            Err(e) => {
                tracing::error!("got error while updating last player id redis function was called") ;
                Err(e)
            }
        }
    }

    pub async fn atomic_delete(&mut self, key: &str) -> redis::RedisResult<i32> {
        let script = r#"
        local existed = redis.call('DEL', KEYS[1])
        return existed
    "#;

        let result: i32 = redis::Script::new(script)
            .key(key)
            .invoke_async(&mut self.connection)
            .await?;

        Ok(result)
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

    pub async fn update_remaining_rtms(&mut self, room_id: &str, participant_id: i32) -> Result<i16, redis::RedisError> {
        tracing::info!("updating remaining rtms redis function was called") ;
        let result: RedisResult<String> = self.connection.get(room_id).await ;
        match result {
            Ok(value) => {
                let mut room: AuctionRoom = serde_json::from_str(&value).unwrap();
                let participant = get_participant_details(participant_id, &room.participants).unwrap();
                room.participants[participant.1 as usize].remaining_rtms -= 1 ;
                let remaining_rtms = room.participants[participant.1 as usize].remaining_rtms ;
                // we are going to set the updated value in the redis
                self.connection.set::<_, _, ()>(room_id, serde_json::to_string(&room).unwrap()).await.expect("unable to set the updated value in update_remaining_rtms") ;
                Ok(remaining_rtms)
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

    pub async fn get_room_details(&mut self, room_id: &str) -> Result<AuctionRoom, redis::RedisError> {
        let room: RedisResult<String> = self.connection.get(room_id).await ;
        match room {
            Ok(room) => Ok(serde_json::from_str(&room).unwrap()),
            Err(e) => Err(e)
        }
    }

    pub async fn set_pause_status(&mut self, room_id: &str, pause_status: bool) -> Result<(), redis::RedisError> {
        let room: RedisResult<String> = self.connection.get(room_id).await ;
        match room {
            Ok(room) => {
                tracing::info!("setting pause status redis function was called") ;
                let mut room: AuctionRoom = serde_json::from_str(&room).unwrap();
                if room.pause != pause_status {
                    room.pause = pause_status ;
                    self.connection.set::<_, _, ()>(room_id, serde_json::to_string(&room).unwrap()).await.expect("unable to set the updated value in set_pause_status") ;
                }
                Ok(())
            },
            Err(err) => {
                tracing::error!("error occurred while getting room details from set_pause_status") ;
                Err(err)
            }
        }
    }

    pub async fn update_mute_status(&mut self, room_id: &str, participant_id: i32, unmute: bool) -> Result<(), redis::RedisError> {
        tracing::info!("updating mute status redis function was called") ;
        let mut room = self.get_room_details(room_id).await?;
        let mut participant = get_participant_details(participant_id, &room.participants).unwrap() ;
        tracing::info!("got the participant details from the mute status") ;
        participant.0.is_unmuted = unmute ;
        room.participants[participant.1 as usize] = participant.0 ; // updated the participant mute or unmute status
        let room = serde_json::to_string(&room).unwrap();
        self.connection.set::<_, _, ()>(room_id.clone(), room).await.expect("unable to set the updated value in update_mute_status");
        Ok(())
    }


    pub async fn is_creator(&mut self, room_id: &str, participant_id: i32) -> bool {
        let room = self.get_room_details(room_id).await.expect("unable to get room details");
        if room.room_creator_id == participant_id {
            true
        }else {
            false
        }
    }

    pub async fn increment_foreign_player_count(&mut self, room_id: &str, participant_id: i32) -> Result<(), redis::RedisError> {
        let mut room = self.get_room_details(room_id).await.expect("error getting room details from increment_foreign_player_count") ;
        let mut participant = get_participant_details(participant_id, &room.participants).expect("error getting participant details from increment_foreign_player_count") ;
        participant.0.foreign_players_brought += 1 ;
        room.participants[participant.1 as usize] = participant.0 ;
        self.connection.set::<_, _, ()>(room_id.clone(), serde_json::to_string(&room).unwrap()).await.expect("unable to set the updated value in increment_foreign_player_count") ;
        Ok(())
    }

    pub async fn add_current_player(&mut self, room_id: &str, player: Player)  {
        tracing::info!("adding current player redis function was called") ;
        let mut room = self.get_room_details(room_id).await.expect("unable to get room details") ;
        room.current_player = Some(player) ;
        self.connection.set::<_, _, ()>(room_id.clone(), serde_json::to_string(&room).unwrap()).await.expect("unable to set the updated value in add_current_player") ;
    }

    pub async fn update_balance_total_players_brought(&mut self, room_id: &str, participant_id: i32, new_balance: f32) -> Result<u8, redis::RedisError> {
        let mut room = self.get_room_details(room_id).await.expect("error getting room details from update_balance_total_players_brought") ;
        let mut participant = get_participant_details(participant_id, &room.participants).expect("error getting participant details from update_balance_total_players_brought") ;
        room.participants[participant.1 as usize].balance = new_balance ;
        room.participants[participant.1 as usize].total_players_brought += 1 ;
        let total_players_brought = room.participants[participant.1 as usize].total_players_brought ;
        self.connection.set::<_, _, ()>(room_id.clone(), serde_json::to_string(&room).unwrap()).await.expect("unable to set the updated value in update_balance_total_players_brought") ;
        Ok(total_players_brought)
    }

    pub async fn update_skip_count(&mut self, room_id: &str, skip_count: HashMap<i32,bool>) -> Result<(), redis::RedisError> {
        let mut room = self.get_room_details(room_id).await.expect("error getting room details from update_skip_count") ;
        room.skip_count = skip_count ;
        self.connection.set::<_, _, ()>(room_id.clone(), serde_json::to_string(&room).unwrap()).await.expect("unable to set the updated value in update_skip_count") ;
        Ok(())
    }
}



// -------------------------- Spawning the task for expiry bid logic ------------------------------------
use tokio_stream::StreamExt;
use redis::{Client, aio::PubSub};
use axum::extract::ws::{Message};
use futures_util::future::err;
use serde_json::Error;
use crate::models;
use crate::models::background_db_tasks::DBCommands;
use crate::models::room_models::Participant;
use crate::services::other::get_previous_team_full_name;

pub async fn listen_for_expiry_events(redis_url: &str, app_state: &Arc<AppState>) -> redis::RedisResult<()> {
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
        let mut sold = false ;
        tracing::info!("is rtm ---------------> {}", is_rtm) ;
        tracing::info!("room_id from that expiry key was {}", room_id);
        tracing::info!("we are going to use the key to broadcast") ;
        match conn.get::<_, Option<String>>(&room_id).await {
            Ok(Some(room)) => {
                let mut res: AuctionRoom = serde_json::from_str(&room).unwrap();
                let message;
                let participant_id = res.current_bid.clone().unwrap().participant_id ;
                let player_id = res.current_bid.clone().unwrap().player_id ;

                // here we are going to check whether the specific player having the previous team or not
                let previous_player = redis_connection.get_player(player_id, &room_id).await? ;
                let pause_status = res.pause ;
                if is_rtm == "rtms" {

                    // from know all redis operation will be done with out any data races , because

                    tracing::info!("the expired key was the RTM one") ;
                    // as it is expired, we are going to sell the player to the last bidded person
                    let bid = res.current_bid.clone().unwrap() ;
                    let mut details = get_participant_details(participant_id, &res.participants).expect("error getting participant details from increment_foreign_player_count") ;
                    let mut remaining_rtms = res.participants[details.1 as usize].remaining_rtms ;
                    if bid.is_rtm {
                        // we are going to update the rtms of the user
                       // redis_connection.update_remaining_rtms(room_id.clone(), participant_id).await?;
                        // we are going to update in the sql as well.
                        app_state.database_execute_task.send(DBCommands::UpdateRemainingRTMS(models::background_db_tasks::ParticipantId{
                            id: bid.participant_id
                        })).expect("Error while sending participant id for updating rtms to a unbounded channel") ;
                        remaining_rtms = redis_connection.update_remaining_rtms(&room_id, participant_id).await? ;
                    }
                    // we are going to sell the player to the person,
                    tracing::info!("player was a sold player") ;
                    app_state.database_execute_task.send(DBCommands::PlayerSold(models::background_db_tasks::SoldPlayer {
                        room_id: room_id.clone(),
                        player_id: bid.player_id,
                        participant_id: bid.participant_id,
                        bid_amount: bid.bid_amount
                    })).expect("Error While adding Player sold to the unbounded channel") ;
                    let remaining_balance = res.participants[details.1 as usize].balance -  res.current_bid.clone().unwrap().bid_amount;
                    let total_players_brought = redis_connection.update_balance_total_players_brought(&room_id, participant_id,remaining_balance).await.expect("error while updating balance and total players brought") ;
                    // updating the participant balance in the participant table
                    app_state.database_execute_task.send(DBCommands::BalanceUpdate(models::background_db_tasks::BalanceUpdate {
                        participant_id: bid.participant_id,
                        remaining_balance
                    })).expect("Error While update balance to the unbounded channel") ;
                    tracing::info!("successfully updated the balance in the psql") ;
                    tracing::info!("updating in the redis along with the balance and bid") ;
                    let bid = (Bid::new(0, 0,0.0,0.0, false, false)) ;
                    redis_connection.update_skip_count(&room_id, HashMap::new()).await.expect("") ;
                    // bid expiry zero means it will not use timer just update the current bid value
                    redis_connection.update_current_bid(&room_id, bid, 0).await.expect("") ;
                    let mut foreign_players_brought = details.0.foreign_players_brought ;
                    if !res.current_player.clone().unwrap().is_indian {
                        // details.0.foreign_players_brought += 1 ;
                        // foreign_players_brought = details.0.foreign_players_brought ; // getting updated foreign_players count
                        // res.participants[details.1 as usize] = details.0 ;
                        redis_connection.increment_foreign_player_count(&room_id, participant_id).await?;
                        foreign_players_brought += 1 ;
                    }
                    // let res = serde_json::to_string(&res).unwrap();
                    // conn.set::<_,_,()>(&room_id, res).await?;

                    sold = true ;
                    
                    broadcast_handler(Message::from(
                        serde_json::to_string(&SoldPlayer {
                            team_name: app_state.database_connection.get_team_name(participant_id).await.unwrap(),
                            sold_price: bid.bid_amount,
                            remaining_balance,
                            remaining_rtms,
                            foreign_players_brought
                        }).unwrap()
                    ), room_id.clone(), &app_state).await ;
                    

                }else{
                    tracing::info!("****************************") ;
                    let full_team_name = get_previous_team_full_name(&previous_player.previous_team) ;
                    let mut remaining_rtms = 0;
                    let mut previous_team_participant_id= 0 ;
                    for participant in res.participants.iter() {
                        tracing::info!("participant.teamname {} and previous_player.previous_team {}", participant.team_name, full_team_name) ;
                        if participant.team_name == full_team_name {
                            tracing::info!("previous team participant {}", participant.id) ;
                            previous_team_participant_id = participant.id ;
                            remaining_rtms = participant.remaining_rtms ;
                            tracing::info!("remaining_rtms {}", remaining_rtms) ;
                            break
                        }
                    }
                    let current_bid= res.current_bid.clone().unwrap() ;
                    tracing::info!("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx") ;
                    tracing::info!("previous team {}", previous_player.previous_team) ;
                    tracing::info!("remaining rtms {}",remaining_rtms) ;
                    tracing::info!("previous team {}", full_team_name) ;
                    tracing::info!("is rtm bid {}, it should be false",current_bid.rtm_bid) ;
                    tracing::info!("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx") ;
                    if ((!previous_player.previous_team.contains("-"))  && remaining_rtms > 0) && (!current_bid.rtm_bid)
                        && current_bid.participant_id > 0 && previous_team_participant_id != current_bid.participant_id
                        && !res.skip_count.contains_key(&previous_team_participant_id) && (res.current_player.clone().unwrap().is_indian || is_foreigner_allowed(previous_team_participant_id, &room_id, &mut redis_connection).await)
                    { // if it is rtm_bid means rtm was accepted such that the highest bidder willing to buy the player with the price quoted by the rtm team
                        tracing::info!("going to send the Use RTM") ;
                        // so we are going to create a new expiry key, and for that key there will be another subscriber
                        // now we are going to send the notification to the previous team to use the RTM, if he not uses it
                        // then this will expiry in 20 seconds.

                        send_message_to_participant(previous_team_participant_id, String::from("Use RTM"), room_id.clone(), &app_state).await ;

                        // setting the new timer
                        let rtm_timer_key = format!("auction:timer:rtms:{}", room_id); // if this key exists in the redis then no bids takes place
                        redis_connection.connection.set_ex::<_, _, ()>(&rtm_timer_key, "rtm", bid_expiry as u64).await.expect("unable to set the updated value in new_bid");
                        tracing::info!("we have successfully sent the message to the previous team, regarding RTM") ;
                        continue;
                    }else {

                            tracing::info!("we are going to update the balance of the participant") ;

                            let length ;
                            {
                                let rooms = &app_state.rooms;
                                let mut rooms_lock = rooms.read().await; // acquire mutable write lock
                                length = rooms_lock.get(&room_id).unwrap().len() ;
                            }
                            tracing::info!("length of room {}", length) ;
                            let mut remaining_balance: f32 = 0.0 ;
                            if current_bid.clone().bid_amount != 0.0 {
                                let mut details = get_participant_details(participant_id, &res.participants).expect("error getting participant details from increment_foreign_player_count") ;
                                remaining_balance = res.participants[details.1 as usize].balance -  res.current_bid.clone().unwrap().bid_amount;
                                let total_players_brought = redis_connection.update_balance_total_players_brought(&room_id, participant_id,remaining_balance).await.expect("error while updating balance and total players brought") ;
                                let bid = Bid::new(0, 0,0.0,0.0, false, false) ;
                                redis_connection.update_current_bid(&room_id, bid, 0).await.expect("") ;
                                let mut foreign_players_brought = details.0.foreign_players_brought ;
                                if !res.current_player.clone().unwrap().is_indian {
                                    tracing::info!("he is a foreign player, so updating foreign player count") ;
                                    redis_connection.increment_foreign_player_count(&room_id, participant_id).await?;
                                    foreign_players_brought += 1 ;
                                }
                                message= Message::from(
                                    serde_json::to_string(&SoldPlayer {
                                        team_name: details.0.team_name.clone(),
                                        sold_price: current_bid.clone().bid_amount,
                                        remaining_balance,
                                        remaining_rtms,
                                        foreign_players_brought
                                    }).unwrap()
                                );
                            }else{
                                message = Message::text("UnSold") ;
                            }

                            // making sure no skipped count
                            redis_connection.update_skip_count(&room_id, HashMap::new()).await.expect("") ;
                            tracing::info!("we are going to broadcast the message to the room participant") ;
                            broadcast_handler(message,room_id.clone(), &app_state ).await ;

                            // -------------------- over here we need to add the player to the sold player list with room-id and player-id
                            if current_bid.bid_amount != 0.0 {
                                tracing::info!("player was a sold player") ;
                                app_state.database_execute_task.send(DBCommands::PlayerSold(models::background_db_tasks::SoldPlayer {
                                    room_id: room_id.clone(),
                                    player_id: current_bid.player_id,
                                    participant_id: current_bid.participant_id,
                                    bid_amount: current_bid.bid_amount
                                })).expect("Error While adding Player sold to the unbounded channel") ;
                                // updating the participant balance in the participant table
                                app_state.database_execute_task.send(DBCommands::BalanceUpdate(models::background_db_tasks::BalanceUpdate {
                                    participant_id: current_bid.participant_id,
                                    remaining_balance
                                })).expect("Error While update balance to the unbounded channel") ;
                                tracing::info!("successfully updated the balance in the psql") ;
                            }else {
                                tracing::info!("player was an unsold player") ;
                                app_state.database_execute_task.send(DBCommands::PlayerUnSold(models::background_db_tasks::UnSoldPlayer {
                                    room_id: room_id.clone(),
                                    player_id: current_bid.player_id
                                })).expect("Error While adding Player Unsold to the unbounded channel") ;
                            }
                            sold = true ;
                    }

                }

                let message ;
                if sold  {
                    message = get_next_player(room_id.clone(), player_id, bid_expiry, pause_status).await ;
                }else {
                    message = Message::text("Auction was Paused");
                }

                broadcast_handler(message,room_id.clone(), &app_state ).await ;
            },
            Ok(None) => {
                tracing::warn!("room does not exist in redis") ;
                tracing::error!("As some one was already clicked the end button") ;
            },
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


pub async fn get_next_player(room_id: String, player_id: i32, bid_expiry: u8, pause_status: bool) -> Message {
    // we are going to get the next player and broadcasting the next player
    let next_player = player_id + 1 ;
    let mut redis_connection = RedisConnection::new().await ;
    let player: RedisResult<Player> = redis_connection.get_player(next_player, &room_id).await;
    let mut message ;
    match player {
        Ok(player) => {

            tracing::info!("now updating last player id") ;
            redis_connection.update_current_player(room_id.clone(), player.clone()).await.expect("unable to update last player id");
            // we are going to update the current bid
            message = Message::text("Auction was Paused") ;
            tracing::info!("auction was paused and updated last player in redis") ;
           if !pause_status {
               redis_connection.update_current_bid(&room_id, Bid::new(0, next_player, 0.0, player.base_price, false, false), bid_expiry).await.expect("unable to update current bid");
               tracing::info!("we are going to broadcast the next player, completed with updating current bid with new player") ;
               message = Message::from(serde_json::to_string(&player).unwrap()) ;
           }
        },
        Err(err) => {
            if err.kind() == redis::ErrorKind::TypeError
                && err.to_string().contains("Player not found")
            {
                message = Message::text("Auction Completed") ;
                tracing::warn!("Player with ID {} not found in Redis", next_player);
                // Handle "not found" case separately
            } else {
                tracing::error!("Redis error occurred: {:?}", err);
                tracing::warn!("error occurred while getting players") ;
                tracing::error!("error was {}", err) ;
                message = Message::text("Error Occurred while getting players from redis") ;
            }
        }
    };
    message
}