use std::sync::Arc;
use std::time::Duration;
use axum::extract::ws::Message;
use chrono::Utc;
use redis::AsyncCommands;
use crate::models::app_state::AppState;
use crate::models::background_db_tasks::{DBCommandsAuctionRoom, DBCommandsAuction, IpInfoResponse, RetryEnvelope, RetryTask, BalanceUpdate, SoldPlayer, CompletedRoom, UnSoldPlayer, RoomStatus, ParticipantId};

pub async fn background_tasks_executor(app_state: Arc<AppState>, mut rx: tokio::sync::mpsc::UnboundedReceiver<DBCommandsAuctionRoom>) {
    tracing::info!("Background tasks executor for postgres sql started");
    while let Some(command) = rx.recv().await {
        match command {
            DBCommandsAuctionRoom::UpdateRemainingRTMS(participant) => {
                tracing::info!("Updating remaining rtms for {}", participant.id);
                match app_state.database_connection.update_remaining_rtms(participant.id).await {
                   Ok(_res) => {
                       tracing::info!("Updated remaining rtms for {}", participant.id);
                   } ,
                    Err(err) =>{
                        tracing::error!("Failed to update remaining rtms for {} : {}", participant.id, err);
                        app_state.redis_connection.add_retry_task(&RetryEnvelope{
                            task: RetryTask::UpdateRemainingRTMS {
                                participant_id: participant.id,
                                retry_count: participant.retry_count + 1
                            },
                            last_error: err.to_string(),
                            retry_count: participant.retry_count + 1
                        }, &app_state).await.unwrap();
                    } // we need to check a way for the failed tasks, retry logics
                } ;
            },
            DBCommandsAuctionRoom::BalanceUpdate(balance_update) => {
                tracing::info!("balance update was being called") ;
                match app_state.database_connection.update_balance(balance_update.participant_id, balance_update.remaining_balance).await {
                    Ok(_res) => {
                        tracing::info!("balance successfully updated") ;
                    },
                    Err(err) => {
                        tracing::error!("error for balance update was {}", err) ;
                        app_state.redis_connection.add_retry_task(&RetryEnvelope{
                            task: RetryTask::BalanceUpdate {
                                participant_id: balance_update.participant_id,
                                remaining_balance: balance_update.remaining_balance,
                                retry_count: balance_update.retry_count + 1
                            },
                            last_error: err.to_string(),
                            retry_count: balance_update.retry_count + 1
                        }, &app_state).await.unwrap();
                    }
                }
            },
            DBCommandsAuctionRoom::PlayerSold(player_sold) => {
                tracing::info!("player_sold was being being executed as a background task") ;
                match app_state.database_connection.add_sold_player(&player_sold.room_id,player_sold.player_id,player_sold.participant_id, player_sold.bid_amount).await {
                    Ok(_res) => {
                        tracing::info!("successfully executed the add_sold_player") ;
                    },
                    Err(err) => {
                        tracing::error!("error for player unsold was {}", err) ;
                        app_state.redis_connection.add_retry_task(&RetryEnvelope{
                            task: RetryTask::PlayerSold {
                                room_id: player_sold.room_id,
                                player_id: player_sold.player_id,
                                participant_id: player_sold.participant_id,
                                bid_amount: player_sold.bid_amount,
                                retry_count: player_sold.retry_count + 1
                            },
                            last_error: err.to_string(),
                            retry_count: player_sold.retry_count + 1
                        }, &app_state).await.unwrap();
                    }
                }
            },
            DBCommandsAuctionRoom::PlayerUnSold(player_un_sold) => {
                tracing::info!("player_un_sold was being executed as a background task") ;
                match app_state.database_connection.add_unsold_player(&player_un_sold.room_id,player_un_sold.player_id).await {
                    Ok(_res) => {
                        tracing::info!("successfully executed player unsold") ;
                    },
                    Err(err) => {
                        tracing::error!("error for player unsold was {}", err) ;
                        app_state.redis_connection.add_retry_task(&RetryEnvelope{
                            task: RetryTask::PlayerUnSold {
                                room_id: player_un_sold.room_id,
                                player_id: player_un_sold.player_id,
                                retry_count: player_un_sold.retry_count + 1
                            },
                            last_error: err.to_string(),
                            retry_count: player_un_sold.retry_count + 1
                        }, &app_state).await.unwrap();
                    }
                }
            },
            DBCommandsAuctionRoom::UpdateRoomStatus(room_status) => {
                tracing::info!("Room status update was being executed as background task") ;
                match app_state.database_connection.update_room_status(&room_status.room_id, &room_status.status).await {
                    Ok(_res) => {
                        tracing::info!("successfully executed update room-status") ;
                    },
                    Err(err) => {
                        tracing::error!("error for room-status updating {}", err) ;
                        app_state.redis_connection.add_retry_task(&RetryEnvelope{
                            task: RetryTask::UpdateRoomStatus {
                                room_id: room_status.room_id,
                                status: room_status.status,
                                retry_count: room_status.retry_count + 1
                            },
                            last_error: err.to_string(),
                            retry_count: room_status.retry_count + 1
                        }, &app_state).await.unwrap();
                    }
                }
            },
            DBCommandsAuctionRoom::CompletedRoom(room_id) => {
                auction_completed_tasks_executor(&room_id.room_id, &app_state).await ;
            }
        }
    }
}


