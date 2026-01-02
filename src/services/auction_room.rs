use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use redis::{AsyncCommands, Commands, RedisResult};
use crate::auction::{bid_allowance_handler, broadcast_handler, send_himself, send_message_to_participant};
use crate::models::app_state::{AppState, Player, PoolPlayer};
use crate::models::auction_models::{AuctionParticipant, Bid, RoomMeta, SoldPlayer};

#[derive(Debug, Clone)]
pub struct RedisConnection {
    pub connection: redis::aio::MultiplexedConnection,
}

impl RedisConnection {
    pub async fn new() -> Self {
        let connection_url = std::env::var("REDIS_URL").unwrap();
        Self {
            connection: Client::open(format!("redis://{}:6379/",connection_url)).unwrap().get_multiplexed_async_connection().await.unwrap(),
        }
    }

    pub async fn set_room_meta(&self, room_id: &str, room_meta: RoomMeta) -> Result<(), redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!("room:{}:meta", room_id);

        redis::cmd("HSET")
            .arg(&key)
            .arg("pause")
            .arg(if room_meta.pause { 1 } else { 0 })
            .arg("room_creator_id")
            .arg(room_meta.room_creator_id)
            .query_async::<i32>(&mut conn)
            .await?;

        Ok(())
    }
    pub async fn set_participant(&self, room_id: &str, participant: AuctionParticipant) -> Result<(), redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!(
            "room:{}:participant:{}:meta",
            room_id, participant.id
        );

        redis::cmd("HSET")
            .arg(&key)
            .arg("team_name")
            .arg(&participant.team_name)
            .arg("balance")
            .arg(participant.balance)
            .arg("total_players_brought")
            .arg(participant.total_players_brought)
            .arg("remaining_rtms")
            .arg(participant.remaining_rtms)
            .arg("is_unmuted")
            .arg(if participant.is_unmuted { 1 } else { 0 })
            .arg("foreign_players_brought")
            .arg(participant.foreign_players_brought)
            .query_async::<i32>(&mut conn)
            .await?;

        // 2. adding participant id to participants set
        let set_key = format!("room:{}:participants", room_id);

        redis::cmd("SADD")
            .arg(&set_key)
            .arg(participant.id)
            .query_async::<i32>(&mut conn)
            .await?;


        Ok(())
    }

    pub async fn get_participant(
        &self,
        room_id: &str,
        participant_id: i32,
    ) -> Result<Option<AuctionParticipant>, redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!("room:{}:participant:{}:meta", room_id, participant_id);

        // HMGET must return a tuple of Options
        let (team_name, balance, total_players_brought,
            remaining_rtms, is_unmuted_raw, foreign_players_brought)
            : (Option<String>, Option<f32>, Option<u8>, Option<i16>, Option<u8>, Option<u8>)
            = redis::cmd("HMGET")
            .arg(&key)
            .arg("team_name")
            .arg("balance")
            .arg("total_players_brought")
            .arg("remaining_rtms")
            .arg("is_unmuted")
            .arg("foreign_players_brought")
            .query_async(&mut conn)
            .await?;

        // If team_name is None, assume participant does not exist
        let Some(team_name) = team_name else {
            return Ok(None);
        };

        let participant = AuctionParticipant {
            id: participant_id,
            team_name,
            balance: balance.unwrap_or_default(),
            total_players_brought: total_players_brought.unwrap_or(0),
            remaining_rtms: remaining_rtms.unwrap_or(0),
            is_unmuted: is_unmuted_raw.unwrap_or(1) == 1,
            foreign_players_brought: foreign_players_brought.unwrap_or(0),
        };

        Ok(Some(participant))
    }


    pub async fn get_room_meta(
        &self,
        room_id: &str,
    ) -> Result<Option<RoomMeta>, redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!("room:{}:meta", room_id);

        // HMGET returns Option<T> for each field
        let (pause_raw, creator_id):
            (Option<i32>, Option<i32>) =
            redis::cmd("HMGET")
                .arg(&key)
                .arg("pause")
                .arg("room_creator_id")
                .query_async(&mut conn)
                .await?;

        // If pause is None, then meta does not exist
        let Some(pause_raw) = pause_raw else {
            return Ok(None);
        };

        // creator_id must exist, but default if missing
        let creator_id = creator_id.unwrap_or(0);

        let meta = RoomMeta {
            pause: pause_raw == 1,
            room_creator_id: creator_id,
        };

        Ok(Some(meta))
    }


    pub async fn set_pause(&self, room_id: &str, pause_status: bool) -> Result<(), redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!("room:{}:meta", room_id);

        redis::cmd("HSET")
            .arg(&key)
            .arg("pause")
            .arg(if pause_status { 1 } else { 0 })
            .query_async::<i32>(&mut conn)
            .await?;

        Ok(())

    }

    pub async fn set_current_player(&self, room_id: &str, player: Player) -> Result<(), redis::RedisError> {
        let mut conn = self.connection.clone();

        // Redis key
        let key = format!("room:{}:current_player", room_id);

        redis::cmd("HSET")
            .arg(&key)
            .arg("id")
            .arg(player.id)
            .arg("name")
            .arg(&player.name)
            .arg("base_price")
            .arg(player.base_price)
            .arg("country")
            .arg(&player.country)
            .arg("role")
            .arg(&player.role)
            .arg("previous_team")
            .arg(&player.previous_team)
            .arg("is_indian")
            .arg(if player.is_indian { 1 } else { 0 })
            .arg("pool_no")
            .arg(player.pool_no)
            .query_async::<i32>(&mut conn)
            .await?;

        Ok(())
    }

    pub async fn increment_foreign_player_count(&self, room_id: &str, participant_id: i32) -> Result<(), redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!(
            "room:{}:participant:{}:meta",
            room_id, participant_id
        );

        redis::cmd("HINCRBY")
            .arg(&key)
            .arg("foreign_players_brought")
            .arg(1)
            .query_async::<i32>(&mut conn)
            .await?;

        Ok(())
    }

    pub async fn increment_total_players_brought(&self, room_id: &str, participant_id: i32) -> Result<(), redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!(
            "room:{}:participant:{}:meta",
            room_id, participant_id
        );

        redis::cmd("HINCRBY")
            .arg(&key)
            .arg("total_players_brought")
            .arg(1)
            .query_async::<i32>(&mut conn)
            .await?;

        Ok(())
    }

    pub async fn update_balance(&self, room_id: &str, participant_id: i32, remaining_balance: f32) -> Result<(), redis::RedisError> {
        let mut conn = self.connection.clone();
        let key = format!("room:{}:participant:{}:meta", room_id, participant_id);
        redis::cmd("HSET")
            .arg(&key)
            .arg("balance")
            .arg(remaining_balance)
            .query_async::<i32>(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn decrement_rtm(&self, room_id: &str, participant_id: i32) -> Result<(), redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!("room:{}:participant:{}:meta", room_id, participant_id);
        let result: RedisResult<()> = redis::cmd("HINCRBY")
            .arg(&key)
            .arg("remaining_rtms")
            .arg(-1)
            .query_async::<()>(&mut conn)
            .await;
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub async fn toggle_mute(&self, room_id: &str, participant_id: i32, is_unmuted: bool) -> Result<(), redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!("room:{}:participant:{}:meta", room_id, participant_id);

        let result : RedisResult<()> = redis::cmd("HSET")
            .arg(&key)
            .arg("is_unmuted")
            .arg(if is_unmuted { 1 } else { 0 })
            .query_async::<()>(&mut conn)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub async fn list_participants(&self, room_id: &str) -> Result<Vec<i32>, redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!("room:{}:participants", room_id);

        // SMEMBERS returns Vec<T> directly
        let participants: Vec<i32> = redis::cmd("SMEMBERS")
            .arg(&key)
            .query_async::<Vec<i32>>(&mut conn)
            .await?;

        Ok(participants)
    }

    pub async fn get_current_player(
        &self,
        room_id: &str,
    ) -> Result<Option<Player>, redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!("room:{}:current_player", room_id);

        // HMGET returns tuple of Options
        let result: (Option<i32>, Option<String>, Option<f32>, Option<String>, Option<String>, Option<String>, Option<i32>, Option<String>, Option<i32>) =
            redis::cmd("HMGET")
                .arg(&key)
                .arg("id")
                .arg("name")
                .arg("base_price")
                .arg("country")
                .arg("role")
                .arg("previous_team")
                .arg("is_indian")
                .arg("profile_url")
                .arg("pool_no")
                .query_async(&mut conn)
                .await?;

        let (
            id,
            name,
            base_price,
            country,
            role,
            previous_team,
            is_indian_raw,
            profile_url,
            pool_no,
        ) = result;

        // If id is None → no player stored
        if id.is_none() {
            return Ok(None);
        }

        let player = Player {
            id: id.unwrap(),
            name: name.unwrap_or_default(),
            base_price: base_price.unwrap_or_default(),
            country: country.unwrap_or_default(),
            role: role.unwrap_or_default(),
            previous_team: previous_team.unwrap_or_default(),
            is_indian: is_indian_raw.unwrap_or(0) == 1,
            profile_url: profile_url.unwrap_or_default(),
            pool_no: pool_no.unwrap_or(0) as i16,
        };

        Ok(Some(player))
    }

    pub async fn get_current_bid(
        &self,
        room_id: &str,
    ) -> Result<Option<Bid>, redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!("room:{}:current_bid", room_id);

        let (participant_id, player_id, bid_amount, base_price, is_rtm, rtm_bid):
            (Option<i32>, Option<i32>, Option<f32>, Option<f32>, Option<i32>, Option<i32>)
            = redis::cmd("HMGET")
            .arg(&key)
            .arg("participant_id")
            .arg("player_id")
            .arg("bid_amount")
            .arg("base_price")
            .arg("is_rtm")
            .arg("rtm_bid")
            .query_async(&mut conn)
            .await?;

        // If no participant_id, then no current bid exists in Redis
        let Some(participant_id) = participant_id else {
            return Ok(None);
        };

        Ok(Some(Bid {
            participant_id,
            player_id: player_id.unwrap_or_default(),
            bid_amount: bid_amount.unwrap_or_default(),
            base_price: base_price.unwrap_or_default(),
            is_rtm: is_rtm.unwrap_or(0) == 1,
            rtm_bid: rtm_bid.unwrap_or(0) == 1,
        }))
    }


    pub async fn set_current_bid(&self, room_id: &str, bid: Bid) -> Result<(), redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!("room:{}:current_bid", room_id);

        redis::cmd("HSET")
            .arg(&key)
            .arg("participant_id")
            .arg(bid.participant_id)
            .arg("player_id")
            .arg(bid.player_id)
            .arg("bid_amount")
            .arg(bid.bid_amount)
            .arg("base_price")
            .arg(bid.base_price)
            .arg("is_rtm")
            .arg(if bid.is_rtm { 1 } else { 0 })
            .arg("rtm_bid")
            .arg(if bid.rtm_bid { 1 } else { 0 })
            .query_async::<()>(&mut conn)
            .await?;

        Ok(())
    }

    pub async fn update_current_bid(&self, room_id: &str, mut bid: Bid, bid_expiry: u8, participant_id: i32, strict_mode: bool, bid_should_not_increment: bool) -> Result<f32, String>{
        let mut conn = self.connection.clone();

        let key = format!("room:{}:current_bid", room_id);
        // we need to add the timer key here also
        let mut timer_key = format!("auction:timer:{}", room_id);
        let previous_bid_amount = bid.bid_amount;
        if bid.is_rtm {
            timer_key = format!("auction:timer:rtms:{}", room_id)
        }
        let mut next_bid_increment = bid.bid_amount;
        let mut allowed = true ;
        if bid.participant_id != 0 && !bid_should_not_increment { 
            // in cases where, bid is not supposed to increment like in rtm cases, where we infront increment the bid
            // and we will pass new bid , in such cases no need to increment and also in skip cases the highest bidder
            // will get the players, in such case also no need to increment, so this is very important in those cases.
            if previous_bid_amount == 0.0 {
                next_bid_increment = bid.base_price;
            } else if previous_bid_amount < 1.0 {
                // we are going to increment by 0.05
                next_bid_increment = 0.05;
            } else if previous_bid_amount < 10.0 {
                next_bid_increment = 0.10;
            } else {
                next_bid_increment = 0.25;
            }
            tracing::info!("incrementing bid by {}", next_bid_increment);
            next_bid_increment = round_two_decimals(bid.bid_amount + next_bid_increment);
            tracing::info!("after increment the bid amount was {}", next_bid_increment);
            bid.bid_amount = next_bid_increment;
            if participant_id != -1 {
                let participant = self.get_participant(room_id, participant_id).await.expect("team name not found").expect("no participant found");
                allowed = bid_allowance_handler(bid.bid_amount, participant.balance, participant.total_players_brought,strict_mode).await;
            }
        }
        if allowed {
            tracing::info!("the bid we are currently storing was {:?}", bid);
            self.set_current_bid(room_id, bid).await.expect("failed to set current bid");

            if bid_expiry != 0 {
                conn.set_ex::<_,_, ()>(&timer_key, "active", bid_expiry as u64).await.expect("failed to set timer key");
            }
            Ok(next_bid_increment)
        }else{
            let message ;
            if strict_mode {
                message = String::from("Bid not allowed, Check strict mode rules") ;
            }else {
                message = String::from("Bid not allowed") ;
            }
            Err(message)
        }
    }

    pub async fn check_room_existence(&self, room_id: &str) -> Result<bool, redis::RedisError> {
        let mut conn = self.connection.clone();

        let key = format!("room:{}:meta", room_id);

        let exists: i32 = redis::cmd("EXISTS")
            .arg(&key)
            .query_async::<i32>(&mut conn)
            .await?;

        Ok(exists == 1)
    }

    pub async fn reset_skip(&self, room_id: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.connection.clone();
        let key = format!("room:{}:skip_state", room_id);
        // Delete the Redis SET to remove all skipped participants
        redis::cmd("DEL")
            .arg(&key)
            .query_async::<()>(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn reset_skipped_pool(&self, room_id: &str) -> Result<(), redis::RedisError> {
        tracing::info!("resetting skipped pool");
        let mut conn = self.connection.clone();
        let key = format!("room:{}:skipped_pool", room_id);

        redis::cmd("DEL")
        .arg(&key)
        .query_async::<()>(&mut conn).await? ;

        Ok(())
    }

    pub async fn mark_skipped(&self, room_id: &str, participant_id: i32) -> Result<u8, redis::RedisError> {
        let mut conn = self.connection.clone();
        let key = format!("room:{}:skip_state", room_id);
        redis::cmd("SADD")
            .arg(&key)
            .arg(participant_id)
            .query_async::<()>(&mut conn).await?;

        let skip_count: u8 = self.get_skipped_count(room_id).await?;

        Ok(skip_count)
    }

    pub async fn mark_participant_skipped_pool(&self, room_id: &str, participant_id: i32) -> Result<(), redis::RedisError> {
        let mut conn = self.connection.clone();
        let key = format!("room:{}:skipped_pool", room_id);
        redis::cmd("SADD")
            .arg(&key)
            .arg(participant_id)
            .query_async::<()>(&mut conn).await?;

        Ok(())
    }

    pub async fn is_participant_skipped_pool(
        &self,
        room_id: &str,
        participant_id: i32,
    ) -> Result<bool, redis::RedisError> {
        let mut conn = self.connection.clone();
        let key = format!("room:{}:skipped_pool", room_id);

        let exists: bool = redis::cmd("SISMEMBER")
            .arg(&key)
            .arg(participant_id)
            .query_async(&mut conn)
            .await?;

        Ok(exists)
    }


    pub async fn get_skipped_pool_count(&self, room_id: &str) -> Result<u8, redis::RedisError> {
        let mut conn = self.connection.clone();
        let key = format!("room:{}:skipped_pool", room_id);
        let skip_count: u8 = redis::cmd("SCARD").arg(&key).query_async(&mut conn).await?;

        Ok(skip_count)
    }
    
    pub async fn get_skipped_count(&self, room_id: &str) -> Result<u8, redis::RedisError> {
        let mut conn = self.connection.clone();
        let key = format!("room:{}:skip_state", room_id);
        let skip_count: u8 = redis::cmd("SCARD")
            .arg(&key)
            .query_async::<u8>(&mut conn).await? ;
        Ok(skip_count)
    }

    pub async fn get_player_from_next_pool(&self, room_id: &str) -> Result<(i32,String), redis::RedisError> {
        let next_player = self.get_current_player(room_id).await?.unwrap();
        tracing::info!("*=* the next player was {:?}", next_player) ;
        if next_player.pool_no == 12 {
            return Ok((-1,"completed".to_string()))
        }
        let next_pool_no = next_player.pool_no + 1 ;
        let redis_key = format!("players_{}", next_pool_no);
        tracing::info!("*=* current pool {} next pool {}", next_pool_no, redis_key);
        let next_player = self.get_smallest_player_id_by_pool(next_pool_no)
        .await.unwrap().unwrap();
        Ok((next_player as i32, "".to_string()))
    }

    pub async fn get_smallest_player_id_by_pool(
        &self,
        pool_no: i16,
    ) -> Result<Option<i64>, String> {
        let mut conn = self.connection.clone();

        let zset_key = format!("players_{}:ids", pool_no);
        tracing::info!("getting smallest id from the pool_no {}", pool_no);
        let ids: Vec<i64> = conn
            .zrange(&zset_key, 0, 0)
            .await
            .map_err(|e| format!("Redis ZRANGE failed: {}", e))?;

        Ok(ids.first().copied())
    }


    pub async fn is_skipped(&self, room_id: &str, participant_id: i32 ) -> Result<bool, redis::RedisError> {
        let mut conn = self.connection.clone();
        let key = format!("room:{}:skip_state", room_id);
        let result: RedisResult<bool> = redis::cmd("SISMEMBER").arg(&key).arg(participant_id).query_async::<bool>(&mut conn).await;
        match result {
            Ok(is_skipped) => Ok(is_skipped),
            Err(err) => Err(err),
        }
    }

    pub async fn check_participant(&self, room_id: &str, participant_id: i32) -> Result<bool, redis::RedisError>{
        let mut conn = self.connection.clone();
        let key = format!("room:{}:participants", room_id);
        let result: RedisResult<bool> = redis::cmd("SISMEMBER").arg(&key).arg(participant_id).query_async::<bool>(&mut conn).await;
        match result {
            Ok(is_participant) => Ok(is_participant),
            Err(err) => Err(err),
        }
    }


    pub async fn load_players_to_redis(
        &self,
        players: Vec<Player>,
    ) -> Result<(), String> {
        let mut conn = self.connection.clone();

        // 1️⃣ Group players by pool
        let mut pool_map: std::collections::HashMap<i16, Vec<Player>> =
            std::collections::HashMap::new();

        for player in players {
            pool_map
                .entry(player.pool_no)
                .or_insert_with(Vec::new)
                .push(player);
        }

        // 2️⃣ Iterate over each pool
        for (pool_no, pool_players) in pool_map {
            let hash_key = format!("players_{}", pool_no);
            let zset_key = format!("players_{}:ids", pool_no);

            // 3️⃣ Check if pool already exists
            let exists: bool = conn
                .exists(&hash_key)
                .await
                .map_err(|e| format!("Redis EXISTS failed: {}", e))?;

            if exists {
                tracing::info!(
                "⏭️ Pool '{}' already exists, skipping",
                hash_key
            );
                continue;
            }

            tracing::info!(
            "✅ Loading {} players into '{}'",
            pool_players.len(),
            hash_key
        );

            // 4️⃣ Insert players into HASH + ZSET
            for player in pool_players {
                let player_json = serde_json::to_string(&player)
                    .map_err(|e| format!("Serialize error: {}", e))?;

                // Store player data
                conn.hset::<_, _, _, ()>(
                    &hash_key,
                    player.id,
                    player_json,
                )
                    .await
                    .map_err(|e| format!("Redis HSET failed: {}", e))?;

                // Store sorted player_id
                conn.zadd::<_, _, _, ()>(
                    &zset_key,
                    player.id,
                    player.id as f64,
                )
                    .await
                    .map_err(|e| format!("Redis ZADD failed: {}", e))?;
            }
        }

        Ok(())
    }




    pub async fn get_player(&self, player_id: i32, room_id: &str) -> Result<Player, redis::RedisError> {
        tracing::info!("get player was called, getting player from redis") ;
        let mut conn = self.connection.clone();

        // 1️⃣ If room_id has a current player → return immediately
        if !room_id.is_empty() {
            if let Some(current_player) = self.get_current_player(room_id).await? {
                tracing::info!("current-player exists, returning it without checking Redis pools");
                if current_player.id == player_id {
                    return Ok(current_player);
                }
            } else {
                tracing::info!("no current player found for this room, checking Redis pools");
            }
        }

        // 2️⃣ Loop pools 1 to 12
        for pool_no in 1..=12 {
            let redis_key = format!("players_{}", pool_no);

            tracing::info!("checking the pool_no {}", pool_no) ;
            // Check if the player exists in this pool
            let exists: bool = conn
                .hexists::<_, _, bool>(&redis_key, player_id)
                .await
                .map_err(|e| {
                    tracing::error!("Redis HEXISTS error: {:?}", e);
                    e
                })?;

            if exists {
                tracing::info!("Player {} found in Redis key '{}'", player_id, redis_key);

                // Fetch the player JSON
                let json: String = conn
                    .hget(&redis_key, player_id)
                    .await
                    .map_err(|e| {
                        tracing::error!("Redis HGET error: {:?}", e);
                        e
                    })?;

                // Deserialize into Player struct
                let player: Player = serde_json::from_str(&json)
                    .map_err(|_| {
                        redis::RedisError::from((
                            redis::ErrorKind::TypeError,
                            "Invalid JSON for player object"
                        ))
                    })?;

                return Ok(player);
            }
        }

        // 3️⃣ If we reach here → player was not found
        tracing::warn!("Player ID {} not found in any of the 12 Redis pools", player_id);

        Err(redis::RedisError::from((
            redis::ErrorKind::TypeError,
            "Player not found"
        )))
    }


    pub async fn get_players_by_pool(
        &self,
        pool_no: i16,
    ) -> Result<Vec<PoolPlayer>, redis::RedisError> {
        tracing::info!("get players by pool was called, getting players from redis") ;
        let mut conn = self.connection.clone();
        let redis_key = format!("players_{}", pool_no);

        // 1️⃣ Check if pool exists
        let exists: bool = conn.exists(&redis_key).await?;
        if !exists {
            tracing::warn!("Pool {} not found in Redis", pool_no);
            return Ok(Vec::new()); // empty pool, not an error
        }

        // 2️⃣ Get all values (JSON strings) from Redis hash
        let players_json: Vec<String> = conn.hvals(&redis_key).await?;

        // 3️⃣ Deserialize + map to PoolPlayer
        let mut pool_players = Vec::with_capacity(players_json.len());
        tracing::info!("players_json length is {}", players_json.len()) ;
        for json in players_json {
            let player: Player = serde_json::from_str(&json).map_err(|_| {
                redis::RedisError::from((
                    redis::ErrorKind::TypeError,
                    "Invalid Player JSON in Redis",
                ))
            })?;

            pool_players.push(PoolPlayer {
                id: player.id,
                name: player.name,
                base_price: player.base_price as f32,
                country: player.country,
                role: player.role,
                previous_team: player.previous_team,
                is_indian: player.is_indian,
            });
        }

        Ok(pool_players)
    }




    pub async fn auction_clean_up(
        &self,
        room_id: &str,
    ) -> Result<bool, redis::RedisError> {
        let mut conn = self.connection.clone();

        // Pattern for all room keys
        let pattern = format!("room:{}:*", room_id);

        // SCAN cursor
        let mut cursor: u64 = 0;

        let mut keys_to_delete = Vec::new();

        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(200)
                .query_async(&mut conn)
                .await?;

            cursor = new_cursor;

            keys_to_delete.extend(keys);

            if cursor == 0 {
                break;
            }
        }

        if keys_to_delete.is_empty() {
            return Ok(false); // nothing to delete
        }

        // Delete all keys in one atomic pipeline
        let mut pipe = redis::pipe();
        pipe.atomic();

        for key in keys_to_delete {
            pipe.cmd("DEL").arg(key);
        }

        // Execute pipeline
        pipe.query_async::<()>(&mut conn).await?;

        Ok(true)
    }


    pub async fn atomic_delete(&self, key: &str) -> redis::RedisResult<i32> {
        let mut conn = self.connection.clone();
        let script = r#"
        local existed = redis.call('DEL', KEYS[1])
        return existed
    "#;

        let result: i32 = redis::Script::new(script)
            .key(key)
            .invoke_async(&mut conn)
            .await?;

        Ok(result)
    }

    pub async fn check_key_exists(&self, key: &str) -> Result<bool, redis::RedisError> {
        tracing::info!("checking key exists redis function was called");

        let mut conn = self.connection.clone();

        // EXISTS returns bool in redis-rs async API
        let exists: bool = conn.exists(key).await?;
        tracing::info!("the key exists {}",exists);
        Ok(exists)
    }
    // this is enough to check whether the participant or the room exists or not


    pub async fn add_retry_task(&self, val: &DBCommandsAuctionRoom, app_state: &AppState) -> Result<(), redis::RedisError> {

        let retry_count = match val.clone() {
            DBCommandsAuctionRoom::PlayerSold(player_sold) => {

                player_sold.retry_count
            },
            DBCommandsAuctionRoom::BalanceUpdate(balance_update) => {
                balance_update.retry_count
            },
            DBCommandsAuctionRoom::PlayerUnSold(unsold_player) => {
                unsold_player.retry_count
            },
            DBCommandsAuctionRoom::UpdateRoomStatus(room_status_update) => {
                room_status_update.retry_count
            },
            DBCommandsAuctionRoom::UpdateRemainingRTMS(remaining_rtms) => {
                remaining_rtms.retry_count
            },
            DBCommandsAuctionRoom::CompletedRoomCompletedAt(completed_room) => {
                completed_room.retry_count
            },
            DBCommandsAuctionRoom::CompletedRoomUnsoldPlayers(completed_room) => {
                completed_room.retry_count
            },
            DBCommandsAuctionRoom::CompletedRoomSoldPlayers(completed_room) => {
                completed_room.retry_count
            }
        } ;

        let retry_delay = match retry_count{
            0 => 12,  // 2 minutes
            1 => 30,  // 5 minutes
            2 => 60,  // 10 minutes
            _ => {
                tracing::info!("moving to Dead Letter Queue, all retries are exhausted") ;

                // moving to DLQ
                app_state.dlq_task_executor.send(val.clone()).unwrap() ;
                return Ok(())
            }
        };



        let retry_at = Utc::now().timestamp() + retry_delay;
        let mut conn = self.connection.clone();
        let _: usize = conn
            .zadd("auction:retry:zset", serde_json::to_string(val).unwrap(), retry_at)
            .await?;

        Ok(())
    }

}



