use std::collections::HashMap;
use std::sync::Arc;
use axum::body::Bytes;
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::extract::ws::{WebSocket, Message};
use axum::response::IntoResponse;
use crate::models::app_state::AppState;
use crate::models::auction_models::{AuctionParticipant, Bid, ChatMessage, ParticipantAudio, RoomMeta};
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use crate::{services};
use crate::models::background_db_tasks::{DBCommandsAuctionRoom, CompletedRoom};
use crate::models::room_models::Participant;
use crate::models::webRTC_models::SignalingMessage;


pub async fn ws_handler(ws: WebSocketUpgrade, Path((room_id, participant_id)): Path<(String, i32)>, State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| socket_handler(socket, room_id, participant_id, app_state))
}

async fn socket_handler(web_socket: WebSocket, room_id: String,participant_id: i32, app_state: Arc<AppState>) {
    tracing::info!("A new websocket connection has been established");
    // if room doesn't exist, we are going to create a broadcast channel over here
    let mut rooms = app_state.rooms.write().await;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>() ;
    let (mut sender, mut receiver) = web_socket.split() ;


    // as the sender and receiver were not clonable, we are using a channel to send messages to the clients



    // here we are going to set the redis room, if room doesn't exist we are going to create and add participant,
    // else we are going to add the participant to the room that is existing, and also checks whether the participant
    // is already exists or not

    /*
        over here we are going get the room_status, then if room_status was not started then we are going to check,
        whether the participant is already in redis , if exists we are going to return those details , else if the room
        status was in_progress then we are going to check, if the participant was exists in redis , if not we are going
        to return by sending the message room is closed and auction started, then if room_status was completed then we
        are going to return, that auction was completed room is close.

    */
    let redis_connection = app_state.redis_connection.clone();
    let room_status = app_state.database_connection.get_room_status(room_id.clone()).await ;
    let room_mode = app_state.database_connection.get_room_mode(&room_id).await ;
    let room_mode = match room_mode {
       Ok(room_mode) => room_mode,
        Err(err) =>{
            tracing::error!("error in getting room mode {}", err) ;
            sender.send(Message::text("Server Side Error, Unable to get room-mode")).await.expect("unable to send message");
            return;
        }
    } ;
    let room_status = match room_status {
        Ok(room_status) => room_status,
        Err(err) => {
            tracing::error!("error in getting room_status in ws {}", err) ;
            sender.send(Message::text("Server Side Error, Unable to create connection")).await.expect("unable to send message");
            return;
        }
    } ;
    let result = app_state.database_connection.get_team_name(participant_id).await;  // getting team name from the participant
    let team_name = match result {
        Ok(result) => {
            result
        },Err(err ) => {
            tracing::warn!("unable to get team_selected by a participant {}", err) ;
            sender.send(Message::text("Server Side Error, Unable to create connection")).await.expect("unable to send message");
            drop(rooms) ;
            return;
        }
    } ;


    if let Some(vec) = rooms.get_mut(&room_id) {
        {
            // tracing::info!("*** doing a double check if user was opened in multiple tabs") ;
            // let mut index = 0 ;
            // for participant in vec.iter() {
            //     if participant.0 == participant_id {
            //         tracing::info!("we got a duplicate") ;
            //         break;
            //     }
            //     index += 1 ;
            // }
            // if index < vec.len() {
            //     tracing::info!(" as index is less than the number of participants, closing the transaction") ;
            //     let mut previous_value = vec.remove(index) ;
            //     let _ = previous_value.1.send(Message::text("close-connection"));
            //     // we need this close frame, and wait for this to complete
            //     /*
            //         Browser refresh
            //         │
            //         ├─ Old WS still open
            //         │
            //         ├─ New WS connects (same participant_id)
            //         │
            //         ├─ Server removes old tx and sends "close-connection"
            //         │
            //         ├─ Browser receives CLOSE frame for OLD socket
            //         │   ❌ Browser may treat it as connection-level close
            //         │
            //         ├─ New socket handshake is still in progress
            //         │
            //         └─ Browser reports WebSocket error
            //
            //         So we are waiting to execute the whole thing
            //     */
            // } this logic still failing sometimes that too only in chrome browser, but working in firefox
        }
        vec.push((participant_id, tx)) ;
        tracing::info!("Room exists, adding participant {}", participant_id);
    }else{
        tracing::info!("Room doesn't exist, creating a new room {}", room_id);
        rooms.insert(room_id.clone(), vec![(participant_id, tx)]);
        tracing::info!("Creating room in redis") ;
    }
    drop(rooms); // release lock early

    if room_status == "not_started" || room_status == "in_progress" {
        // over here we are going to check room-status if room-status was not-started or pending, if it is finished, then return
         // we stored the tx, which is used to send the data to the receiver channel

        if  !redis_connection.check_room_existence(&room_id).await.unwrap() {
            tracing::info!("creating room in redis as it doesn't exists in redis") ;
            redis_connection.set_room_meta(&room_id, RoomMeta {
                room_creator_id: participant_id,
                pause: false
            }).await.expect("Room unable to Create");
        }
        let participant_exists = redis_connection.check_participant(&room_id, participant_id).await ;
        let participant_exists = match participant_exists {
            Ok(participant_exists) => participant_exists,
            Err(err) => {
                tracing::error!("unable to get the check participant in redis") ;
                tracing::warn!("error was {}", err) ;
                sender.send(Message::text("Server Side Error, Unable to create connection")).await.expect("unable to send message");
                return;
            }
        } ;
        // over here we are going to add participant to the redis
        if !participant_exists && room_status == "not_started" {
            let result = redis_connection.set_participant(&room_id, AuctionParticipant::new(
                participant_id,
                team_name.clone(),
                3 // by default for each and every team having 3 rtms
            )).await ;

            match result {
                Ok(val) => {
                    tracing::info!("participant added to the redis ") ;
                }  ,
                Err(err) => {
                    tracing::warn!("error in the adding participant to the redis was {}", err) ;
                    sender.send(Message::text("Server Side Error, Unable to create connection")).await.expect("unable to send message");
                    return;
                }
            };
            tracing::info!("sending the new participant to all other participants") ;
            let participant = AuctionParticipant {
                id: participant_id,
                team_name: team_name.clone(),
                balance: 100.00,
                total_players_brought: 0,
                remaining_rtms: 3,
                is_unmuted: true,
                foreign_players_brought: 0
            } ;
            broadcast_handler(Message::from(serde_json::to_string(&participant).unwrap()), &room_id, &app_state).await;
            tracing::info!("new member has joined in the room {} and with team {}", room_id, team_name) ;

        }else if !participant_exists {
          tracing::info!("participant not exists and room_status was in_progress") ;
            send_himself(Message::text("Auction Started Room was close"), participant_id, &room_id, &app_state).await ;
            return;
        } else{
            let Some(participant) = redis_connection.get_participant(&room_id,participant_id).await.unwrap() else {
                tracing::error!("The participant was in the redis room but we are not getting the participant from the get_participant") ;
                sender.send(Message::text("Your not in the room")).await.expect("unable to send message");
                return;
            } ;
            tracing::info!("sending the new participant to all other participants") ;
            // here we are going to get the details of the old participant, and sending the old participant details
            broadcast_handler(Message::from(serde_json::to_string(&participant).unwrap()), &room_id, &app_state).await;
            tracing::info!("new member has joined in the room {} and with team {}", room_id, team_name) ;

        }

    }else if room_status == "completed" {
        sender.send(Message::text("Auction was completed, Room was Closed")).await.expect("unable to send the message to the sender") ;
    }



    tracing::info!("getting all participants") ;
    // we need to send the remaining participants list over here
    let mut participants = redis_connection.list_participants(&room_id).await ;
    // tracing::info!("participants were ---------> {:?}", participants.unwrap()) ;
    // before sending let's revamp the current active connections
    let mut participants = match participants {
        Ok(participants) => participants,
        Err(err) => {
            tracing::error!("The error occurred in auction room websockets where sending the list of participants") ;
            return;
        }
    } ;
        // first convert to hashmap
        let mut hashmap = HashMap::new() ;
    {
        for participant in app_state.rooms.read().await.get(&room_id).unwrap().iter() {
            hashmap.insert(participant.0, true) ;
        }
    }
        participants.retain(|p| hashmap.contains_key(&p));
    let mut participant_object: Vec<AuctionParticipant> = vec![];
    for participant in participants.iter() {
        let participant_obj = redis_connection.get_participant(&room_id, *participant).await.unwrap().unwrap() ;
        participant_object.push(participant_obj) ;
    }
        send_himself(
            Message::from(serde_json::to_string(&participant_object).unwrap()),participant_id, &room_id, &app_state
        ).await ; //> sending remaining participants their team name and participant_id
        tracing::info!("sent all active participants list to the participant") ;

    let msg ;
    if room_mode {
        msg = Message::text("strict-mode") ;
        send_himself(msg, participant_id, &room_id, &app_state).await ;
    }


    tokio::spawn({
        let room_id = room_id.clone();
        let participant_id = participant_id;
        let app_state = app_state.clone();

        async move {
            while let Some(msg) = rx.recv().await {

                // ⃣ Checking if message is Text
                if let Message::Text(text) = &msg {
                    if text.as_str() == "close-connection" {
                        tracing::info!(
                        "Closing connection for participant {}",
                        participant_id
                    );

                        // Send WebSocket Close frame (graceful)
                        let _ = sender.send(Message::Close(None)).await;

                        // Removing participant from room (DROPING sender)
                        let mut rooms = app_state.rooms.write().await;
                        if let Some(vec) = rooms.get_mut(&room_id) {
                            vec.retain(|(id, _)| *id != participant_id);
                        }

                        break; // exiting loop → rx dropped → task ends
                    }
                }

                // 4️⃣ Normal forwarding
                if let Err(err) = sender.send(msg).await {
                    tracing::warn!("WebSocket send failed: {}", err);

                    // Cleanup on failure
                    let mut rooms = app_state.rooms.write().await;
                    if let Some(vec) = rooms.get_mut(&room_id) {
                        vec.retain(|(id, _)| *id != participant_id);
                    }

                    break;
                }
            }

            tracing::info!("Forwarding task ended for {}", participant_id);
        }
    });






    // till now what happened was :
    /* when the user clicked join room, by entering room_id, he will get the available teams, then he can choose the team,
    * and click continue, now at that point it will hit the end point and returns participant id, now using that participant-id
    * and room-id the websocket connection will be created and he is taken to the auction room , and for the creator of the room,
    * he will be having button called start auction. when auction starts the state of the room changes, so no other people can join
    * only already an existing participant can join, when he was joining his participant id was in the list then only he
    * can join.
    */


    let expiry_time = std::env::var("BID_EXPIRY").unwrap().parse::<u8>().unwrap();
    let timer_key = format!("auction:timer:{}", room_id); // if this key exists in the redis then no bids takes place
    let rtm_timer_key = format!("auction:timer:rtms:{}", room_id) ;
    // let's read continuous messages from the client
    while let Some(message) = receiver.next().await {
        match message {
            Ok(message) => {
                tracing::info!("Received message: {:?}", message);
                match message {
                    Message::Text(text) => {
                        tracing::info!("Received text message: {}", text);
                        let text = text.to_string() ;
                        // if a bid message was sent, then we are going to check for allowance
                        if text == "ping" {
                            tracing::info!("a ping message in room {}", room_id) ;
                            send_himself(Message::Pong(Bytes::from_static(b"pong")), participant_id,&room_id,&app_state).await ;
                            continue;
                        }else if text == "mute" || text == "unmute" {
                            let value = text ;
                            tracing::info!("{} message was received", value) ;
                            let x = team_name.to_string()+ " " + &value.clone() + "d" ; // the message will be mute-12, means participant 12 has muted himself
                            broadcast_handler(Message::text(x), &room_id, &app_state).await ;
                            // from now we are going to store the mute and unmute states
                            let val ;
                            if value == "mute" {
                                val = false ;
                            }else {
                                val = true ;
                            }
                            redis_connection.toggle_mute(&room_id, participant_id, val).await.expect("Unable to update mute and unmute status") ;
                            broadcast_handler(Message::from(serde_json::to_string(&ParticipantAudio {
                                participant_id,
                                is_unmuted: val
                            }).unwrap()), &room_id, &app_state).await ;
                        }else if text == "start" {

                            tracing::info!("going to call the start auction function") ;
                            services::auction_logic_executor::start_auction(&room_id, participant_id, &app_state, expiry_time, room_mode).await ;

                        }else if text == "bid" {
                            // room_id: String, participant_id: i32,app_state: &AppState, timer_key: String, team_name: &str, room_mode: bool, expiry_time: u8
                            tracing::info!("going to execute the bid logic") ;
                            services::auction_logic_executor::bid(&room_id, participant_id,&app_state, &timer_key, &team_name, room_mode, expiry_time).await ;

                        } else if text == "end" {
                            // ending the auction
                            // when we click on end we are getting only exit as the message without any reason
                            // check whether he was the creator of the room

                                    if redis_connection.get_room_meta(&room_id).await.unwrap().unwrap().room_creator_id != participant_id {
                                        send_himself(Message::text("Only Creator can have permission"), participant_id, &room_id, &app_state).await ;
                                    }else if redis_connection.check_key_exists(&rtm_timer_key).await.unwrap() {
                                        send_himself(Message::text("During RTM You cannot End the Auction"), participant_id, &room_id, &app_state).await ;
                                    }else{
                                        tracing::info!("deleting the timer key ") ;
                                        redis_connection.atomic_delete(&timer_key).await.unwrap() ;
                                        // second, check whether all the participants having least 15 players in their squad
                                        // no need to have a condition to have at least 15 players in each squad
                                        tracing::info!("cleaning up the redis keys related to the auction") ;
                                        let res = redis_connection.auction_clean_up(&room_id).await ;
                                        let message ;
                                        match res {
                                            Ok(res) => {
                                                if res {
                                                    // when front-end has disconnected automatically it's going to be the end.
                                                    // we are going to change the state of the auction to completed such that this room get's invalid
                                                    match app_state.database_connection.update_room_status(&room_id, "completed").await {
                                                        Ok(_) => {
                                                            tracing::info!("room status changed to completed") ;
                                                            // here we are going to remove the data from redis
                                                            tracing::info!("successfully removed the room from redis") ;
                                                            // we are going to make sure add the completed_at field and also unsold players list from this auction

                                                            // deleting the unsold players list
                                                            //app_state.database_connection.remove_unsold_players(&room_id).await.expect("error occurred while deleting unsold players") ;
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
                                                            /*
                                                                over here we are going to assign a background task which removes unsold players from the unsold_players table
                                                                and store it in another table , called list of completed rooms unsold players.

                                                                And then if the room was created and was not been in completed status after 24 hours a cron job,
                                                                will executed and change the status and remove the redis logic and everything, for every 24 hours.
                                                                or when ever i run the logic explicitly

                                                                another logic, was asking the user to allow location when they are signing in.

                                                                lastly adding connector_id to disable multiple joining of room.
                                                            */
                                                            // updating the set_completed_at
                                                            

                                                            message = Message::text("exit") ; // in front-end when this message was executed then it must stop the ws connection with server
                                                        },
                                                        Err(err) => {
                                                            tracing::info!("unable to update the room status to completed") ;
                                                            tracing::error!("{}",err) ;
                                                            message = Message::text("Technical Issue") ;
                                                        }
                                                    }

                                                }else {
                                                    message = Message::text("Unable to End Auction, Due to Technical Problem") ;
                                                }
                                            },
                                            Err(_) => {
                                                tracing::info!("Unable to get the room") ;
                                                message = Message::text("Till all participants brought at least 15 player") ;
                                            }
                                        } ;
                                        broadcast_handler(message,&room_id,&app_state).await ;
                                    }

                        }else if text == "pause" {

                            // we are going to pause the auction, such that when clicked create again, going to start from the last player
                                    if redis_connection.get_room_meta(&room_id).await.unwrap().unwrap().room_creator_id == participant_id {
                                        // we are going to pause auction after the current bid
                                        match redis_connection.set_pause(&room_id, true).await {
                                            Ok(_) => {
                                                tracing::info!("successfully set the status to pause") ;
                                                send_himself(Message::text("After the Current Bid Auction will be Paused"), participant_id, &room_id, &app_state).await ;
                                            },
                                            Err(err) => {
                                                tracing::error!("error occurred while setting the pause status") ;
                                                tracing::error!("err was {}", err) ;
                                                send_himself(Message::text("Technical Problem"), participant_id, &room_id, &app_state).await ;
                                            }
                                        } ;
                                    }else {
                                        send_himself(Message::text("Only Creator can have permission"), participant_id, &room_id, &app_state).await ;
                                    }

                        }else if text == "rtm-accept" { // need to check RTM, why even timer was there it was failing and also need to check whether the RTM timer was the expiry time
                            // can only be called, if the key was rtms
                            if redis_connection.check_key_exists(&rtm_timer_key).await.unwrap() {
                                tracing::info!("rtm was being accepted") ;
                                redis_connection.atomic_delete(&rtm_timer_key).await.unwrap();
                                // accepting the bid
                                let bid = redis_connection.get_current_bid(&room_id).await.unwrap().unwrap() ;
                                let bid = Bid::new(participant_id, bid.player_id, bid.bid_amount, bid.base_price, false, true) ;
                                // adding the bid to the redis
                                match redis_connection.update_current_bid(&room_id, bid, 1, participant_id, room_mode, true).await {
                                    Ok(_) => {},
                                    Err(err) => {
                                        if err.contains("Bid not allowed") {
                                            send_himself(Message::text(&err), participant_id, &room_id, &app_state).await ;
                                        }else {
                                            send_himself(Message::text("Technical Issue"), participant_id, &room_id, &app_state).await ;
                                        }
                                    }
                                };
                            }else {
                                send_message_to_participant(participant_id, String::from("Invalid RTM was not taken place"), &room_id, &app_state).await ;
                            }
                        }else if text == "instant-rtm-cancel" {

                            services::auction_logic_executor::instant_rtm_cancel(&room_id, participant_id, &app_state, room_mode, &rtm_timer_key).await ;

                        } else if text.contains("rtm-cancel") {

                            services::auction_logic_executor::rtm_cancel(&room_id, participant_id, &app_state, room_mode, &rtm_timer_key).await ;

                        } else if text.contains("rtm") {

                            // room_id: &str, participant_id: i32, app_state: &AppState, room_mode: bool, rtm_timer_key: &str, text: String, expiry_time: u8
                            services::auction_logic_executor::use_rtm(&room_id, participant_id, &app_state, room_mode, &rtm_timer_key, text, expiry_time).await ;

                        }else if text == "skip" {

                            // room_id: &str, participant_id: i32, app_state: &AppState, timer_key: &str, room_mode: bool, text: String, team_name: &str
                            tracing::info!("executing skip") ;
                            services::auction_logic_executor::skip(&room_id, participant_id, &app_state, &timer_key, room_mode, text, &team_name).await ;

                        }else if text.contains("chat-") {
                          tracing::info!("received the message from the team {}", team_name) ;
                            // chat-message
                            let message = text.split("-").collect::<Vec<&str>>()[1].parse::<String>().unwrap() ;
                            tracing::info!("message was {}", message) ;

                            // no stroing chats in the db, because it was an emergency case, when the audio communication was lost
                            broadcast_handler(Message::from(serde_json::to_string(
                                &ChatMessage{
                                    team_name: team_name.clone(),
                                    message
                                }
                            ).unwrap()),&room_id,&app_state).await ;

                            /*
                                we need to make sure notify the other users, with no of chats , with chat symbol on top for
                                mobile users , what ever message they got. when the chat symbol is clicked it will be hovered on
                                auction, with transparent background, they can see chats as well as the auction.
                            */

                        }else if text == "skip-current-pool" {

                            // going to add a new feature, so maintain a separate state in the redis
                            /*
                               -> using a hash set for storing list of users skipped the current pool
                               -> so if every one skipped the pool , then we will jump into the next pool
                               in between even a player skipped the pool is allowed to bid for the players
                               in pool, once every one has skipped then the pool will be jumped to another pool.
                               -> If all skipped, from the next player we are going to jump to the next pool.
                            */
                            tracing::info!("*=* skip current pool called");
                            redis_connection.mark_participant_skipped_pool(&room_id, participant_id).await.map_err(
                                |e| {
                                    tracing::error!("marking participant skipped pool room_id {} and participant_id {}", room_id, participant_id) ;
                                    tracing::error!("error was {} for room_id {}", e, room_id) ;
                                }
                            ).unwrap() ;
                            broadcast_handler(Message::text(&format!("{} want's to skip the current-pool", team_name)),&room_id,&app_state).await ;

                        }else if text == "get-is-skipped-pool" {
                            tracing::info!("asking whether he {} skipped pool or not", participant_id) ;

                            let result = redis_connection.is_participant_skipped_pool(&room_id, participant_id).await.map_err(|err| {

                            }).unwrap() ;
                            let message ;
                            if result {
                                tracing::info!("he skipped") ;
                                message = Message::text("is_skipped:true") ;
                            }else{
                                tracing::info!("he not skipped yet") ;
                                message = Message::text("is_skipped:false") ;
                            }
                            send_himself(message, participant_id, &room_id, &app_state).await ;
                        }
                        else {
                            let message ;
                            let to_participant ;
                            tracing::info!("******************* Message for WeB RTC was ***************************") ;
                            tracing::info!("here is the front-end passed message {}", text.to_string()) ;
                            tracing::info!("****************************** *****************************************") ;
                            if !(text.starts_with('{') && text.ends_with('}')) {
                                tracing::warn!("Ignoring non-JSON message: {}", text);
                                continue;
                            }
                            let parsed: SignalingMessage = match serde_json::from_str(&text) {
                                Ok(msg) => msg,
                                Err(err) => {
                                    eprintln!("Failed to parse signaling message: {}", err);
                                    send_himself(Message::text("Unable to parse what you have sent"), participant_id, &room_id, &app_state).await ;
                                    continue;
                                }
                            };

                            match parsed {

                                SignalingMessage::Offer { from, to, payload } => {
                                    tracing::info!("got the offer from {} and to {}", from, to) ;
                                    message = Message::from(serde_json::to_string(&SignalingMessage::Offer {from, to, payload}).unwrap()) ;
                                    to_participant = to ;
                                },
                                SignalingMessage::Answer { from, to, payload } => {
                                    // forward to participant `to`
                                    tracing::info!("got the answer from {} and to {}", from, to) ;
                                    message = Message::from(serde_json::to_string(&SignalingMessage::Answer {from, to, payload}).unwrap()) ;
                                    to_participant = to ;
                                }

                                SignalingMessage::IceCandidate { from, to, payload } => {
                                    // forward to participant `to`
                                    tracing::info!("got the ice-candidate from {} and to {}", from, to) ;
                                    message = Message::from(serde_json::to_string(&SignalingMessage::IceCandidate {from, to, payload}).unwrap()) ;
                                    to_participant = to ;
                                }

                                _ => {
                                    message = Message::text("Invalid Message") ;
                                    to_participant = participant_id ;
                                }
                            };

                            send_himself(message, to_participant, &room_id, &app_state).await ;

                        }

                    },
                    Message::Close(_) => {
                        tracing::info!("Client disconnected");
                        handle_disconnect(&room_id, participant_id, team_name, &app_state).await ;
                        return;
                    }
                    Message::Binary(bytes) => todo!(),
                    Message::Ping(bytes) => {
                        // Browser will NEVER reach here.
                        // Only servers / Node clients can trigger this.
                    },
                    Message::Pong(bytes) => todo!(),
                }
            },
            Err(err) => {
                tracing::warn!("disrupt disconnect, mostly on page refreshes") ;
                handle_disconnect(&room_id, participant_id, team_name, &app_state).await ;
                return
            },
        }
    }
}


