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
    pub fn check_team(team: String) -> Teams {
        match team.as_str() { 
            "Mumbai Indians" => Teams::MumbaiIndians,
            "Chennai Super Kings" => Teams::ChennaiSuperKings,
            "Sun Risers Hyderabad" => Teams::SunRisersHyderabad,
            "Punjab Kings" => Teams::PunjabKings,
            "Rajasthan Royals" => Teams::RajasthanRoyals,
            "Royal Challengers Bangalore" => Teams::RoyalChallengersBangalore,
            "Kolkata Knight Riders" => Teams::KolkataKnightRiders,
            "Delhi Capitals" => Teams::DelhiCapitals,
            "Lucknow Super Gaints" => Teams::LucknowSuperGaints,
            "Gujarat Titans" => Teams::GujaratTitans,
            _ => Teams::Unknown,
        }
    } // if the team name was not in this format, we are going to return the team name was incorrect
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