pub async fn background_task_executor_outside_auction_db_calls(app_state: Arc<AppState>, mut rx: tokio::sync::mpsc::UnboundedReceiver<DBCommandsAuction>) {
    tracing::info!("Background tasks executor for postgres sql started");
    let api_key = std::env::var("IP_INFO_API_KEY").unwrap();
    while let Some(command) = rx.recv().await {
        match command {
            DBCommandsAuction::AddUserExternalDetails(user_details) => {
                tracing::info!("Got user details") ;
                /*
                    over here we are going to get the location based on the ip address using the API.
                */
                tracing::info!("using the ip address to get the geo location") ;
                let location = get_location(&user_details.ip_address, &api_key).await.unwrap();
                tracing::warn!("location was {}", location) ;

                tracing::info!("starting to store the details in the database") ;
                match app_state.database_connection.add_location(user_details.user_id, &location).await { 
                  Ok(_res) => {
                       tracing::info!("successfully stored the location in the database") ;
                   } ,
                    Err(err) =>{
                        tracing::error!("error while storing the location in the database {}", err) ;
                    }  
                };
            },
            DBCommandsAuction::FavoriteTeamUpdated(fav_team) => {
                tracing::info!("favorite team was updated") ;
                /*
                    over here we are going to add the new record to the table favorite_teams_update.
                */
            }
        }
    }
}


use reqwest::Client;
use crate::models::background_db_tasks::RetryTask::{PlayerSold, PlayerUnSold};

pub async fn get_location(ip_address: &str, api_key: &str) -> Result<String, reqwest::Error> {
    println!("Getting Location for IP Address: {}", ip_address);

    let url = format!("https://ipinfo.io/{}?token={}", ip_address, api_key);

    let client = Client::new();
    let response = client.get(&url).send().await?;

    println!("Response from IP Geolocation API: {:?}", response);

    let mut data: IpInfoResponse = response.json().await?;
    println!("Data from IP Geolocation API: {:?}", data);

    // Convert IN → India
    if let Some(country) = &data.country {
        if country == "IN" {
            data.country = Some("India".to_string());
        }
    }

    Ok(format!(
        "{}, {}, {}, {}",
        data.city.unwrap_or_else(|| "Unknown".to_string()),
        data.region.unwrap_or_else(|| "Unknown".to_string()),
        data.postal.unwrap_or_else(|| "Unknown".to_string()),
        data.country.unwrap_or_else(|| "Unknown".to_string())
    ))
}


