use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct AuctionCompletedTasksExecutionModel {
    pub room_id: String,
    pub password: String,
}