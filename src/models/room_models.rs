use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Participant {
    participant_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Teams {
    teams: Vec<String>,
}
