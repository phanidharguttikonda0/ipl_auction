use std::sync::Arc;
use axum::extract::ws::Message;
use crate::models::app_state::AppState;
use crate::models::background_db_tasks::{DBCommandsAuctionRoom, DBCommandsAuction, IpInfoResponse};

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
                    }
                }
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
                let location = get_location(&user_details.ip_address, "5522222222222222").await.unwrap();
                tracing::warn!("location was {}", location) ;

                tracing::info!("starting to store the details in the database") ;
                match app_state.database_connection.add_location(user_details.user_id, &user_details.ip_address).await { 
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