pub async fn handle_disconnect(room_id: &str, participant_id: i32, team_name: String,app_state: &AppState) {
    // we are removing the disconnected client, such that the unbounded channel will not overload if queue is filled with multiple disconnected message to client
    let mut value = app_state.rooms.write().await ;
    let mut index: u8 = 0 ;
    let mut user_exists = false ;
    for participant in value.get(room_id).unwrap().iter() {
        if participant.0 == participant_id {
            user_exists = true ;
            break
        }
        index += 1 ;
    }

    /*
        need to broadcast which participant has been disconnected, and when joins we are any way sending the message
    */

    if user_exists {
        value.get_mut(room_id).unwrap().remove(index as usize);
    }
    drop(value) ;
    broadcast_handler(Message::from(serde_json::to_string(&Participant { participant_id, team_name }).unwrap()), room_id, &app_state).await ;
}

pub async fn broadcast_handler(msg: Message,room_id: &str, state: &AppState) { // if we want to send a message to all the participants in the room, we use broadcaster
    // over here we are going to get all the participants from the room-id
    // and send the message to all the participants
    let mut rooms = state.rooms.read().await;
    for sender in rooms.get(room_id).unwrap().iter() {
        if let Ok(_) = sender.1.send(msg.clone()) {
            tracing::info!("Message sent to participant successfully");
        }else{
            tracing::info!("Failed to send message to participant");
        }
    }
} // lock drops over here