// -------------------------- Spawning the task for expiry bid logic ------------------------------------
use tokio_stream::StreamExt;
use redis::{Client};
use axum::extract::ws::{Message};
use chrono::Utc;
use crate::models;
use crate::models::background_db_tasks::{CompletedRoom, DBCommandsAuctionRoom, ParticipantId, RoomStatus, UnSoldPlayer};
use crate::services::other::get_previous_team_full_name;

pub async fn listen_for_expiry_events(redis_url: &str, app_state: &Arc<AppState>) -> redis::RedisResult<()> {
    tracing::info!("🔔 Redis expiry listener started");
    let client = Client::open(redis_url)?;
    let mut pubsub = client.get_async_pubsub().await?;
    let mut conn = client.get_multiplexed_async_connection().await?;
    // Subscribe to key event notifications for expired keys
    pubsub.subscribe("__keyevent@0__:expired").await?;

    let redis_connection = app_state.redis_connection.clone();

    let bid_expiry = std::env::var("BID_EXPIRY").unwrap().parse::<u8>().unwrap();

    let mut stream = pubsub.on_message();
    while let Some(msg) = stream.next().await {
        let expired_key: String = msg.get_payload()?;
        tracing::info!("Key expired: {}", expired_key);
        let parts: Vec<&str> = expired_key.split(':').collect();

        let room_id = parts.get(parts.len() - 1).unwrap_or(&"").to_string();
        let is_rtm = parts.get(parts.len() - 2).unwrap_or(&"").to_string();
        handling_expiry_events(&app_state,&room_id, is_rtm, bid_expiry).await ;
    }

    Ok(())
}



