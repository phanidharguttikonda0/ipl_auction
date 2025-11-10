use redis_derive::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};

#[derive(Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize)]
pub struct AuctionRoom {
    pub current_bid: Option<Bid>,
    pub participants: Vec<AuctionParticipant>,
} //  this is where we are going to store in redis with key as room_id and value as auction_room

impl AuctionRoom {
    pub fn new() -> Self {
        Self {
            current_bid: None,
            participants: Vec::new(),
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
}

impl AuctionParticipant {
    pub fn new(id: i32, team_name: String) -> Self {
        Self {
            id,
            team_name,
            balance: 100.0,
            total_players_brought: 0,
        }
    }
}

#[derive(Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize)]
pub struct Bid {
    pub participant_id: i32,
    pub player_id: i32,
    pub bid_amount: f32,
    pub base_price: f32,
}

impl Bid {
    pub fn new(participant_id: i32, player_id: i32, bid_amount: f32, base_price: f32) -> Self {
        Bid {
            participant_id,
            player_id,
            bid_amount,
            base_price,
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
}