pub async fn auction_completed_tasks_executor(room_id: &str, app_state: &Arc<AppState>) {
    tracing::info!("completed Auction Room was {}",room_id) ;
    // firstly we are gonna transfer all sold and unsold players list to the completed_rooms_* table
    let result1 = app_state.database_connection.add_to_completed_room_sold_players(room_id).await ;
    let result2 = app_state.database_connection.add_to_completed_room_unsold_players(room_id).await ;
    // now we are gonna remove the players from the sold and unsold players from the sold_players and unsold_players tables
    let result3 = app_state.database_connection.remove_sold_players(room_id).await ;
    let result4 = app_state.database_connection.remove_unsold_players(room_id).await ;
    let result5 = app_state.database_connection.set_completed_at(room_id).await.expect("error while updating completed_at") ;
    /*
        here we are going to divide each task what ever tasks failed we are going to
    */
    tracing::info!("Successfully completed background work for complete auction room with room_id {}", room_id) ;
}


pub async fn listening_to_retries(app_state: Arc<AppState>) {

    tokio::spawn(async move {
        tracing::info!("starting listening to retries") ;
        let mut redis_connection = app_state.redis_connection.connection.clone();
        loop {
            let now = Utc::now().timestamp();

            let due_tasks: Vec<String> = redis_connection
                .zrangebyscore(
                    "auction:retry:zset",
                    "-inf",
                    now
                )
                .await.unwrap();

            for task_json in due_tasks {
               let _: usize =  redis_connection.zrem("auction:retry:zset", &task_json).await.expect("error while retrieving auction") ;

                let envelope: RetryEnvelope = serde_json::from_str(&task_json).unwrap();

                // need to figure out what envelope was it , list at least the task name
                match envelope.task {
                    RetryTask::BalanceUpdate{participant_id, remaining_balance,retry_count } => {
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::BalanceUpdate(
                            BalanceUpdate {
                                participant_id, remaining_balance, retry_count
                            }
                        )).unwrap();
                    },
                    RetryTask::PlayerSold {player_id, bid_amount, participant_id, room_id,retry_count} => {
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::PlayerSold(
                            SoldPlayer {
                                participant_id, player_id, bid_amount,room_id,retry_count
                            }
                        )).unwrap();
                    },
                    RetryTask::CompletedRoom {room_id,retry_count} => {
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::CompletedRoom(
                            CompletedRoom {
                                room_id,
                                retry_count
                            }
                        )).unwrap();
                    },
                    RetryTask::PlayerUnSold {player_id, room_id,retry_count} => {
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::PlayerUnSold(
                            UnSoldPlayer {
                                player_id, room_id,retry_count
                            }
                        )).unwrap();
                    },
                    RetryTask::UpdateRoomStatus {status, room_id,retry_count} => {
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::UpdateRoomStatus(
                            RoomStatus {
                                room_id, status,retry_count
                            }
                        )).unwrap();
                    },
                    RetryTask::UpdateRemainingRTMS {participant_id,retry_count} => {
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::UpdateRemainingRTMS(
                            ParticipantId {
                                id: participant_id,retry_count
                            }
                        )).unwrap();
                    }
                }

            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        tracing::warn!("Stopped listening to the retries") ;
    }) ;

}


pub async fn save_to_DLQ(app_state: Arc<AppState>, mut rx: tokio::sync::mpsc::UnboundedReceiver<DBCommandsAuctionRoom>) {

    while let Some(command) = rx.recv().await {
        match  command {
            DBCommandsAuctionRoom::UpdateRemainingRTMS(participant) => {
                tracing::info!("Updating remaining rtms all retries were Exhausted");
                
            },
            DBCommandsAuctionRoom::BalanceUpdate(balance_update) => {
                tracing::info!("balance update all retries were Exhausted") ;

            },
            DBCommandsAuctionRoom::PlayerSold(player_sold) => {
                tracing::info!("player_sold all retries were Exhausted") ;

            },
            DBCommandsAuctionRoom::PlayerUnSold(player_un_sold) => {
                tracing::info!("player_un_sold all retries were Exhausted") ;

            },
            DBCommandsAuctionRoom::UpdateRoomStatus(room_status) => {
                tracing::info!("Room status update all retries were Exhausted") ;

            },
            DBCommandsAuctionRoom::CompletedRoom(_) => {
                tracing::info!("this never exists, the tasks inside this will execute independently") ;
            },
        };
    }
}