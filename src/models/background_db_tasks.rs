

#[derive(Serialize, Deserialize, Clone)]
pub struct ParticipantId {
    pub id: i32,
    pub retry_count: u8
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SoldPlayer {
    pub room_id: String,
    pub player_id: i32,
    pub participant_id: i32,
    pub bid_amount: f32,
    pub retry_count: u8
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UnSoldPlayer {
    pub player_id: i32,
    pub room_id: String,
    pub retry_count: u8
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BalanceUpdate {
    pub participant_id: i32,
    pub remaining_balance: f32,
    pub retry_count: u8
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RoomStatus {
    pub room_id: String,
    pub status: String,
    pub retry_count: u8
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CompletedRoom {
    pub room_id: String,
    pub retry_count: u8
}

#[derive(Serialize, Deserialize, Clone)]
pub enum DBCommandsAuctionRoom { //inside auction room db tasks will be executed by this
    UpdateRemainingRTMS(ParticipantId),
    PlayerSold(SoldPlayer),
    PlayerUnSold(UnSoldPlayer),
    BalanceUpdate(BalanceUpdate),
    UpdateRoomStatus(RoomStatus),
    CompletedRoom(CompletedRoom),
}


#[derive(Serialize, Deserialize, Clone)]
pub enum RetryTask {
    UpdateRemainingRTMS { participant_id: i32, retry_count: u8 },
    PlayerSold {
        room_id: String,
        player_id: i32,
        participant_id: i32,
        bid_amount: f32,
        retry_count: u8
    },
    PlayerUnSold {
        room_id: String,
        player_id: i32,
        retry_count: u8
    },
    BalanceUpdate {
        participant_id: i32,
        remaining_balance: f32,
        retry_count: u8
    },
    UpdateRoomStatus {
        room_id: String,
        status: String,
        retry_count: u8
    },
    CompletedRoom {
        room_id: String,
        retry_count: u8
    },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RetryEnvelope { // this is the envelope we are going to store in the redis as the key
    pub task: RetryTask,
    pub retry_count: u8,
    pub last_error: String,
}


pub struct UserExternalDetails {
    pub user_id: i32,
    pub ip_address: String
}

pub struct FavoriteTeamUpdated {
    pub user_id: i32,
    pub old_favorite_team: String,
    pub new_favorite_team: String
}
pub enum DBCommandsAuction {
    AddUserExternalDetails(UserExternalDetails),
    FavoriteTeamUpdated(FavoriteTeamUpdated)
}

use redis_derive::FromRedisValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct IpInfoResponse {
    pub city: Option<String>,
    pub region: Option<String>,
    pub postal: Option<String>,
    pub country: Option<String>,
}
