use redis_derive::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};

#[derive(Debug,Clone)]
pub enum Teams{
    MumbaiIndians,
    ChennaiSuperKings,
    SunRisersHyderabad,
    PunjabKings,
    RajasthanRoyals,
    RoyalChallengersBangalore,
    KolkataKnightRiders,
    DelhiCapitals,
    LucknowSuperGaints,
    GujaratTitans,
    Unknown,
} // after getting the team name, we are going to check whether the team name in the same valid format or not

impl Teams {
    pub fn check_team(team: &str) -> bool {
        matches!(
            team,
            "Mumbai Indians"
                | "Chennai Super Kings"
                | "Sun Risers Hyderabad"
                | "Punjab Kings"
                | "Rajasthan Royals"
                | "Royal Challengers Bangalore"
                | "Kolkata Knight Riders"
                | "Delhi Capitals"
                | "Lucknow Super Gaints"
                | "Gujarat Titans"
        )
    }
}


pub enum RoomStatus {
    NotStarted,
    InProgress,
    Completed,
}

impl RoomStatus {
    pub fn check_room_status(room_status: String) -> RoomStatus {
        match room_status.as_str() {
            "not_started" => RoomStatus::NotStarted,
            "in_progress" => RoomStatus::InProgress,
            "completed" => RoomStatus::Completed,
            _ => RoomStatus::NotStarted,
        }
    }
}

pub struct PlayerBrought {
    pub player_id : i32,
    pub player_name : String,
    pub role : String,
    pub amount: u8
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TeamDetails {
    pub remaining_balance: f32,
    pub total_players: i32,
    pub total_batsmans: i32,
    pub total_bowlers: i32,
    pub all_rounders: i32
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerDetails {
    pub player_id: i32,
    pub player_name: String,
    pub role: String,
    pub brought_price: f32
}

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct SoldPlayerOutput {
    pub player_id: i32,
    pub player_name: String,
    pub team_name: String,
    pub bought_price: f32,
    pub role: String,
}

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct UnSoldPlayerOutput {
    pub player_id: i32,
    pub player_name: String,
    pub role: String,
    pub base_price: f32,
}
