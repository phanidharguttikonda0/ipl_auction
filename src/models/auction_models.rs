use std::collections::HashMap;
use redis_derive::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use crate::models::app_state::Player;

#[derive(Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize)]
pub struct AuctionRoom {
    pub current_bid: Option<Bid>,
    pub participants: Vec<AuctionParticipant>,
    pub current_player: Option<Player>,
    pub pause: bool,
    pub skip_count: HashMap<i32, bool>, // if a participant has been skipped, or he has not clicked the skip button
    pub room_creator_id: i32
} //  this is where we are going to store in redis with a key as room_id and value as auction_room

impl AuctionRoom {
    pub fn new(room_creator_id: i32) -> Self {
        Self {
            current_bid: None,
            participants: Vec::new(),
            current_player: None,
            pause: false,
            skip_count: HashMap::new(),
            room_creator_id
        }
    }
    pub fn add_participant(&mut self, participant: AuctionParticipant) {
        self.participants.push(participant);
    }
}

#[derive(Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize)]
pub struct AuctionParticipant {
    pub id: i32, // participant id
    pub team_name: String, // team name
    pub balance: f32, // at start 100cr is the balance
    pub total_players_brought: u8,
    pub remaining_rtms: i16,
    pub is_unmuted: bool,
    pub foreign_players_brought: u8
}

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

#[derive(Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize)]
pub struct Bid {
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

