use sqlx::{query_scalar, Pool, Postgres, Row};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;
use crate::models::app_state::Player;
use dotenv::dotenv;
use redis::AsyncCommands;
use crate::models::auction_models::SoldPlayer;
use crate::models::player_models::{PlayerDetails, SoldPlayerOutput, TeamDetails, UnSoldPlayerOutput};
use crate::models::room_models::{Participant, ParticipantResponse, Rooms};

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



    pub async fn create_room(&self, user_id: i32, is_strict_mode: bool) -> Result<String, sqlx::Error> {
        let room = sqlx::query("insert into rooms (creator_id, strict_mode) values ($1, $2) returning id")
            .bind(user_id)
            .bind(is_strict_mode)
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
        let teams = sqlx::query("select team_selected from participants where room_id = $1")
            .bind(sqlx::types::Uuid::parse_str(&room_id).expect("unable to parse the UUID"))
            .fetch_all(&self.connection).await ;
        match teams {
            Ok(teams_selected) => {
                let mut teams = vec![
                    "Mumbai Indians".to_string(),
                    "Chennai Super Kings".to_string(),
                    "Sun Risers Hyderabad".to_string(),
                    "Punjab Kings".to_string(),
                    "Rajasthan Royals".to_string(),
                    "Royal Challengers Bangalore".to_string(),
                    "Kolkata Knight Riders".to_string(),
                    "Delhi Capitals".to_string(),
                    "Lucknow Super Gaints".to_string(),
                    "Gujarat Titans".to_string(),
                ];

                for team in teams_selected {
                    let mut i = 0 ;
                    while i < teams.len() {
                        let team_name: String = team.get("team_selected") ;
                        if team_name == teams[i] {
                            teams.remove(i) ;
                            break;
                        }
                        i += 1 ;
                    }
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

    pub async fn get_room_mode(&self, room_id: &str) -> Result<bool, sqlx::Error> {
        let room_mode = sqlx::query("select strict_mode from rooms where id = $1")
            .bind(sqlx::types::Uuid::parse_str(&room_id).expect("unable to parse the UUID"))
            .fetch_one(&self.connection).await ;
        match room_mode {
            Ok(room_mode) =>{
                let strict_mode: bool = room_mode.get("strict_mode") ;
                tracing::info!("the room strict_mode for room_id {}  was {}", room_id, strict_mode) ;
                Ok(strict_mode)
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


    pub async fn add_sold_player(&self, room_id: &str, player_id: i32, participant_id: i32, amount: f32) -> Result<(), sqlx::Error> {
        let result = sqlx::query("insert into sold_players (room_id, player_id, participant_id, amount) values ($1,$2,$3,$4)")
            .bind(sqlx::types::Uuid::parse_str(room_id).expect("unable to parse the UUID"))
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
    pub async fn add_unsold_player(&self, room_id: &str, player_id: i32) -> Result<(), sqlx::Error> {
        let result = sqlx::query("insert into unsold_players (room_id, player_id) values ($1,$2)")
            .bind(sqlx::types::Uuid::parse_str(room_id).expect("unable to parse the UUID"))
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

    pub async fn update_balance(&self, participant_id: i32, remaining_balance: f32) -> Result<(), sqlx::Error> {
        tracing::info!("Executing the update_balance to update balance in psql") ;
        let updated = sqlx::query("update participants set purse_remaining=$1 where id=$2")
            .bind(remaining_balance)
            .bind(participant_id)
            .execute(&self.connection).await ;

        match updated {
            Ok(_) => Ok(()) ,
            Err(err) => {
                tracing::error!("Occurred while updating the balance in the postgres") ;
                Err(err)
            }

        }
    }

    pub async fn update_room_status(&self, room_id: &str, status: &str) -> Result<(), sqlx::Error> {
        tracing::info!("Executing the update_room_status to update status in psql") ;
        let updated = sqlx::query("update rooms set status=$1::room_status  where id=$2")
            .bind(status)
            .bind(sqlx::types::Uuid::parse_str(room_id).expect("unable to parse the UUID"))
        .execute(&self.connection).await ;

        match updated {
            Ok(_) => Ok(()) ,
            Err(err) => {
                tracing::error!("Occurred while updating the balance in the postgres") ;
                Err(err)
            }

        }
    }

    pub async fn get_team_details(&self, participant_id: i32) -> Result<(i32,i32,i32,i32), sqlx::Error> {
        let rows = sqlx::query(
            r#"
                    SELECT
                        (SELECT COUNT(*)
                         FROM sold_players
                         WHERE participant_id = $1) AS total_count,
                        p.role,
                        COUNT(*) AS role_count
                    FROM sold_players sp
                    JOIN players p ON sp.player_id = p.id
                    WHERE sp.participant_id = $1
                    GROUP BY p.role
                    "#,
                        )
                            .bind(participant_id)
                            .fetch_all(&self.connection)
                            .await;

        match rows {
            Ok(rows) => {
                let mut bat_count = 0;
                let mut bowl_count = 0;
                let mut ar_count = 0;

                for row in rows {
                    let role: String = row.get("role");
                    let count: i64 = row.get("role_count");

                    match role.as_str() {
                        "BAT" => bat_count = count as i32,
                        "BOWL" => bowl_count = count as i32,
                        "AR" => ar_count = count as i32,
                        _ => {}
                    }
                }

                let total_count = bat_count + bowl_count + ar_count;
                Ok((total_count, bat_count, bowl_count, ar_count))

            },
            Err(err) => {
                tracing::error!("error occurred while fetching team details") ;
                Err(err)
            }
        }
    }

    pub async fn get_team_players(&self, participant_id: i32) -> Result<Vec<PlayerDetails>, sqlx::Error>
    {
        let rows = sqlx::query(
            r#"
        SELECT
            sp.player_id,
            p.name,
            p.role,
            sp.amount
        FROM sold_players sp
        JOIN players p ON sp.player_id = p.id
        WHERE sp.participant_id = $1
        "#
        )
            .bind(participant_id)
            .fetch_all(&self.connection)
            .await;

        match rows {
            Ok(rows) => {
                tracing::info!("got the player details");

                let players = rows.into_iter().map(|row| {
                    PlayerDetails {
                        player_id: row.get::<i32, _>("player_id"),
                        player_name: row.get::<String, _>("name"),
                        role: row.get::<String, _>("role"),
                        brought_price: row.get::<f32, _>("amount"),
                    }
                }).collect::<Vec<PlayerDetails>>();

                Ok(players)
            }

            Err(err) => {
                tracing::error!("error occurred while getting team players: {}", err);
                Err(err)
            }
        }
    }


    pub async fn get_remaining_balance(&self, participant_id: i32) -> Result<f32, sqlx::Error> {
        let balance = sqlx::query("select purse_remaining from participants where id=$1")
            .bind(participant_id)
            .fetch_one(&self.connection).await ;

        match balance {
            Ok(balance) => {
                tracing::info!("got the balance") ;
                let value = balance.get("purse_remaining") ;
                tracing::info!("{} -> ", value) ;
                Ok(value)
            },
            Err(err) => {
                tracing::error!("got error while getting remaining balance") ;
                tracing::error!("{}", err) ;
                Err(err)
            }
        }

    }

    pub async fn get_rooms(&self, user_id: i32, timestamp: &str, per_page: i32, room_id: &str) -> Result<Vec<Rooms>, sqlx::Error> {

        let timestamp: Option<&str> = if timestamp == "0" {
            tracing::info!("timestamp was 0") ;
            None
        }else {
            tracing::info!("timestamp was {}", timestamp) ;
            Some(timestamp)
        } ;
        let rooms = sqlx::query(
                    "
            SELECT
            r.id::TEXT AS room_id,
            r.created_at,
            r.status::TEXT
        FROM rooms r
        JOIN participants p
            ON r.id = p.room_id
        WHERE p.user_id = $1
          AND (
                $2::timestamptz IS NULL
                OR (r.created_at, r.id) < ($2::timestamptz, $3::uuid)
              )
        ORDER BY r.created_at DESC, r.id DESC
        LIMIT $4;

            "
                )
            .bind(user_id)
            .bind(timestamp)
            .bind(room_id)
            .bind(per_page)
            .fetch_all(&self.connection)
            .await;


        match rooms {
            Ok(rooms) => {
                tracing::info!("got the rooms from the user {}", user_id) ;
                let mut rooms_ = vec![] ;
                for room in rooms.iter() {
                    let room_id = room.get("room_id") ;
                    let created_at = room.get("created_at") ;
                    let room_status = room.get("status") ;
                    rooms_.push(Rooms {
                        room_id, created_at, status: room_status
                    }) ;

                }
                Ok(rooms_)
            },
            Err(err) => {
                tracing::error!("error occurred while getting rooms") ;
                tracing::error!("{}",err) ;
                Err(err)
            }
        }
    }

    pub async fn get_participants_in_room(&self, room_id: String) -> Result<Vec<ParticipantResponse>, sqlx::Error> {
        let participants = sqlx::query("select id,team_selected,user_id from participants where room_id=$1")
            .bind(sqlx::types::Uuid::parse_str(&room_id).expect("unable to parse the UUID"))
            .fetch_all(&self.connection).await ;

        match participants {
            Ok(participants) => {
                tracing::info!("got the participants from the room") ;

                let mut participants_ = vec![] ;
                for participant in participants.iter() {
                    let participant_id: i32 = participant.get("id") ;
                    let team_selected = participant.get("team_selected") ;
                    let user_id = participant.get("user_id") ;
                    participants_.push(ParticipantResponse{
                        participant_id, team_name: team_selected, user_id
                    }) ;
                }
                Ok(participants_)
            }, Err(err) => {
                tracing::error!("got an error while getting participants in room") ;
                tracing::error!("{}", err) ;
                Err(err)
            }
        }

    }

    pub async fn is_room_creator(
        &self,
        participant_id: i32,
        room_id: String,
    ) -> Result<bool, sqlx::Error> {
        tracing::info!("is the room-creator") ;
        let is_creator: Option<bool> = sqlx::query_scalar(
            r#"
        SELECT
            CASE
                WHEN r.creator_id = p.user_id THEN TRUE
                ELSE FALSE
            END AS is_creator
        FROM participants p
        JOIN rooms r
            ON r.id = p.room_id
        WHERE p.id = $1
          AND p.room_id = $2;
        "#
        )
            .bind(participant_id)
            .bind(sqlx::types::Uuid::parse_str(&room_id).expect("unable to parse the UUID"))
            .fetch_optional(&self.connection)
            .await?;

        // If no row found, return false
        Ok(is_creator.unwrap_or(false))
    }

    pub async fn get_sold_players(&self, room_id: String, page_no: i32, offset: i32) -> Result<Vec<SoldPlayerOutput>, sqlx::Error> {
        tracing::info!("getting sold players") ;
        let result = sqlx::query_as::<_, SoldPlayerOutput>(
            r#"
        SELECT
            sp.player_id,
            p.name AS player_name,
            pr.team_selected AS team_name,
            sp.amount AS bought_price,
            p.role
        FROM sold_players sp
        JOIN players p ON sp.player_id = p.id
        JOIN participants pr ON sp.participant_id = pr.id
        WHERE sp.room_id = $1
        ORDER BY sp.id DESC  -- so we are going to get the latest records
        LIMIT $2 OFFSET $3;
        "#
        )
            .bind(sqlx::types::Uuid::parse_str(&room_id).expect("unable to parse the UUID"))
            .bind(offset)      // LIMIT
            .bind((page_no - 1) * offset) // OFFSET - basically offset will 10 by default
            .fetch_all(&self.connection)
            .await;

        match result {
            Ok(sold_players ) => {
                tracing::info!("got sold players") ;
                Ok(sold_players)
            },
            Err(err) => {
                tracing::error!("{}", err) ;
                Err(err)
            }
        }
    }

    pub async fn get_unsold_players(&self, room_id: String, page_no: i32, offset: i32) -> Result<Vec<UnSoldPlayerOutput>, sqlx::Error>
    {
        tracing::info!("getting unsold players");

        let result = sqlx::query_as::<_, UnSoldPlayerOutput>(
            r#"
        SELECT
            up.player_id,
            p.name AS player_name,
            p.role,
            p.base_price
        FROM unsold_players up
        JOIN players p ON up.player_id = p.id
        WHERE up.room_id = $1
        ORDER BY up.id DESC
        LIMIT $2 OFFSET $3;
        "#
        )
            .bind(sqlx::types::Uuid::parse_str(&room_id).expect("unable to parse the UUID"))
            .bind(offset)
            .bind((page_no - 1) * offset)
            .fetch_all(&self.connection)
            .await;

        match result {
            Ok(unsold_players) =>{
                tracing::info!("got the unsold players list") ;
                Ok(unsold_players)
            },
            Err(err) => {
                tracing::error!("got an error while getting unsold players") ;
                tracing::error!("{}", err) ;
                Err(err)
            }
        }
    }


    pub async fn update_remaining_rtms(&self, participant_id: i32 ) -> Result<(), sqlx::Error> {
        tracing::info!("updating remaining rtms in psql");
        let result = sqlx::query("update participants set remaining_rtms = remaining_rtms - 1 where id=$1")
            .bind(participant_id)
            .execute(&self.connection).await ;
        match result {
            Ok(result) => {
                tracing::info!("updated remaining rtms count") ;
                Ok(())
            },
            Err(err) => {
                tracing::error!("got error while updating remaining rtms") ;
                tracing::error!("{}", err) ;
                Err(err)
            }
        }
    }

    pub async fn remove_unsold_players(&self, room_id: &str) -> Result<(), sqlx::Error> {
        tracing::info!("removing unsold players as room was closing") ;
        let result = sqlx::query("delete from unsold_players where room_id=$1")
            .bind(sqlx::types::Uuid::parse_str(&room_id).expect("unable to parse the UUID"))
            .execute(&self.connection).await ;

        tracing::info!("query executed") ;
        match result {
            Ok(result) => {
                tracing::info!("deleted the unsold players from the room_id") ;
                Ok(())
            },
            Err(err) => {
                tracing::error!("error occurred while removing unsold players {}",err) ;
                Err(err)
            }
        }
    }


    pub async fn set_completed_at(&self, room_id: &str) -> Result<(), sqlx::Error> {
        tracing::info!("setting the completed at time for the room") ;
        let result = sqlx::query( r#"
            UPDATE rooms
            SET completed_at = NOW()
            WHERE id = $1
            "#,)
            .bind(sqlx::types::Uuid::parse_str(&room_id).expect("unable to parse the UUID"))
            .execute(&self.connection).await ;

        match result {
            Ok(result) =>{
                tracing::info!("updated completed it") ;
                Ok(())
            },
            Err(err) => {
                tracing::error!("got an error, while updating the completed status room") ;
                Err(err)
            }
        }
    }

    
    pub async fn update_favorite_team(&self, user_id: i32, favorite_team: &str) -> Result<(), sqlx::Error> {
        tracing::info!("updating favorite team for user {}", user_id) ;

        let result = sqlx::query("update users set favorite_team=$1 where id=$2")
            .bind(&favorite_team)
            .bind(user_id)
            .execute(&self.connection).await ;

        match result {
            Ok(res) => {
                tracing::info!("Successfully Updated to the new team");
                Ok(())
            },
            Err(err) => {
                tracing::error!("error while updating favorite team in db {}",err);
                Err(err)
            }
        }

    }

    pub async fn add_location(&self, user_id: i32, location: &str) -> Result<(), sqlx::Error> {
        tracing::info!("updating location for user {}", user_id) ;
        let result = sqlx::query("update users set location=$1 where id=$2")
            .bind(location)
            .bind(user_id)
            .execute(&self.connection).await ;

        match result {
            Ok(res) => {
                tracing::info!("successfully updated to the location") ;
                Ok(())
            },
            Err(err) => {
                tracing::error!("Got an error while updating the location") ;
                Err(err)
            }
        }
    }

}