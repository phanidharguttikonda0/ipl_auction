use std::sync::Arc;
use axum::Extension;
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::extract::ws::{WebSocket, Message};
use axum::response::IntoResponse;
use redis::aio::AsyncPushSender;
use tokio::sync::broadcast;
use crate::models::app_state::AppState;
use crate::models::auction_models::{AuctionParticipant, AuctionRoom, Bid, BidOutput, NewJoiner};
use crate::services::auction_room::RedisConnection;
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use crate::models::authentication_models::Claims;

pub async fn ws_handler(ws: WebSocketUpgrade,Extension(user): Extension<Claims> ,Path((room_id, participant_id)): Path<(String, i32)>, State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| socket_handler(socket, room_id, participant_id, app_state))
}

async fn socket_handler(mut web_socket: WebSocket, room_id: String,participant_id: i32, app_state: Arc<AppState>) {
    tracing::info!("A new websocket connection has been established");
    // if room doesn't exist, we are going to create a broadcast channel over here
    let mut rooms = app_state.rooms.write().await;
    let (mut tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>() ;
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
    let mut redis_connection = RedisConnection::new().await;
    let room_status = app_state.database_connection.get_room_status(room_id.clone()).await ;
    let room_status = match room_status {
        Ok(room_status) => room_status,
        Err(err) => {
            tracing::error!("error in getting room_status in ws") ;
            sender.send(Message::text("Server Side Error, Unable to create connection")).await.expect("unable to send message");
            return;
        }
    } ;
    let result = app_state.database_connection.get_team_name(participant_id).await;  // getting team name from the participant
    let team_name = match result {
        Ok(result) => {
            result
        },Err(err ) => {
            tracing::warn!("unable to get team_selected by a participant") ;
            sender.send(Message::text("Server Side Error, Unable to create connection")).await.expect("unable to send message");
            drop(rooms) ;
            return;
        }
    } ;

    let room_exists ;

    if let Some(vec) = rooms.get_mut(&room_id) {
        vec.push((participant_id, tx)) ;
        tracing::info!("Room exists, adding participant {}", participant_id);
        room_exists = true ;
    }else{
        tracing::info!("Room doesn't exist, creating a new room {}", room_id);
        rooms.insert(room_id.clone(), vec![(participant_id, tx)]);
        tracing::info!("Creating room in redis") ;
        room_exists = false;
    }
    drop(rooms); // release lock early
    let participant_exists = redis_connection.check_participant(participant_id, room_id.clone()).await ;
    let participant_exists = match participant_exists {
        Ok(participant_exists) => participant_exists,
        Err(err) => {
            tracing::error!("unable to get the check participant in redis") ;
            sender.send(Message::text("Server Side Error, Unable to create connection")).await.expect("unable to send message");
            return;
        }
    } ;
    if room_status == "not_started" {
        // over here we are going to check room-status if room-status was not-started or pending, if it is finished, then return
         // we stored the tx, which is used to send the data to the receiver channel

        if  !room_exists {
            redis_connection.set_room(room_id.clone(), AuctionRoom::new()).await.expect("Room unable to Create");
        }

        // over here we are going to add participant to the redis
        if !participant_exists {
            let result = redis_connection.add_participant(room_id.clone(), AuctionParticipant::new(
                participant_id,
                team_name.clone()
            )).await ;

            match result {
                Ok(val) => {
                    tracing::info!("participant added to the redis {}", val) ;
                }  ,
                Err(err) => {
                    tracing::warn!("error in the adding participant to the redis was {}", err) ;
                    sender.send(Message::text("Server Side Error, Unable to create connection")).await.expect("unable to send message");
                    return;
                }
            };

            // after joining we need to send that this particular participant with this team has joined the room , to all the
            // participants in the room
            broadcast_handler(Message::from(NewJoiner {
                participant_id,
                team_name: team_name.clone(),
                balance: 100.00
            }), room_id.clone(), &app_state).await;
            tracing::info!("new member has joined in the room {} and with team {}", room_id, team_name) ;
        }else{
            let Some(participant) = redis_connection.get_participant(room_id.clone(),participant_id).await ;
            // here we are going to get the details of the old participant, and sending the old participant details
            broadcast_handler(Message::from(NewJoiner {
                participant_id,
                team_name: team_name.clone(),
                balance: 100.00
            }), room_id.clone(), &app_state).await;
            tracing::info!("new member has joined in the room {} and with team {}", room_id, team_name) ;

        }

    }else if room_status == "in_progress" && !participant_exists || room_status == "completed"{
        // now we are going to check whether the participant exists, if not exists we are going to add him
        // because any way he was in the room , but somehow he was not able to create the ws connection and get into the room
        sender.send(Message::text("room is closed, Auction going on")).await.expect("unable to send message");
        return;
    }

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            sender.send(msg).await.unwrap(); // we are sending the message to the client
        }
    }) ;


    // till now what happened was :
    /* when the user clicked join room, by entering room_id, he will get the available teams, then he can choose the team,
    * and click continue, now at that point it will hit the end point and returns participant id, now using that participant-id
    * and room-id the websocket connection will be created and he is taken to the auction room , and for the creator of the room,
    * he will be having button called start auction. when auction starts the state of the room changes, so no other people can join
    * only already an existing participant can join, when he was joining his participant id was in the list then only he
    * can join.
    */


    let expiry_time = std::env::var("BID_EXPIRY").unwrap().parse::<u8>().unwrap();
    // let's read continuous messages from the client
    while let Some(Ok(message)) = receiver.next().await {
        tracing::info!("Received message: {:?}", message);
        match message {
            Message::Text(text) => {
                tracing::info!("Received text message: {}", text);

                // if a bid message was sent, then we are going to check for allowance
                if text.to_string() == "start" {
                    // we are going to return the first player from the auction
                    let player = redis_connection.get_player(1).await ;
                    let message ;
                    match player {
                       Ok(player) => {
                           message = Message::from(serde_json::to_string(&player).unwrap()) ;
                           // here we are going to add the player as Bid to the redis
                           Bid::new(0, player.id, 0.0, player.base_price) ; // no one yet bidded
                       } ,
                        Err(err) => {
                            tracing::info!("Unable to get the player-id, may be a technical Issue") ;
                            message = Message::text("Technical Glitch") ;
                        }
                    } ;
                    broadcast_handler(message,room_id.clone(),&app_state).await ;
                }else if text.to_string() == "bid" {
                    // the participant has bided
                   let result =  redis_connection.new_bid(participant_id, room_id.clone(),expiry_time).await ;
                    match result {
                       Ok(amount) => {
                           let message = Message::from(serde_json::to_string(&BidOutput{
                               bid_amount: amount,
                               team: team_name.clone()
                           }).unwrap()) ;
                           broadcast_handler(message,room_id.clone(),&app_state).await ;
                       }, Err(err) => {
                            send_himself(Message::text("Technical Issue"), participant_id, room_id.clone(), &app_state).await ;
                        }
                    } ;

                }else if text.to_string() == "end" {
                    // ending the auction

                    // check whether he was the creator of the room
                    let result = app_state.database_connection.is_room_creator(participant_id, room_id.clone()).await ;
                    match result {
                        Ok(result) => {
                            if !result {
                                send_himself(Message::text("You will not having permissions"), participant_id, room_id.clone(), &app_state).await ;
                            }
                        },
                        Err(err) => {
                            tracing::info!("getting error while is room_creator") ;
                            send_himself(Message::text("Technical Issue"), participant_id, room_id.clone(), &app_state).await ;
                        }
                    }
                    // second, check whether all the participants having least 15 players in their squad
                    let res = redis_connection.check_end_auction(room_id.clone()).await ;
                    let message ;
                    match res {
                        Ok(res) => {
                            message = Message::text("exit") ;
                        },
                        Err(err) => {
                            tracing::info!("Unable to get the room") ;
                            message = Message::text("Till all participants brought at least 15 player") ;
                        }
                    } ;
                    broadcast_handler(message,room_id.clone(),&app_state).await ;
                }else {
                    send_himself(Message::text("Invalid Message"), participant_id, room_id.clone(), &app_state).await ;
                }

            },
            Message::Close(_) => {
                tracing::info!("Client disconnected");
                // we are removing the disconnected client, such that the unbounded channel will not overload , if queue is filled with multiple disconnected message to client
                let mut value = app_state.rooms.write().await ;
                let mut index: u8 = 0 ;
                for participant in value.get(&room_id).unwrap().iter() {
                    if participant.0 == participant_id {
                        break
                    }
                    index += 1 ;
                }
                value.get_mut(&room_id).unwrap().remove(index as usize);
                drop(value) ;
                return;
            }
            Message::Binary(bytes) => todo!(),
            Message::Ping(bytes) => todo!(),
            Message::Pong(bytes) => todo!(),
        }
    }
}

