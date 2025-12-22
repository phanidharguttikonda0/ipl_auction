use axum::extract::ws::Message;
use crate::auction::{broadcast_handler, send_himself};
use crate::models;
use crate::models::app_state::AppState;
use crate::models::auction_models::Bid;
use crate::models::background_db_tasks::DBCommandsAuctionRoom;


#[tracing::instrument(
    name = "start_auction",
    skip(app_state),
    fields(
        room_id = %room_id,
        participant_id = participant_id,
        expiry_time = expiry_time,
        room_mode = room_mode
    )
)]
pub async fn start_auction(room_id: String, participant_id: i32, app_state: &AppState, expiry_time: u8, room_mode: bool) {
    let redis_connection = app_state.redis_connection.clone();
    if redis_connection.get_room_meta(&room_id).await.unwrap().unwrap().room_creator_id != participant_id {
        send_himself(Message::text("You will not having permissions"), participant_id, &room_id, &app_state).await;
    } else {
        if app_state.rooms.read().await.get(&room_id).unwrap().len() < 3 {
            send_himself(Message::text("Min of 3 participants should be in the room to start auction"), participant_id, &room_id, &app_state).await;
        } else {
            match redis_connection.set_pause(&room_id, false).await {
                Ok(_) => {
                    tracing::info!("successfully set the status to pause");
                    // send_himself(Message::text("After the Current Bid Auction will be Paused"), participant_id, &room_id, &app_state).await ;
                },
                Err(err) => {
                    tracing::error!("error occurred while setting the pause status");
                    tracing::error!("err was {}", err);
                    send_himself(Message::text("Technical Problem"), participant_id, &room_id, &app_state).await;
                }
            };

            // we are going to return the first player from the auction
            let player = redis_connection.get_current_player(&room_id).await.unwrap();
            let player = match player {
                Some(player) => {
                    tracing::info!("got the current player");
                    player
                },
                None => {
                    tracing::warn!("I guess auction was just starting no current player");
                    // we need to get the 1st player
                    let player = redis_connection.get_player(1, &room_id).await.unwrap();
                    redis_connection.set_current_player(&room_id, player.clone()).await.unwrap();
                    player
                }
            };
            let message;

            if player.id == 1 {
                // // changing room-status
                app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::UpdateRoomStatus(models::background_db_tasks::RoomStatus {
                    room_id: room_id.clone(),
                    status: "in_progress".to_string(),
                })).expect("Error while sending room_status to a unbounded channel");
            }

            message = Message::from(serde_json::to_string(&player).unwrap());
            // here we are going to add the player as Bid to the redis
            let bid = Bid::new(0, player.id, 0.0, player.base_price, false, false); // no one yet bidded
            redis_connection.update_current_bid(&room_id, bid, expiry_time, -1, room_mode).await.expect("unable to update the bid");


            // broadcasting
            broadcast_handler(message, &room_id, &app_state).await;
        }
    }
    /*
    If we want to know which function out of there was causing the more time , then we can add
    instrument to those functions as well.

    redis_connection.get_room_meta(&room_id).await
    redis_connection.set_pause(&room_id, false).await
    redis_connection.get_current_player(&room_id).await
    redis_connection.update_current_bid(...).await
    broadcast_handler(...).await

*/
}