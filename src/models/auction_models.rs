use std::collections::HashMap;
use redis_derive::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use crate::models::app_state::Player;


#[derive(Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize)]
pub struct AuctionParticipant {
    pub id: i32, // participant id
    pub team_name: String, // team name
    pub balance: f32, // at start 100cr is the balance
    pub total_players_brought: u8,
    pub remaining_rtms: i16,
    pub is_unmuted: bool,
    pub foreign_players_brought: u8
} // room_id:participant_id:meta is the key to get the participant from redis

impl AuctionParticipant {
    pub fn new(id: i32, team_name: String, remaining_rtms: i16) -> Self {
        Self {
            id,
            team_name,
            balance: 100.0,
            total_players_brought: 0,
            remaining_rtms,
            is_unmuted: true,
            foreign_players_brought: 0
        }
    }
}

/*

    For Current Player -> room_id:current_player

*/

pub type SkipState = HashMap<i32, bool>; //  room_id:skip_state is the key to get the skip state from redis

#[derive(Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize)]
pub struct Bid { // room_id:current_bid is the key to get the current bid from redis
    pub participant_id: i32,
    pub player_id: i32,
    pub bid_amount: f32,
    pub base_price: f32,
    pub is_rtm: bool,
    pub rtm_bid: bool // only in rtm-accept case it will be true
}

impl Bid {
    pub fn new(participant_id: i32, player_id: i32, bid_amount: f32, base_price: f32, is_rtm: bool, rtm_bid: bool) -> Self {
        Bid {
            participant_id,
            player_id,
            bid_amount,
            base_price,
            is_rtm,
            rtm_bid,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomMeta {
    pub pause: bool,
    pub room_creator_id: i32,
} // meta data of the room -> room_id:meta is the key to get the meta data from redis




#[derive(Debug,Clone, Serialize, Deserialize)]
pub struct BidOutput {
    pub team : String,
    pub bid_amount: f32,
}

#[derive(Debug,Clone, Serialize, Deserialize)]
pub struct SoldPlayer {
    pub team_name: String,
    pub sold_price: f32,
    pub(crate) remaining_balance: f32,
    pub remaining_rtms: i16,
    pub foreign_players_brought: u8
}

#[derive(Debug,Clone, Serialize, Deserialize)]
pub struct NewJoiner {
    pub participant_id: i32,
    pub team_name: String,
    pub balance: f32,
}

#[derive(Debug,Clone, Serialize, Deserialize)]
pub struct ParticipantAudio {
    pub participant_id: i32,
    pub is_unmuted: bool
}

#[derive(Debug,Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub team_name: String,
    pub message: String
}