use sqlx::{query_scalar, Pool, Postgres};
use sqlx::postgres::PgPoolOptions;

#[derive(Debug, Clone)]
pub struct DatabaseAccess {
    connection: Pool<Postgres>
}

impl DatabaseAccess {
    pub async fn new() -> Self {
        let postgress_url = std::env::var("POSTGRESS_URL").unwrap();
        let max_connections = std::env::var("MAX_CONNECTIONS").unwrap().parse::<u32>().unwrap();
        let pool = PgPoolOptions::new().max_connections(max_connections).connect(&postgress_url).await.unwrap() ;
        Self {
            connection: pool,
        }
    }

    pub async fn get_team_name(&self, participant_id: i32) -> Result<String, sqlx::Error> {
        let team_name = query_scalar::<_, String>("SELECT team_selected FROM participants WHERE id = $1")
            .bind(participant_id)
            .fetch_one(&self)
            .await;
        
        match team_name { 
            Ok(team_name) => {
                Ok(team_name)
            },
            Err(err) => {
                tracing::warn!("error occured while getting team name based on the participant_id {}", err) ;
                Err(err)
            }
        }

    }
}