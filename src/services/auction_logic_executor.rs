use std::time::Instant;
use axum::extract::ws::Message;
use crate::auction::{bid_allowance_handler, broadcast_handler, send_himself, send_message_to_participant};
use crate::models;
use crate::models::app_state::AppState;
use crate::models::auction_models::{Bid, BidOutput};
use crate::models::background_db_tasks::DBCommandsAuctionRoom;
use metrics::counter ;
use crate::services::other::get_previous_team_full_name;


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
pub async fn start_auction(room_id: &str, participant_id: i32, app_state: &AppState, expiry_time: u8, room_mode: bool) {
    let start = Instant::now();

    counter!("auction_start_total").increment(1);


    let redis_connection = app_state.redis_connection.clone();
    if redis_connection.get_room_meta(room_id).await.unwrap().unwrap().room_creator_id != participant_id {
        counter!("auction_start_denied_total").increment(1);

        send_himself(Message::text("You will not having permissions"), participant_id, room_id, app_state).await;
    } else {
        if app_state.rooms.read().await.get(room_id).unwrap().len() < 3 {
            send_himself(Message::text("Min of 3 participants should be in the room to start auction"), participant_id, room_id, app_state).await;
        } else {
            match redis_connection.set_pause(room_id, false).await {
                Ok(_) => {
                    tracing::info!("successfully set the status to pause");
                    // send_himself(Message::text("After the Current Bid Auction will be Paused"), participant_id, room_id, &app_state).await ;
                },
                Err(err) => {
                    metrics::counter!("failed while checking pause status in auction_start").increment(1) ;
                    tracing::error!("error occurred while setting the pause status");
                    tracing::error!("err was {}", err);
                    send_himself(Message::text("Technical Problem"), participant_id, room_id, app_state).await;
                }
            };

            // we are going to return the first player from the auction
            let player = redis_connection.get_current_player(room_id).await.unwrap();
            let player = match player {
                Some(player) => {
                    tracing::info!("got the current player");
                    player
                },
                None => {
                    tracing::warn!("I guess auction was just starting no current player");
                    // we need to get the 1st player
                    let player = redis_connection.get_player(1, room_id).await.unwrap();
                    redis_connection.set_current_player(room_id, player.clone()).await.unwrap();
                    player
                }
            };
            let message;

            if player.id == 1 {
                // // changing room-status
                app_state.auction_room_database_task_executor.send(DBCommandsAuctionRoom::UpdateRoomStatus(models::background_db_tasks::RoomStatus {
                    room_id: room_id.to_string(),
                    status: "in_progress".to_string(),
                })).map_err(|err| {
                    metrics::counter!("failed while sending update status in auction_start").increment(1);
                }).expect("Failed to send");
            }

            message = Message::from(serde_json::to_string(&player).unwrap());
            // here we are going to add the player as Bid to the redis
            let bid = Bid::new(0, player.id, 0.0, player.base_price, false, false); // no one yet bidded
            redis_connection.update_current_bid(room_id, bid, expiry_time, -1, room_mode).await.expect("unable to update the bid");


            // broadcasting
            broadcast_handler(message, room_id, app_state).await;
        }
    }
    // ---------- SUCCESS ----------
    metrics::counter!("auction_start_success_total").increment(1);

    let elapsed = start.elapsed().as_secs_f64();
    metrics::histogram!("auction_start_duration_seconds").record(elapsed);
    /*
    If we want to know which function out of there was causing the more time , then we can add
    instrument to those functions as well.

    redis_connection.get_room_meta(room_id).await
    redis_connection.set_pause(room_id, false).await
    redis_connection.get_current_player(room_id).await
    redis_connection.update_current_bid(...).await
    broadcast_handler(...).await

*/
}