#[tracing::instrument(
    name = "handling_expiry_events",
    skip(app_state),
    fields(
        room_id = %room_id,
        is_rtm_key = is_rtm,
        bid_expiry = bid_expiry
    )
)]
pub async fn handling_expiry_events(app_state: &Arc<AppState>, room_id: &str,is_rtm: String, bid_expiry: u8) {
    let mut sold = false ;
    let redis_connection = app_state.redis_connection.clone() ;
    tracing::info!("is rtm ---------------> {}", is_rtm) ;
    tracing::info!("room_id from that expiry key was {}", room_id);
    tracing::info!("we are going to use the key to broadcast") ;

    let current_bid = redis_connection.get_current_bid(room_id).await.unwrap().unwrap();
    let room_meta = redis_connection.get_room_meta(room_id).await.unwrap() ;
    let room_meta: RoomMeta = match room_meta {
        Some(room_meta) => room_meta,
        None => { return; }
    } ;
    let message;
    let participant_id = current_bid.clone().participant_id ;
    let player_id = current_bid.clone().player_id ;

    // here we are going to check whether the specific player having the previous team or not
    let current_player = redis_connection.get_current_player(room_id).await.unwrap();
    let current_player = match current_player {
        Some(current_player) => current_player,
        None => {
            tracing::warn!("No current player") ;
            return;
        }
    } ;
    let pause_status = room_meta.pause ;
    if is_rtm == "rtms" {

        // from know all redis operation will be done with out any data races , because
        tracing::info!("the expired key was the RTM one") ;
        // as it is expired, we are going to sell the player to the last bidded person
        let bid = current_bid.clone() ;
        let details = redis_connection.get_participant(room_id, bid.participant_id).await.unwrap() ;
        let mut participant = match details {
            Some(details) => details,
            None => {
                return ;
            }
        } ;
        let mut remaining_rtms = participant.remaining_rtms ;
        if bid.is_rtm {
            // we are going to update the rtms of the user
            // redis_connection.update_remaining_rtms(room_id.clone(), participant_id).await?;
            // we are going to update in the sql as well.
            app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::UpdateRemainingRTMS(models::background_db_tasks::ParticipantId{
                id: bid.participant_id,
                retry_count: 0,
                last_error: String::from("")
            })).expect("Error while sending participant id for updating rtms to a unbounded channel") ;
            redis_connection.decrement_rtm(room_id, participant_id).await.unwrap() ;
            remaining_rtms -= 1 ;
        }
        // we are going to sell the player to the person,
        tracing::info!("player was a sold player") ;
        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::PlayerSold(models::background_db_tasks::SoldPlayer {
            room_id: room_id.to_string(),
            player_id: bid.player_id,
            participant_id: bid.participant_id,
            bid_amount: bid.bid_amount,
            retry_count: 0,
            last_error: String::from("")
        })).expect("Error While adding Player sold to the unbounded channel") ;
        let remaining_balance = round_two_decimals(participant.balance -  bid.bid_amount);
        redis_connection.increment_total_players_brought(room_id, participant_id).await.expect("error while updating total players brought") ;
        let total_players_brought = participant.total_players_brought + 1 ;
        redis_connection.update_balance(room_id, participant_id, remaining_balance).await.expect("error while updating balance") ;
        /*
            for a new player brought, we need to update balance total players brought, foreign player
        */
        // updating the participant balance in the participant table
        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::BalanceUpdate(models::background_db_tasks::BalanceUpdate {
            participant_id: bid.participant_id,
            remaining_balance,
            retry_count: 0,
            last_error: String::from("")
        })).expect("Error While update balance to the unbounded channel") ;
        tracing::info!("successfully updated the balance in the psql") ;
        tracing::info!("updating in the redis along with the balance and bid") ;
        let bid_ = Bid::new(0, 0,0.0,0.0, false, false) ;
        redis_connection.reset_skip(room_id).await.expect("error while resetting skip_count") ;
        // bid expiry zero means it will not use timer just update the current bid value
        redis_connection.update_current_bid(room_id, bid_, 0, -1, true, false).await.expect("") ;
        let mut foreign_players_brought = participant.foreign_players_brought ;

        if !current_player.is_indian {
            // details.0.foreign_players_brought += 1 ;
            // foreign_players_brought = details.0.foreign_players_brought ; // getting updated foreign_players count
            // res.participants[details.1 as usize] = details.0 ;
            redis_connection.increment_foreign_player_count(room_id, participant_id).await.unwrap();
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
        ), room_id, &app_state).await ;


    }else{
        tracing::info!("****************************") ;
        let full_team_name = get_previous_team_full_name(&current_player.previous_team) ;
        /*
            we need to get the remaining rtms of the previous team and participant_id
        */
        let mut remaining_rtms = 0; // remaining_rtms of previous player
        let mut previous_team_participant_id= 0 ;
        let mut previous_team_foreign_player_count = 0 ;
        let mut list_of_participants = redis_connection.list_participants(room_id).await.expect("error while getting list of participants") ;
        for participant_id in list_of_participants.iter() {
            let participant = redis_connection.get_participant(room_id, *participant_id).await.expect("error while getting participant from the participant-id") ;
            if let Some(participant) = participant {
                tracing::info!("participant.teamname {} and previous_player.previous_team {}", participant.team_name, full_team_name) ;
                if participant.team_name == full_team_name {
                    tracing::info!("previous team participant {}", participant.id) ;
                    previous_team_participant_id = participant.id ;
                    remaining_rtms = participant.remaining_rtms ;
                    previous_team_foreign_player_count = participant.foreign_players_brought ;
                    tracing::info!("remaining_rtms {}", remaining_rtms) ;
                    break
                }
            }else{
                continue
            }
        }
        let current_bid= current_bid.clone() ;
        tracing::info!("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx") ;
        tracing::info!("previous team {}", current_player.previous_team) ;
        tracing::info!("remaining rtms {}",remaining_rtms) ;
        tracing::info!("previous team {}", full_team_name) ;
        tracing::info!("is rtm bid {}, it should be false",current_bid.rtm_bid) ;
        tracing::info!("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx") ;
        if ((!current_player.previous_team.contains("-"))  && remaining_rtms > 0) && (!current_bid.rtm_bid)
            && current_bid.participant_id > 0 && previous_team_participant_id != current_bid.participant_id
            && !redis_connection.is_skipped(room_id,previous_team_participant_id).await.expect("") && (current_player.clone().is_indian || previous_team_foreign_player_count < 8)
        { // if it is rtm_bid means rtm was accepted such that the highest bidder willing to buy the player with the price quoted by the rtm team
            tracing::info!("going to send the Use RTM") ;
            // so we are going to create a new expiry key, and for that key there will be another subscriber
            // now we are going to send the notification to the previous team to use the RTM, if he not uses it
            // then this will expiry in 20 seconds.

            send_message_to_participant(previous_team_participant_id, String::from("Use RTM"), room_id, &app_state).await ;

            // setting the new timer
            let rtm_timer_key = format!("auction:timer:rtms:{}", room_id); // if this key exists in the redis then no bids takes place
            let mut conn = redis_connection.connection.clone() ;
            conn.set_ex::<_, _, ()>(&rtm_timer_key, "rtm", bid_expiry as u64).await.expect("unable to set the updated value in new_bid");
            tracing::info!("we have successfully sent the message to the previous team, regarding RTM") ;
            return;
        }else {

            tracing::info!("we are going to update the balance of the participant") ;

            let length ;
            {
                let rooms = &app_state.rooms;
                let mut rooms_lock = rooms.read().await; // acquire mutable write lock
                length = rooms_lock.get(room_id).unwrap().len() ;
            }

            tracing::info!("length of room {}", length) ;
            let mut remaining_balance: f32 = 0.0 ;
            if current_bid.bid_amount != 0.0 {
                let participant = redis_connection.get_participant(room_id, current_bid.participant_id).await.unwrap() ;
                let participant = match participant {
                    Some(participant) => participant,
                    None => {
                        return;
                    }
                };
                remaining_balance = participant.balance -  current_bid.bid_amount;
                remaining_rtms = participant.remaining_rtms ;
                let total_players_brought = participant.total_players_brought + 1 ;
                redis_connection.increment_total_players_brought(room_id, participant_id).await.expect("error while updating total players brought") ;
                redis_connection.update_balance(room_id, participant_id, remaining_balance).await.expect("error while updating balance") ;

                let bid = Bid::new(0, 0,0.0,0.0, false, false) ;
                redis_connection.update_current_bid(room_id, bid, 0, -1, true, false).await.expect("error while updating the current bid") ;
                let mut foreign_players_brought = participant.foreign_players_brought ;
                if !current_player.is_indian {
                    tracing::info!("he is a foreign player, so updating foreign player count") ;
                    redis_connection.increment_foreign_player_count(room_id, participant_id).await.unwrap();
                    foreign_players_brought += 1 ;
                }
                message= Message::from(
                    serde_json::to_string(&SoldPlayer {
                        team_name: participant.team_name.clone(),
                        sold_price: current_bid.bid_amount,
                        remaining_balance,
                        remaining_rtms,
                        foreign_players_brought
                    }).unwrap()
                );
            }else{
                message = Message::text("UnSold") ;
            }

            // making sure no skipped count
            redis_connection.reset_skip(room_id).await.expect("error while reseting the skip count") ;
            tracing::info!("we are going to broadcast the message to the room participant") ;
            broadcast_handler(message,room_id, &app_state ).await ;

            // -------------------- over here we need to add the player to the sold player list with room-id and player-id
            if current_bid.bid_amount != 0.0 {
                tracing::info!("player was a sold player") ;
                app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::PlayerSold(models::background_db_tasks::SoldPlayer {
                    room_id: room_id.to_string(),
                    player_id: current_bid.player_id,
                    participant_id: current_bid.participant_id,
                    bid_amount: current_bid.bid_amount,
                    retry_count: 0,
                    last_error: String::from("")
                })).expect("Error While adding Player sold to the unbounded channel") ;
                // updating the participant balance in the participant table
                app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::BalanceUpdate(models::background_db_tasks::BalanceUpdate {
                    participant_id: current_bid.participant_id,
                    remaining_balance,
                    retry_count: 0,
                    last_error: String::from("")
                })).expect("Error While update balance to the unbounded channel") ;
                tracing::info!("successfully updated the balance in the psql") ;
            }else {
                tracing::info!("player was an unsold player") ;
                app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::PlayerUnSold(models::background_db_tasks::UnSoldPlayer {
                    room_id: room_id.to_string(),
                    player_id: current_bid.player_id,
                    retry_count: 0,
                    last_error: String::from("")
                })).expect("Error While adding Player Unsold to the unbounded channel") ;
            }
            sold = true ;
        }

    }

    let message ;
    if sold  {
        message = get_next_player(room_id, player_id, bid_expiry, pause_status, &app_state).await ;
    }else {
        message = Message::text("Auction was Paused");
    }

    broadcast_handler(message,room_id, &app_state ).await ;

}