pub async fn broadcast_handler(msg: Message,room_id: String, state: &AppState) { // if we want to send a message to all the participants in the room, we use broadcaster
    // over here we are going to get all the participants from the room-id
    // and send the message to all the participants
    let mut rooms = state.rooms.read().await;
    for sender in rooms.get(&room_id).unwrap().iter() {
        if let Ok(_) = sender.1.send(msg.clone()) {
            tracing::info!("Message sent to participant successfully");
        }else{
            tracing::info!("Failed to send message to participant");
        }
    }
} // lock drops over here

pub async fn send_himself(msg: Message, participant_id: i32,room_id: String, state: &AppState) {
    let mut rooms = state.rooms.read().await;
    for sender in rooms.get(&room_id).unwrap().iter() {
        if participant_id == sender.0 {
            if let Ok(_) = sender.1.send(msg.clone()) {
                tracing::info!("Message sent to participant successfully");
            }else{
                tracing::info!("Failed to send message to participant");
            }
        }
    }
}

pub async fn bid_allowance_handler(room_id: String, current_bid: f32, balance: f32, total_players_brought: u8) -> bool {
    let total_players_required: i8 = (15 - total_players_brought) as i8;
    let money_required: f32 = (total_players_required) as f32 * 0.30 ;
    if money_required <= (balance-current_bid) {
        true
    }else{
        false
    }
}