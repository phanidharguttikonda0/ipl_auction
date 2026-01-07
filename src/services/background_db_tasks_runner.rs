use std::sync::Arc;
use std::time::Duration;
use chrono::Utc;
use redis::AsyncCommands;
use crate::models::app_state::AppState;
use crate::models::background_db_tasks::{DBCommandsAuctionRoom, DBCommandsAuction, IpInfoResponse, SoldPlayer, CompletedRoom, UnSoldPlayer, RoomStatus, ParticipantId, BalanceUpdate, AuctionRoomRetryTasks};

pub async fn background_tasks_executor(app_state: Arc<AppState>, mut rx: tokio::sync::mpsc::UnboundedReceiver<DBCommandsAuctionRoom>) {
    tracing::info!("Background tasks executor for postgres sql started");
    while let Some(command) = rx.recv().await {
        match command {
            DBCommandsAuctionRoom::UpdateRemainingRTMS(mut participant) => {
                tracing::info!("Updating remaining rtms for {}", participant.id);
                let state = app_state.clone() ;
                tokio::task::spawn(async move {
                    let app_state = state ;
                    match app_state.database_connection.update_remaining_rtms(participant.id).await {
                        Ok(_res) => {
                            tracing::info!("Updated remaining rtms for {}", participant.id);
                        } ,
                        Err(err) =>{
                            tracing::error!("Failed to update remaining rtms for {} : {}", participant.id, err);
                            participant.retry_count += 1 ;
                            participant.last_error =  err.to_string() ;
                            app_state.redis_connection.add_retry_task(&DBCommandsAuctionRoom::UpdateRemainingRTMS(participant), &app_state).await.unwrap();
                        } // we need to check a way for the failed tasks, retry logics
                    } ;
                }) ;

            },
            DBCommandsAuctionRoom::BalanceUpdate(mut balance_update) => {
                tracing::info!("balance update was being called") ;
                let state = app_state.clone() ;
                tokio::task::spawn(async move {
                    let app_state = state ;
                    match app_state.database_connection.update_balance(balance_update.participant_id, balance_update.remaining_balance).await {
                        Ok(_res) => {
                            tracing::info!("balance successfully updated") ;
                        },
                        Err(err) => {
                            tracing::error!("error for balance update was {}", err) ;
                            balance_update.retry_count += 1 ;
                            balance_update.last_error = err.to_string() ;
                            app_state.redis_connection.add_retry_task(&DBCommandsAuctionRoom::BalanceUpdate(balance_update), &app_state).await.unwrap();
                        }
                    }
                }) ;

            },
            DBCommandsAuctionRoom::PlayerSold(mut player_sold) => {
                tracing::info!("player_sold was being being executed as a background task") ;
                let state = app_state.clone() ;
                tokio::task::spawn(async move {
                    let app_state = state ;
                    match app_state.database_connection.add_sold_player(&player_sold.room_id,player_sold.player_id,player_sold.participant_id, player_sold.bid_amount).await {
                        Ok(_res) => {
                            tracing::info!("successfully executed the add_sold_player") ;
                        },
                        Err(err) => {
                            tracing::error!("error for player unsold was {}", err) ;
                            player_sold.retry_count += 1 ;
                            player_sold.last_error = err.to_string() ;
                            app_state.redis_connection.add_retry_task(&DBCommandsAuctionRoom::PlayerSold(player_sold), &app_state).await.unwrap();
                        }
                    }
                }) ;

            },
            DBCommandsAuctionRoom::PlayerUnSold(mut player_un_sold) => {
                tracing::info!("player_un_sold was being executed as a background task") ;
                let state = app_state.clone() ;
                tokio::task::spawn(async move {
                    let app_state = state ;
                    match app_state.database_connection.add_unsold_player(&player_un_sold.room_id,player_un_sold.player_id).await {
                        Ok(_res) => {
                            tracing::info!("successfully executed player unsold") ;
                        },
                        Err(err) => {
                            tracing::error!("error for player unsold was {}", err) ;
                            player_un_sold.retry_count += 1 ;
                            player_un_sold.last_error = err.to_string() ;
                            app_state.redis_connection.add_retry_task(&DBCommandsAuctionRoom::PlayerUnSold(player_un_sold), &app_state).await.unwrap();
                        }
                    }
                }) ;
            },
            DBCommandsAuctionRoom::UpdateRoomStatus(mut room_status) => {
                tracing::info!("Room status update was being executed as background task") ;
                let state = app_state.clone() ;
                tokio::spawn(async move {
                    let app_state = state ;
                    match app_state.database_connection.update_room_status(&room_status.room_id, &room_status.status).await {
                        Ok(_res) => {
                            tracing::info!("successfully executed update room-status") ;
                        },
                        Err(err) => {
                            tracing::error!("error for room-status updating {}", err) ;
                            room_status.retry_count += 1 ;
                            room_status.last_error = err.to_string() ;
                            app_state.redis_connection.add_retry_task(&DBCommandsAuctionRoom::UpdateRoomStatus(
                                room_status
                            ), &app_state).await.unwrap();
                        }
                    }
                }) ;
            },
            DBCommandsAuctionRoom::CompletedRoomCompletedAt(mut completed_room) => {
                tracing::info!("updating completed_at") ;
                let state = app_state.clone() ;
                tokio::spawn(async move {
                    let app_state = state.clone() ;
                    match app_state.database_connection.set_completed_at(&completed_room.room_id).await {
                        Ok(_) => {
                            tracing::info!("successfully updated completed_at for room_id {}", completed_room.room_id) ;
                        },
                        Err(err) => {
                            tracing::error!("error while updating the completedAt for room_id {} and error was {}", completed_room.room_id, err.to_string()) ;
                            completed_room.retry_count += 1 ;
                            completed_room.last_error = err.to_string() ;
                            app_state.redis_connection.add_retry_task(&DBCommandsAuctionRoom::CompletedRoomCompletedAt(
                                completed_room
                            ), &app_state).await.unwrap();
                        }
                    }
                }) ;
            },
            DBCommandsAuctionRoom::CompletedRoomUnsoldPlayers(mut completed_room) => {
                tracing::info!("completed unsold players") ;
                let state = app_state.clone() ;
                tokio::spawn(async move {
                    let app_state = state.clone() ;
                    // we need to make sure an atomicity to be takes place
                    let mut tx: Transaction<Postgres> = app_state.database_connection.connection.begin().await.expect("") ;
                    // firstly move the players to the completed rooms unsold players table
                    match DatabaseAccess::add_to_completed_room_unsold_players(&mut tx, &completed_room.room_id).await {
                        Ok(_) => {
                            tracing::info!("sucessfully add unsold players to completed_rooms table for room_id {}", completed_room.room_id) ;

                            match DatabaseAccess::remove_unsold_players(&mut tx, &completed_room.room_id).await {
                                Ok(_) => {
                                    tx.commit().await.expect("unable to commit transaction") ;
                                    tracing::info!("successfully removed unsold players from unsold_players room for room_id {}", completed_room.room_id) ;
                                },
                                Err(err) => {
                                    tx.rollback().await.expect("unable to rollback") ;
                                    completed_room.retry_count += 1 ;
                                    completed_room.last_error = err.to_string() ;
                                    app_state.redis_connection.add_retry_task(&DBCommandsAuctionRoom::CompletedRoomUnsoldPlayers(
                                        completed_room
                                    ), &app_state).await.unwrap();
                                }
                            }
                        },
                        Err(err) => {
                            tx.rollback().await.expect("unable to rollback") ;
                            completed_room.retry_count += 1 ;
                            completed_room.last_error = err.to_string() ;
                            app_state.redis_connection.add_retry_task(&DBCommandsAuctionRoom::CompletedRoomUnsoldPlayers(
                                completed_room
                            ), &app_state).await.unwrap();
                        }
                    }
                }) ;

            },
            DBCommandsAuctionRoom::CompletedRoomSoldPlayers(mut completed_room) => {
                tracing::info!("completed sold players") ;
                // we need to make sure an atomicity to be takes place
                let state = app_state.clone() ;
                tokio::spawn(async move {
                    let app_state = state ;
                    let mut tx: Transaction<Postgres> = app_state.database_connection.connection.begin().await.expect("") ;
                    // firstly move the players to the completed rooms unsold players table
                    match DatabaseAccess::add_to_completed_room_sold_players(&mut tx, &completed_room.room_id).await {
                        Ok(_) => {
                            tracing::info!("successfully add sold players to completed_rooms table for room_id {}", completed_room.room_id) ;

                            match DatabaseAccess::remove_sold_players(&mut tx, &completed_room.room_id).await {
                                Ok(_) => {
                                    tx.commit().await.expect("unable to commit transaction") ;
                                    tracing::info!("successfully removed sold players from sold_players room for room_id {}", completed_room.room_id) ;
                                },
                                Err(err) => {
                                    tx.rollback().await.expect("unable to rollback") ;
                                    completed_room.retry_count += 1 ;
                                    completed_room.last_error = err.to_string() ;
                                    app_state.redis_connection.add_retry_task(&DBCommandsAuctionRoom::CompletedRoomSoldPlayers(
                                        completed_room
                                    ), &app_state).await.unwrap();
                                }
                            }
                        },
                        Err(err) => {
                            tx.rollback().await.expect("unable to rollback") ;
                            completed_room.retry_count += 1 ;
                            completed_room.last_error = err.to_string() ;
                            app_state.redis_connection.add_retry_task(&DBCommandsAuctionRoom::CompletedRoomSoldPlayers(
                                completed_room
                            ), &app_state).await.unwrap();
                        }
                    }
                }) ;
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
use sqlx::{Postgres, Transaction};
use crate::services::auction::DatabaseAccess;

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


                match serde_json::from_str(&task_json).unwrap() {
                    DBCommandsAuctionRoom::UpdateRemainingRTMS(participant) => {
                        tracing::info!("Updating remaining rtms all retries were Exhausted");
                        // need to figure out what envelope was it , list at least the task name
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::UpdateRemainingRTMS(participant)).unwrap() ;
                    },
                    DBCommandsAuctionRoom::BalanceUpdate(balance_update) => {
                        tracing::info!("balance update all retries were Exhausted") ;
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::BalanceUpdate(balance_update)).expect("") ;
                    },
                    DBCommandsAuctionRoom::PlayerSold(player_sold) => {
                        tracing::info!("player_sold all retries were Exhausted") ;
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::PlayerSold(player_sold)).expect("") ;
                    },
                    DBCommandsAuctionRoom::PlayerUnSold(player_un_sold) => {
                        tracing::info!("player_un_sold all retries were Exhausted") ;
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::PlayerUnSold(player_un_sold)).expect("") ;
                    },
                    DBCommandsAuctionRoom::UpdateRoomStatus(room_status) => {
                        tracing::info!("Room status update all retries were Exhausted") ;
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::UpdateRoomStatus(room_status)).expect("") ;
                    },
                    DBCommandsAuctionRoom::CompletedRoomSoldPlayers(completed_room) => {
                        tracing::info!("this never exists, the tasks inside this will execute independently") ;
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::CompletedRoomSoldPlayers(completed_room) ).expect("") ;
                    },
                    DBCommandsAuctionRoom::CompletedRoomCompletedAt(completed_room) => {
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::CompletedRoomCompletedAt(completed_room)).expect("") ;
                    },
                    DBCommandsAuctionRoom::CompletedRoomUnsoldPlayers(completed_room) => {
                        app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::CompletedRoomUnsoldPlayers(completed_room)).expect("") ;

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
                app_state.database_connection.add_to_dlq::<ParticipantId>("UpdatingRTMs", participant.clone(),participant.retry_count as i16, &participant.last_error ).await.expect("") ;
            },
            DBCommandsAuctionRoom::BalanceUpdate(balance_update) => {
                tracing::info!("balance update all retries were Exhausted") ;
                app_state.database_connection.add_to_dlq::<BalanceUpdate>("UpdatingBalance", balance_update.clone(), balance_update.retry_count as i16, &balance_update.last_error).await.expect("") ;
            },
            DBCommandsAuctionRoom::PlayerSold(player_sold) => {
                tracing::info!("player_sold all retries were Exhausted") ;
                app_state.database_connection.add_to_dlq::<SoldPlayer>("SellingPlayer", player_sold.clone(), player_sold.retry_count as i16, &player_sold.last_error).await.expect("") ;
            },
            DBCommandsAuctionRoom::PlayerUnSold(player_un_sold) => {
                tracing::info!("player_un_sold all retries were Exhausted") ;
                app_state.database_connection.add_to_dlq::<UnSoldPlayer>("UnsoldPlayer", player_un_sold.clone(), player_un_sold.retry_count as i16, &player_un_sold.last_error).await.expect("") ;
            },
            DBCommandsAuctionRoom::UpdateRoomStatus(room_status) => {
                tracing::info!("Room status update all retries were Exhausted") ;
                app_state.database_connection.add_to_dlq::<RoomStatus>("UpdatingRoomStatus", room_status.clone(), room_status.retry_count as i16, &room_status.last_error).await.expect("") ;
            },
            DBCommandsAuctionRoom::CompletedRoomSoldPlayers(completed_room) => {
                tracing::info!("this never exists, the tasks inside this will execute independently") ;
                app_state.database_connection.add_to_dlq::<CompletedRoom>("CompletedRoomSoldPlayers", completed_room.clone(), completed_room.retry_count as i16, &completed_room.last_error).await.expect("") ;
            },
            DBCommandsAuctionRoom::CompletedRoomCompletedAt(completed_room) => {
                app_state.database_connection.add_to_dlq::<CompletedRoom>("CompletedRoomCompletedAt", completed_room.clone(), completed_room.retry_count as i16, &completed_room.last_error).await.expect("") ;
            },
            DBCommandsAuctionRoom::CompletedRoomUnsoldPlayers(completed_room) => {
                app_state.database_connection.add_to_dlq::<CompletedRoom>("CompletedRoomUnsoldPlayers", completed_room.clone(), completed_room.retry_count as i16, &completed_room.last_error).await.expect("") ;
            }
        };
    }
}