#[tracing::instrument(
    name = "bid",
    skip(app_state),
    fields(
        room_id = %room_id,
        participant_id = participant_id,
        team_name = team_name,
        room_mode = room_mode,
        expiry_time = expiry_time
    )
)]
pub async fn bid(room_id: &str, participant_id: i32,app_state: &AppState, timer_key: &str, team_name: &str, room_mode: bool, expiry_time: u8) {
    let redis_connection = app_state.redis_connection.clone() ;
    /*
        If previously the same participant has send the bid, then that shouldn't be considered

     */
    let current_bid = redis_connection.get_current_bid(room_id).await.unwrap() ;
    let mut current_bid = match current_bid {
        Some(bid) => bid,
        None => {
            tracing::warn!("there no current bid that we got over here") ;
            return;
        }
    } ;
    let participant = redis_connection.get_participant(room_id, participant_id).await.unwrap() ;
    let participant =  match participant {
        Some(participant) => participant,
        None => {
            tracing::warn!("no current participant we got over here") ;
            return
        }
    } ;
    let current_player = redis_connection.get_current_player(room_id).await.unwrap().unwrap() ;
    // if this key exists in the redis then only bids takes place
    if !redis_connection.check_key_exists(timer_key).await.unwrap() {
        tracing::info!("as the key doesn't exists we are not going to take this bid") ;
        send_himself(Message::text("Bid is Invalid, RTM is taking place"), participant_id, room_id, app_state).await ;
    } else if (!current_player.is_indian) && (participant.foreign_players_brought >= 8) {
        tracing::info!("foreign players has reached max for the participant, so bid becomes invalid") ;
        send_himself(Message::text("You reached Foreign Player limit"), participant_id, room_id, app_state).await ;
    } else {
        if redis_connection.is_skipped(room_id, participant_id).await.unwrap() {
            tracing::info!("skipped the player, the bid is not valid any more") ;
            send_himself(Message::text("Bid is Invalid, you skipped the player"), participant_id, room_id, app_state).await ;
        } else if app_state.rooms.read().await.get(room_id).unwrap().len() >= 3 {
            // the participant has bided
            if current_bid.participant_id != participant_id {
                current_bid.participant_id = participant_id ;
                let expiry_time_ ;
                if redis_connection.get_skipped_count(room_id).await.unwrap() as usize == app_state.rooms.read().await.get(room_id).unwrap().len() - 1 {
                    expiry_time_ = 1 ;
                }else {
                    expiry_time_ = expiry_time ;
                }
                tracing::info!("the current bid {:?}", current_bid);
                let result =  redis_connection.update_current_bid(room_id, current_bid, expiry_time_, participant_id, room_mode).await ;
                match result {
                    Ok(amount) => {
                        let message = Message::from(serde_json::to_string(&BidOutput{
                            bid_amount: amount,
                            team: team_name.to_string()
                        }).unwrap()) ;
                        broadcast_handler(message,room_id,app_state).await ;
                    }, Err(err) => {
                        if err.contains("Bid not allowed") {
                            send_himself(Message::text(&err), participant_id, room_id, app_state).await ;
                        }else {
                            send_himself(Message::text("Technical Issue"), participant_id, room_id, app_state).await ;
                        }

                    }
                } ;
            }else {
                send_himself(Message::text("You are already the highest bidder"), participant_id,room_id,app_state).await
            }

        }else{
            //  we are going to stop the auction at this point, we will keep another state in redis, called pause, if it is true then we are going to pause the auction
            // make the auction paused
            redis_connection.atomic_delete(timer_key).await.unwrap() ;
            // we need update the current_bid as well
            current_bid.participant_id = 0 ;
            current_bid.bid_amount = 0.0 ;
            redis_connection.update_current_bid(room_id, current_bid, 0,-1,room_mode).await.unwrap() ;
            send_himself(Message::text("Min of 3 participants should be in the room to bid"), participant_id,room_id,app_state).await ;
        }
    }
}



