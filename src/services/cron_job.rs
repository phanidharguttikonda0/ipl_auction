use std::sync::Arc;
use sqlx::Row;
use tokio::time::Duration;
use crate::models::app_state::AppState;
use crate::models::background_db_tasks::{CompletedRoom, DBCommandsAuctionRoom};

pub fn cron_job_making_room_status_to_completed_every_48_hours(app_state: Arc<AppState>) {

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_hours(48));
        tracing::info!("just created an interval, for 48 hours") ;
        loop {
            interval.tick().await ;
            let result = sqlx::query(
                r#"
                UPDATE rooms
                SET status = 'completed'::room_status
                WHERE created_at < NOW() - INTERVAL '48 hours'
                  AND (status = 'not_started'::room_status
                       OR status = 'in_progress'::room_status)
                RETURNING id::Text
                "#
            )
                .fetch_all(&app_state.database_connection.connection)
                .await;


            match result {
                Ok(rows) => {
                    let affected = rows.len();

                    tracing::info!("*x*x cron execution completed");

                    if affected > 0 {
                        let room_ids: Vec<String> = rows.iter().map(|r| r.get("id")).collect();
                        tracing::info!("the rooms_ids were {:#?}", room_ids) ;
                        for room_id in room_ids.iter() {
                            app_state.redis_connection.auction_clean_up(&room_id).await.unwrap() ;
                            let completed_room = CompletedRoom {
                                room_id: room_id.clone(),
                                retry_count: 0,
                                last_error: "".to_string()
                            } ;
                            app_state.auction_room_database_task_executor.send(
                                DBCommandsAuctionRoom::CompletedRoomSoldPlayers(completed_room.clone())
                            ).unwrap() ;
                            app_state.auction_room_database_task_executor.send(
                                DBCommandsAuctionRoom::CompletedRoomUnsoldPlayers(completed_room.clone())
                            ).unwrap() ;
                            app_state.auction_room_database_task_executor.send(
                                DBCommandsAuctionRoom::CompletedRoomCompletedAt(completed_room)
                            ).unwrap() ;
                        }
                        tracing::info!("*x*x updated {} rooms", affected);
                        tracing::info!("*x*x room_ids: {:?}", room_ids);
                    } else {
                        tracing::info!("*x*x no rooms needed updating");
                    }
                }
                Err(err) => {
                    tracing::error!("*+*+ error occurred while updating rooms to completed status");
                    tracing::error!("*+*+ error was {}", err);
                }
            }

        }

    }) ;

}