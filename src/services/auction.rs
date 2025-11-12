use sqlx::{query_scalar, Pool, Postgres, Row};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;
use crate::models::app_state::Player;
use dotenv::dotenv;
use redis::AsyncCommands;
use crate::models::room_models::Participant;

#[derive(Debug, Clone)]
pub struct DatabaseAccess {
    pub connection: Pool<Postgres>
}

impl DatabaseAccess {
    pub async fn new() -> Self {
        let postgress_url = std::env::var("DATABASE_URL");
        let postgress_url =  match postgress_url {
           Ok(url) => url,
            Err(err) => {
                tracing::error!("error occured while getting the database url {}", err) ;
                panic!("error occured while getting the database url {}", err) ;
            }
        } ;
        let max_connections = std::env::var("MAX_CONNECTIONS").unwrap().parse::<u32>().unwrap();
        let pool = PgPoolOptions::new().max_connections(max_connections).connect(&postgress_url).await.unwrap() ;
        Self {
            connection: pool,
        }
    }

    pub async fn get_team_name(&self, participant_id: i32) -> Result<String, sqlx::Error> {
        let team_name = query_scalar::<_, String>("SELECT team_selected FROM participants WHERE id = $1")
            .bind(participant_id)
            .fetch_one(&self.connection)
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
        let players = sqlx::query_as::<_, Player>("SELECT * FROM players").fetch_all(&self.connection).await;
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


    pub async fn is_room_creator(
        &self,
        participant_id: i32,
        room_id: String,
    ) -> Result<bool, sqlx::Error> {
        let row = sqlx::query(
            r#"
        SELECT CASE
                   WHEN r.creator_id = p.user_id THEN TRUE
                   ELSE FALSE
               END AS is_creator
        FROM participants p
        JOIN rooms r ON p.room_id = r.id
        WHERE p.id = $1 AND p.room_id = $2
        "#
        )
            .bind(participant_id)
            .bind(sqlx::types::Uuid::parse_str(&room_id).expect("unable to parse the UUID"))
            .fetch_one(&self.connection)
            .await?;

        Ok(row.try_get::<bool, _>("is_creator")?)
    }

    pub async fn create_room(&self, user_id: i32) -> Result<String, sqlx::Error> {
        let room = sqlx::query("insert into rooms (creator_id) values ($1) returning id")
            .bind(user_id)
            .fetch_one(&self.connection).await ;

        match room {
            Ok(room) =>{
                tracing::info!("created room, getting room_id") ;
                let room_id: sqlx::types::Uuid = room.get("id") ;
                let room_id = room_id.to_string() ;
                tracing::info!("the room id that we got was {}", room_id) ;
                Ok(room_id)
            },
            Err(err) => {
                tracing::error!("got error while creating room {}", err) ;
                Err(err)
            }
        }
    }

    pub async fn get_remaining_teams(&self, room_id: String) -> Result<Vec<String>, sqlx::Error> {
        let teams = sqlx::query("select team_name from participants where room_id = $1")
            .bind(sqlx::types::Uuid::parse_str(&room_id).expect("unable to parse the UUID"))
            .fetch_all(&self.connection).await ;
        match teams {
            Ok(teams_selected) => {
                let mut teams: Vec<String> = vec![] ;
                for team in teams_selected {
                    teams.push(team.get("id"))
                }
                Ok(teams)
            },
            Err(err) => {
                tracing::error!("unable to get the remaining participants in a room {}",err) ;
                Err(err)
            }
        }
    }

    pub async fn is_already_participant(&self, user_id: i32, room_id: String) -> Result<(i32, String), String> {
        let participant = sqlx::query("SELECT id, team_selected FROM participants WHERE user_id = $1 AND room_id = $2")
            .bind(user_id)
            .bind(sqlx::types::Uuid::parse_str(&room_id).expect("unable to parse UUID"))
            .fetch_optional(&self.connection)
            .await;

        match participant {
            Ok(participant) => {
                match participant {
                    Some(row) => {
                        tracing::info!("Found participant!");
                        let participant_id: i32 = row.get("id");
                        let team_selected: String = row.get("team_selected");
                        tracing::info!("id: {}, team: {}", participant_id, team_selected);
                        Ok((participant_id, team_selected))
                    }
                    None => {
                        tracing::warn!("No participant found for this user in this room.");
                        Err(String::from("no participant found"))
                    }
                }
            },
            Err(err) => {
                tracing::error!("error while getting participant {}", err) ;
                Err(String::from("Error"))
            }
        }

    }
    pub async fn get_room_status(&self, room_id: String) -> Result<String, sqlx::Error> {
        let room_status = sqlx::query("select status::TEXT from rooms where id = $1")
            .bind(sqlx::types::Uuid::parse_str(&room_id).expect("unable to parse the UUID"))
            .fetch_one(&self.connection).await ;
        match room_status {
            Ok(room_status) =>{
                let room_status: String = room_status.get("status") ;
                tracing::info!("the room status for room_id {}  was {}", room_id, room_status) ;
                Ok(room_status)
            },
            Err(err) => {
                tracing::error!("getting error while getting room_status for room-id {}", room_id) ;
                Err(err)
            }
        }
    }

    pub async fn add_participant(&self, user_id: i32, room_id: String, team_name: String) -> Result<i32, sqlx::Error> {
        let participant = sqlx::query("insert into participants  (user_id, room_id, team_selected) values ($1,$2,$3) returning id")
            .bind(user_id)
            .bind(sqlx::types::Uuid::parse_str(&room_id).expect("unable to parse the UUID"))
            .bind(&team_name)
            .fetch_one(&self.connection).await ;

        match participant {
            Ok(participant) => {
                tracing::info!("successfully added the participant to the room {}", room_id) ;
                let participant_id: i32 = participant.get("id") ;
                tracing::info!("the participant_id was {}", participant_id) ;
                Ok(participant_id)
            },
            Err(err) =>{
                tracing::error!("got error while inserting a new participant to the room {} error was {}", room_id, err) ;
                Err(err)
            }
        }
    }


    pub async fn add_sold_player(&self, room_id: String, player_id: i32, participant_id: i32, amount: f32) -> Result<(), sqlx::Error> {
        let result = sqlx::query("insert into sold_players (room_id, player_id, participant_id, amount) values ($1,$2,$3,$4)")
            .bind(&room_id)
            .bind(player_id)
            .bind(participant_id)
            .bind(amount).execute(&self.connection).await ;

        match result {
            Ok(_) => {
                tracing::info!("added sold player successfully") ;
                Ok(())
            },
            Err(err) => {
                tracing::error!("got an error while adding a sold player") ;
                tracing::error!("error {}", err) ;
                Err(err)
            }
        }
    }
    pub async fn add_unsold_player(&self, room_id: String, player_id: i32) -> Result<(), sqlx::Error> {
        let result = sqlx::query("insert into unsold_players (room_id, player_id) values ($1,$2)")
            .bind(&room_id)
            .bind(player_id)
            .execute(&self.connection).await ;
        match result {
            Ok(_) => {
                tracing::info!("added unsold player successfully") ;
                Ok(())
            },
            Err(err) => {
                tracing::error!("got an error while adding a unsold player") ;
                tracing::error!("error {}", err) ;
                Err(err)
            }
        }
    }
}