use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Participant {
    pub participant_id: i32,
    pub team_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParticipantResponse {
    pub participant_id: i32,
    pub team_name: String,
    pub user_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParticipantsWithTeam {
    pub participant: Participant,
    pub remaining_participants: Vec<Participant>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Teams {
    teams: Vec<String>,
}

use chrono::{DateTime, Utc};
#[derive(Debug, Serialize, Deserialize)]
pub struct Rooms {
    pub(crate) room_id: String,
    pub(crate) created_at: DateTime<Utc>,
}