pub async fn send_himself(msg: Message, participant_id: i32,room_id: &str, state: &AppState) {
    let mut rooms = state.rooms.read().await;
    for sender in rooms.get(room_id).unwrap().iter() {
        if participant_id == sender.0 {
            if let Ok(_) = sender.1.send(msg.clone()) {
                tracing::info!("Message sent to participant successfully");
            }else{
                tracing::error!("Failed to send message to participant");
            }
        }
    }
}


#[tracing::instrument(
    name = "bid_allowance_handler",
    fields(
        current_bid = current_bid,
        balance = balance,
        total_players_brought = total_players_brought,
        strict_mode = strict_mode
    )
)]
pub async fn bid_allowance_handler(
    current_bid: f32,
    balance: f32,
    total_players_brought: u8,
    strict_mode: bool,
) -> bool {

    tracing::warn!("Strict mode was {}", strict_mode) ;
    let remaining_balance = balance - current_bid;
    if remaining_balance < 0.0 {
        return false;
    }
    // added a new mode for playing a strategic auction.
    if strict_mode {
        tracing::warn!("Inside Strict Mode") ;
        // -------------------------
        // RULE A: GLOBAL LIMITS
        // -------------------------
        let min_required_balance = match total_players_brought {
            0..=4 => 50.0,
            5..=9 => 10.0,
            10..=11 => 4.0,
            12..=14 => 0.0,
            _ => 0.0,
        };

        tracing::warn!("minimum required balance is {}", min_required_balance) ;

        if remaining_balance <= min_required_balance {
            return false;
        }

        // -------------------------
        // RULE B: PER-PLAYER BUFFER
        // -------------------------
        // Determine segment buffer
        let (segment_max_players, buffer_per_player) = match total_players_brought {
            0..=4 => (5, 5.0),
            5..=9 => (10, 4.0),
            10..=14 => (15, 1.0),
            _ => (15, 0.0),
        };
        tracing::warn!("segment max players is {}", segment_max_players) ;
        tracing::warn!("buffer per player is {}", buffer_per_player) ;
        let mut remaining_players_in_segment =
            (segment_max_players as i32 - total_players_brought as i32).max(0);
        tracing::warn!("remaining players in the current segment {}", remaining_players_in_segment) ;
        if remaining_players_in_segment != 0 {
            remaining_players_in_segment -= 1 ; // we need to exclude the current bidding player
        }
        tracing::warn!("after excluding current player {}", remaining_players_in_segment) ;
        let required_buffer = remaining_players_in_segment as f32 * buffer_per_player;
        tracing::warn!("required buffer is {}", required_buffer) ;
        if remaining_balance < required_buffer {
            return false;
        }
        tracing::warn!("remaining balance {}", balance) ;
        let remaining_amount_in_that_segment = (remaining_balance - min_required_balance)-required_buffer ;
        tracing::warn!("remaining balance in that segment {}", remaining_amount_in_that_segment) ;
        // if required buffer is 20 , from remaining balance 100 - min_required_balance which is 50
        // remaining is 50 cr , from that 50cr we need to subract the current bid amound
        if remaining_amount_in_that_segment >= 0.0 {
            tracing::warn!("this bid is allowed") ;
            return true;
        }


        tracing::warn!("this bid was not allowed in the strict mode") ;
        return false;
    }

    // FREE MODE LOGIC
    let total_players_required = 15 - total_players_brought as i32;
    let money_required = total_players_required as f32 * 0.30;
    remaining_balance >= money_required
}


/*
    while generating use of RTM also , we need to make sure that the previous team has the ability to get foreign player or not.
*/

pub async fn send_message_to_participant(participant_id: i32, message: String, room_id: &str, state: &AppState) {
    let mut rooms = state.rooms.read().await;
    for sender in rooms.get(room_id).unwrap().iter() {
        if participant_id == sender.0 {
            if let Ok(_) = sender.1.send(Message::text(&message)) {
                tracing::info!("Message sent to participant successfully");
                break;
            }else{
                tracing::info!("Failed to send message to participant");
                break;
            }
        }
    }
}



