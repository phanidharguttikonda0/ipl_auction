

pub trait AuctionRoomRetryTasks {}

#[derive(Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct ParticipantId {
    pub id: i32,
    pub retry_count: u8,
    pub last_error: String
}

impl AuctionRoomRetryTasks for ParticipantId {}
impl AuctionRoomRetryTasks for SoldPlayer {}
impl AuctionRoomRetryTasks for UnSoldPlayer {}
impl AuctionRoomRetryTasks for BalanceUpdate {}
impl AuctionRoomRetryTasks for RoomStatus {}
impl AuctionRoomRetryTasks for CompletedRoom {}

#[derive(Serialize, Deserialize,sqlx::FromRow, Clone)]
pub struct SoldPlayer {
    pub room_id: String,
    pub player_id: i32,
    pub participant_id: i32,
    pub bid_amount: f32,
    pub retry_count: u8,
    pub last_error: String
}

#[derive(Serialize, Deserialize, sqlx::FromRow,Clone)]
pub struct UnSoldPlayer {
    pub player_id: i32,
    pub room_id: String,
    pub retry_count: u8,
    pub last_error: String
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct BalanceUpdate {
    pub participant_id: i32,
    pub remaining_balance: f32,
    pub retry_count: u8,
    pub last_error: String
}

#[derive(Serialize, Deserialize,sqlx::FromRow, Clone)]
pub struct RoomStatus {
    pub room_id: String,
    pub status: String,
    pub retry_count: u8,
    pub last_error: String
}

#[derive(Serialize, Deserialize,sqlx::FromRow, Clone)]
pub struct CompletedRoom {
    pub room_id: String,
    pub retry_count: u8,
    pub last_error: String
}

#[derive(Serialize, Deserialize, Clone)]
pub enum DBCommandsAuctionRoom { //inside auction room db tasks will be executed by this
    UpdateRemainingRTMS(ParticipantId),
    PlayerSold(SoldPlayer),
    PlayerUnSold(UnSoldPlayer),
    BalanceUpdate(BalanceUpdate),
    UpdateRoomStatus(RoomStatus),
    CompletedRoomSoldPlayers(CompletedRoom), // it will add and remove the sold players
    CompletedRoomUnsoldPlayers(CompletedRoom), // it will add and remove the unsold players
    CompletedRoomCompletedAt(CompletedRoom)
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
