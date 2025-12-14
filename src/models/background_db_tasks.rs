
pub struct ParticipantId {
    pub id: i32,
}

pub struct SoldPlayer {
    pub room_id: String,
    pub player_id: i32,
    pub participant_id: i32,
    pub bid_amount: f32
}

pub struct UnSoldPlayer {
    pub player_id: i32,
    pub room_id: String
}

pub struct BalanceUpdate {
    pub participant_id: i32,
    pub remaining_balance: f32
}

pub struct RoomStatus {
    pub room_id: String,
    pub status: String
}

pub enum DBCommandsAuctionRoom { //inside auction room db tasks will be executed by this
    UpdateRemainingRTMS(ParticipantId),
    PlayerSold(SoldPlayer),
    PlayerUnSold(UnSoldPlayer),
    BalanceUpdate(BalanceUpdate),
    UpdateRoomStatus(RoomStatus),
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

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct IpInfoResponse {
    pub city: Option<String>,
    pub region: Option<String>,
    pub postal: Option<String>,
    pub country: Option<String>,
}