#[tracing::instrument(
    name = "skip_player",
    skip(app_state),
    fields(
        room_id = %room_id,
        participant_id = participant_id,
        room_mode = room_mode,
        team_name = team_name
    )
)]
pub async fn skip(room_id: &str, participant_id: i32, app_state: &AppState, timer_key: &str, room_mode: bool, text: String, team_name: &str) {
    let redis_connection = app_state.redis_connection.clone() ;
    // if skip-s, then it's a strict-mode.
    tracing::info!("message skip was received") ;
    // we need to add a state in redis
    let mut skipped_count = redis_connection.mark_skipped(room_id, participant_id).await.unwrap() ;
    let live_participants_count = { app_state.rooms.read().await.get(room_id).unwrap().len() } as u8;
    tracing::info!("total participants skipped till now was {}", skipped_count) ;
    tracing::info!("total live participants {}", live_participants_count) ;
    if skipped_count == live_participants_count - 1 {
        tracing::info!("all participants skipped, except one, checking whether he was the highest bidder") ;
        let current_bid = match redis_connection.get_current_bid(room_id).await.unwrap() {
            Some(bid) => {
                tracing::info!("we got bid from the current bid") ;
                bid
            }, None => {
                tracing::warn!("No Current Bid we got , so there was a problem") ;
                return;
            }
        } ;
        if !redis_connection.is_skipped(room_id, current_bid.participant_id).await.unwrap() && current_bid.participant_id != 0{
            skipped_count += 1;
            tracing::info!("he was the one who not skipped yet, so we are selling to the highest bidder") ;
            // redis_connection.mark_skipped(&room_id, current_bid.participant_id).await.unwrap() ;
            // there is no need to add , any way we are going to sell the player to him, after selling
            // skipped count will be refreshed.
        }
    }
    if skipped_count == live_participants_count {
        if redis_connection.check_key_exists(timer_key).await.unwrap() {
            redis_connection.atomic_delete(timer_key).await.expect("unable to delete the room inside skip");
            let current_bid = redis_connection.get_current_bid(room_id).await.unwrap().unwrap() ;
            redis_connection.update_current_bid(room_id, current_bid,1, -1, room_mode).await.unwrap() ;
        }else {
            let message = "At this Stage Skip won't work";
            send_himself(Message::text(message), participant_id, room_id, app_state).await ;
        }
    }else {
        let message ;
        if text.contains("-") {
            let reason = text.split("-").collect::<Vec<&str>>()[1].parse::<String>().unwrap() ;
            message = format!("{} was out of bid, due to {}", team_name, reason) ;
        }else {
            message = format!("{} skipped the player", team_name) ;
        }
        broadcast_handler(Message::text(&message),room_id,app_state).await ;
    }


    /*
        if a participant skips, then, that participant cannot be bid again
    */
}


#[tracing::instrument(
    name = "instant_rtm_cancel",
    skip(app_state),
    fields(
        room_id = %room_id,
        participant_id = participant_id,
        rtm_timer_key = rtm_timer_key,
        room_mode = room_mode
    )
)]
pub async fn instant_rtm_cancel(room_id: &str, participant_id: i32, app_state: &AppState, room_mode: bool, rtm_timer_key: &str) {
    let redis_connection = app_state.redis_connection.clone() ;
    tracing::info!("cancelling the rtm instantly, where the previous team , don't want to use the rtm for the current player") ;
    redis_connection.atomic_delete(rtm_timer_key).await.unwrap() ;
    let mut current_bid = redis_connection.get_current_bid(room_id).await.unwrap().unwrap() ;
    current_bid.rtm_bid = true ;
    redis_connection.update_current_bid(room_id, current_bid, 1, -1, room_mode).await.unwrap() ;
    send_message_to_participant(participant_id, String::from("Cancelled the RTM"), room_id, app_state).await ;
}


#[tracing::instrument(
    name = "rtm_cancel",
    skip(app_state),
    fields(
        room_id = %room_id,
        participant_id = participant_id,
        rtm_timer_key = rtm_timer_key,
        room_mode = room_mode
    )
)]
pub async fn rtm_cancel(room_id: &str, participant_id: i32, app_state: &AppState, room_mode: bool, rtm_timer_key: &str) {
    let redis_connection = app_state.redis_connection.clone() ;
    tracing::info!("cancelling the offer by the highest bidder") ;
    redis_connection.atomic_delete(rtm_timer_key).await.unwrap() ;
    // now we are going to send the same bid with expiry 0
    let mut current_bid = redis_connection.get_current_bid(room_id).await.unwrap().unwrap() ;
    current_bid.is_rtm = true ;  // where the last bided person is the person who used rtm, so we need to keep it as rtm only, such that his rtms will decreased
    redis_connection.update_current_bid(room_id, current_bid, 1, -1, room_mode).await.unwrap() ;
    send_message_to_participant(participant_id, String::from("Cancelled the RTM Price"), room_id, app_state).await ;
}


