
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

pub enum DBCommands {
    UpdateRemainingRTMS(ParticipantId),
    PlayerSold(SoldPlayer),
    PlayerUnSold(UnSoldPlayer),
    BalanceUpdate(BalanceUpdate),
    UpdateRoomStatus(RoomStatus),
}