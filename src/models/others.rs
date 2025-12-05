use serde::Deserialize;

#[derive(Deserialize)]
pub struct FeedBackRequest {
    pub feedback_type: String,
    pub rating_value: Option<i16>,
    pub title: Option<String>,
    pub description: Option<String>,
}