#[tracing::instrument(
    name = "using_rtm",
    skip(app_state),
    fields(
        room_id = %room_id,
        participant_id = participant_id,
        rtm_timer_key = rtm_timer_key,
        room_mode = room_mode,
        expiry_time = expiry_time
    )
)]
pub async fn use_rtm(room_id: &str, participant_id: i32, app_state: &AppState, room_mode: bool, rtm_timer_key: &str, text: String, expiry_time: u8) {
    let redis_connection = app_state.redis_connection.clone() ;
    tracing::info!("rtm was accepted with the following {}",text) ;
    // we need to check
    // if this key exists in the redis then no bids takes place
    if redis_connection.check_key_exists(rtm_timer_key).await.unwrap() { // if normal bids were not taking place on in that scenario
        redis_connection.atomic_delete(rtm_timer_key).await.unwrap();
        // rtm-amount eg : rtm-5.00 means increasing 5.00cr from the current price
        let mut bid = redis_connection.get_current_bid(room_id).await.unwrap().unwrap() ;

        let amount = text.split("-").collect::<Vec<&str>>()[1].parse::<f32>().unwrap() ;

        // now we are going to check whether the specific participant has the authority to use the rtm, means the current player
        // previous team should be the participant playing team
        let rtm_placer_participant = redis_connection.get_participant(room_id, participant_id).await.unwrap().unwrap() ;
        let previous_player = redis_connection.get_player(bid.player_id, room_id).await.unwrap() ;
        let full_team_name = get_previous_team_full_name(&previous_player.previous_team);
        let current_participant_team = rtm_placer_participant.team_name ;

        if full_team_name == current_participant_team {
            let new_amount = amount + bid.bid_amount ;
            // here we need to check whether the rtm placer having that much enough money and as well the same other guy having that much enough money
            if rtm_placer_participant.remaining_rtms > 0 {
                let highest_bidder_participant = redis_connection.get_participant(room_id, bid.participant_id).await.unwrap().unwrap() ;
                let rtm_placer_participant_bid_allowance = bid_allowance_handler(new_amount, rtm_placer_participant.balance, rtm_placer_participant.total_players_brought, room_mode).await ;
                let highest_bidder_participant_allowance = bid_allowance_handler(new_amount, highest_bidder_participant.balance, highest_bidder_participant.total_players_brought, room_mode).await ;
                if rtm_placer_participant_bid_allowance && highest_bidder_participant_allowance {
                    tracing::info!("both having money") ;
                    // creating the new Bid
                    let bid_ = Bid::new(participant_id, bid.player_id, new_amount, bid.base_price, true, false) ;
                    // adding the bid to the redis
                    match redis_connection.update_current_bid(room_id, bid_, expiry_time, participant_id, room_mode).await {
                        Ok(_) => {},
                        Err(err) => {
                            if err.contains("Bid not allowed") {
                                send_himself(Message::text(&err), participant_id, room_id, &app_state).await ;
                            }else {
                                send_himself(Message::text("Technical Issue"), participant_id, room_id, app_state).await ;
                            }
                        }
                    };
                    send_message_to_participant(bid.participant_id, format!("rtm-amount-{}", new_amount), room_id, app_state).await ;
                    return;
                }else if rtm_placer_participant_bid_allowance {
                    tracing::info!("rtm bidder has enough money, so bid goes to him") ;
                    // delete the key and add the new bid with expiry 0 seconds

                    // new bid
                    match redis_connection.update_current_bid(room_id, Bid::new(participant_id, bid.player_id, new_amount, bid.base_price, true, false),1, participant_id, room_mode).await {
                        Ok(_) => {},
                        Err(err) => {
                            if err.contains("Bid not allowed") {
                                send_himself(Message::text(&err), participant_id, room_id, app_state).await ;
                            }else {
                                send_himself(Message::text("Technical Issue"), participant_id, room_id, app_state).await ;
                            }
                        }
                    };
                    // send to the highest bidder the reason
                    send_message_to_participant(bid.participant_id, format!("no balance to accept the bid price of {}",new_amount), room_id, app_state).await ;
                    return;
                }else {
                    tracing::info!("only the person having the rtm having the enough money") ;
                    send_himself(Message::text("Invalid Price, You Lost RTM for this Bid"), participant_id, &room_id, &app_state).await ;
                }
            }else{
                send_himself(Message::text("All RTMS were used"), participant_id, room_id, app_state).await ;
            }
        }else {
            send_himself(Message::text("The current player is not in ur team previously"), participant_id, room_id, app_state).await ;
        }
        bid.rtm_bid = true ; // it will be rtm_bid , but for remaining rtms will be same, only thing is in subscriber making sure no infinite loop takes place, where we are going to inifinetly if there previous
        let _ = redis_connection.update_current_bid(room_id, bid, 1, -1, room_mode).await.unwrap() ;
    }else {
        tracing::info!("Now no RTM bids were taking place") ;
        send_himself(Message::text("No RTM Bids are taking place"), participant_id, room_id, app_state).await ;
    }
}