#[tracing::instrument(
    name = "getting_next_player",
    fields(
        room_id = %room_id,
        player_id = player_id,
        pause_status = pause_status,
        bid_expiry = bid_expiry
    )
)]
pub async fn get_next_player(room_id: &str, player_id: i32, bid_expiry: u8, pause_status: bool, app_state: &Arc<AppState>) -> Message {
    // we are going to get the next player and broadcasting the next player
    let mut next_player = player_id + 1 ;
    let mut redis_connection = app_state.redis_connection.clone();
    let participants_count = app_state
    .rooms
    .read()
    .await
    .get(room_id)
    .unwrap()
    .len() ;
    tracing::info!("*=* participants count is {}", participants_count) ;
    let skipped_pool_count = app_state.redis_connection.get_skipped_pool_count(room_id).await.unwrap() as usize ;
    tracing::info!("*=* skipped pool count is {}", skipped_pool_count) ;
    if  participants_count == skipped_pool_count {
        tracing::info!("skipped pool count is equal to the number of participants") ;
        let result  = redis_connection.get_player_from_next_pool(room_id).await.map_err(
            |e| {
                tracing::error!("error while getting next player_id from next pool room_id {}", room_id) ;
            }
        ).unwrap() ;
        redis_connection.reset_skipped_pool(room_id).await.expect("error while resetting skipped pool") ;
        if result.0 == -1 {
           tracing::info!("last pool cannot be skipped");

        }else {
            next_player = result.0 ;
        }

    }
    tracing::info!("*=* next player id is {}", next_player) ;
    let player: RedisResult<Player> = redis_connection.get_player(next_player, room_id).await;
    let mut message ;
    match player {
        Ok(player) => {

            tracing::info!("now updating last player id") ;
            redis_connection.set_current_player(room_id, player.clone()).await.expect("unable to update last player id");
            // we are going to update the current bid
            message = Message::text("Auction was Paused") ;
            tracing::info!("auction was paused and updated last player in redis") ;
           if !pause_status {
               redis_connection.update_current_bid(room_id, Bid::new(0, next_player, 0.0, player.base_price, false, false), bid_expiry, -1, true, false).await.expect("unable to update current bid");
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

fn round_two_decimals(value: f32) -> f32 {
    (value * 100.0).round() / 100.0
}
