use std::sync::Arc;
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::extract::ws::{WebSocket, Message};
use axum::response::IntoResponse;
use tokio::sync::broadcast;
use crate::models::app_state::AppState;
use crate::models::auction_models::{AuctionParticipant, AuctionRoom};
use crate::services::auction_room::RedisConnection;

pub async fn ws_handler(ws: WebSocketUpgrade, Path((room_id, participant_id)): Path<(String, i32)>, State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| socket_handler(socket, room_id, participant_id, app_state))
}

async fn socket_handler(mut web_socket: WebSocket, room_id: String,participant_id: i32, app_state: Arc<AppState>) {
    tracing::info!("A new websocket connection has been established");

    // if room doesn't exist, we are going to create a broadcast channel over here
    let mut rooms = app_state.rooms.write().await;
    let (mut tx, mut rx) = tokio::sync::mpsc::unbounded_channel() ;
    let (mut sender, mut receiver) = web_socket.split() ;
    // as the sender and receiver were not clonable, we are using a channel to send messages to the clients



    // here we are going to set the redis room, if room doesn't exist we are going to create and add participant,
    // else we are going to add the participant to the room that is existing, and also checks whether the participant
    // is already exists or not
    let mut redis_connection = RedisConnection::new();
    let result = app_state.database_connection.get_team_name(participant_id).await;  // getting team name from the participant
    let team_name = match result { 
        Ok(result) => {
            result
        },Err(err ) => {
            tracing::warn!("unable to get team_selected by a participant") ;
            return;
        }
    } ;
    if let Some(vec) = rooms.get_mut(&room_id) {
        vec.push(tx) ;
        tracing::info!("Room exists, adding participant {}", participant_id);

    }else{
        tracing::info!("Room doesn't exist, creating a new room {}", room_id);
        rooms.insert(room_id.clone(), vec![tx]);
        tracing::info!("Creating room in redis") ;
        redis_connection.set_room(room_id.clone(), AuctionRoom::new()).expect("Room unable to Create");
    } // we stored the tx, which is used to send the data to the receiver channel
    drop(rooms); // release lock early

    let result = redis_connection.add_participant(room_id.clone(), participant_id.clone(),AuctionParticipant::new(
        participant_id.parse::<i32>().unwrap(),
        team_name
    )) ;

    match result {
        Ok(val) => {
            tracing::info!("participant added to the redis {}", val) ;
        }  ,
        Err(err) => {
            tracing::warn!("error in the adding participant to the redis was {}", err) ;
            return;
        }
    };



    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            sender.send(Message::text(msg)).await.unwrap(); // we are sending the message to the client
        }
    }) ;
    
    
    // till now what happened was :
    /*
    */
    
    

    // let's read continuous messages from the client
    while let Some(message) = receiver {
        tracing::info!("Received message: {:?}", message);
        match message {
            Message::Text(text) => {
                tracing::info!("Received text message: {}", text);

                // if a bid message was sent, then we are going to check for allowance


                // text was start, need to return the first player

            },
            Message::Close(_) => {
                tracing::info!("Client disconnected");
                return;
            }
        }
    }
}

async fn broadcast_handler(msg: Message, room_id: String, state: &mut AppState) { // if we want to send a message to all the participants in the room, we use broadcaster
    // over here we are going to get all the participants from the room-id
    // and send the message to all the participants
    let mut rooms = state.rooms.read().await;
    for sender in rooms.iter() {
        if let Ok(_) = sender.send(msg) {
            tracing::info!("Message sent to participant successfully");
        }else{
            tracing::info!("Failed to send message to participant");
        }
    }
} // lock drops over here

async fn bid_allowance_handler(room_id: String, participant_id: String, current_bid: f32) -> bool {
    // we will fetch from redis for the current remaining balance of the participant
    let balance:f32 = 0.0 ; // current balance of the participant
    let total_players_brought: u8 = 4 ;
    let total_players_required: i8 = (14 - total_players_brought) as i8;
    let money_required: f32 = (total_players_required) as f32 * 0.30 ;
    if money_required <= (balance-current_bid) {
        true
    }else{
        false
    }
}