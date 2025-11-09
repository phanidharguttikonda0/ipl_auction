use sqlx::{query_scalar, Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use crate::models::app_state::Player;

#[derive(Debug, Clone)]
pub struct DatabaseAccess {
    pub(crate) connection: Pool<Postgres>
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
    pub async fn get_players(&self) -> Result<Vec<Player>, sqlx::Error> {
        let players = sqlx::query_as::<_, Player>("SELECT * FROM players").fetch_all(&self).await;
        match players {
            Ok(players) => {
                Ok(players)
            },
            Err(err) => {
                tracing::warn!("error occured while getting players {}", err) ;
                Err(err)
            }
        }
    }


}