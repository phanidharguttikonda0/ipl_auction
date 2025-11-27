use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use axum::body::Bytes;
use axum::Extension;
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::extract::ws::{WebSocket, Message};
use axum::response::IntoResponse;
use redis::aio::AsyncPushSender;
use tokio::sync::broadcast;
use crate::models::app_state::AppState;
use crate::models::auction_models::{AuctionParticipant, AuctionRoom, Bid, BidOutput, NewJoiner};
use crate::services::auction_room::{get_next_bid_increment, get_participant_details, RedisConnection};
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use crate::models::authentication_models::Claims;
use crate::models::bot::{each_team_desired_counts, get_each_team_user_id, Bot, BotInformation, RatingPlayer};
use crate::models::room_models::Participant;
use crate::models::webRTC_models::SignalingMessage;
use crate::services::other::get_previous_team_full_name;

pub async fn ws_handler(ws: WebSocketUpgrade, Path((room_id, participant_id)): Path<(String, i32)>, State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
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
    let mut result = app_state.database_connection.get_team_name(participant_id).await;  // getting team name from the participant
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


    if let Some(vec) = rooms.get_mut(&room_id) {
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

        if  !redis_connection.check_room_existence(room_id.clone()).await.unwrap() {
            tracing::info!("creating room in redis as it doesn't exists in redis") ;
            redis_connection.set_room(room_id.clone(), AuctionRoom::new(1)).await.expect("Room unable to Create");
        }
        let participant_exists = redis_connection.check_participant(participant_id, room_id.clone()).await ;
        let participant_exists = match participant_exists {
            Ok(participant_exists) => participant_exists,
            Err(err) => {
                tracing::error!("unable to get the check participant in redis") ;
                sender.send(Message::text("Server Side Error, Unable to create connection")).await.expect("unable to send message");
                return;
            }
        } ;
        // over here we are going to add participant to the redis
        if !participant_exists && room_status == "not_started" {
            let result = redis_connection.add_participant(room_id.clone(), AuctionParticipant::new(
                participant_id,
                team_name.clone(),
                3 // by default for each and every team having 3 rtms
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
            tracing::info!("sending the new participant to all other participants") ;
            let participant = AuctionParticipant {
                id: participant_id,
                team_name: team_name.clone(),
                balance: 100.00,
                total_players_brought: 0,
                remaining_rtms: 3,
                is_bot: false
            } ;
            broadcast_handler(Message::from(serde_json::to_string(&participant).unwrap()), room_id.clone(), &app_state).await;
            tracing::info!("new member has joined in the room {} and with team {}", room_id, team_name) ;

        }else if !participant_exists {
          tracing::info!("participant not exists and room_status was in_progress") ;
            send_himself(Message::text("Auction Started Room was close"), participant_id, room_id, &app_state).await ;
            return;
        } else{
            let Some(participant) = redis_connection.get_participant(room_id.clone(),participant_id).await.unwrap() else {
                tracing::error!("The participant was in the redis room but we are not getting the participant from the get_participant") ;
                sender.send(Message::text("Your not in the room")).await.expect("unable to send message");
                return;
            } ;
            tracing::info!("sending the new participant to all other participants") ;
            // here we are going to get the details of the old participant, and sending the old participant details
            broadcast_handler(Message::from(serde_json::to_string(&participant).unwrap()), room_id.clone(), &app_state).await;
            tracing::info!("new member has joined in the room {} and with team {}", room_id, team_name) ;

        }

    }else if room_status == "completed" {
        sender.send(Message::text("Auction was completed, Room was Closed")).await.expect("unable to send the message to the sender") ;
    }



    tracing::info!("getting all participants") ;
    // we need to send the remaining participants list over here
    let mut participants = redis_connection.get_participants(room_id.clone()).await ;
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
    {
        let participants_ = participants.clone() ;
        participants.retain(|p| hashmap.contains_key(&p.id));
        tracing::info!("we are going to send bot players as well") ;
        let room = redis_connection.get_room_details(&room_id).await.unwrap() ;
        let bots = room.bots.list_of_teams ;
        for bot_participant in bots {
            for participant in participants_.iter() {
                if participant.id == bot_participant.participant_id {
                    participants.push(participant.clone()) ;
                }// pushing bot participants
            }
        }
        send_himself(
            Message::from(serde_json::to_string(&participants).unwrap()),participant_id, room_id.clone(), &app_state
        ).await ; //> sending remaining participants their team name and participant_id
        tracing::info!("sent all active participants list to the participant") ;
    }




    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(err) = sender.send(msg).await {
                tracing::warn!("WebSocket send failed: {}", err);
                break; // stopping loop as a client disconnected
            }
        }
        tracing::info!("Message forwarding task ended");
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

                        // if a bid message was sent, then we are going to check for allowance
                        if text.to_string() == "ping" {
                            tracing::info!("a ping message in room {}", room_id) ;
                            send_himself(Message::Pong(Bytes::from_static(b"pong")), participant_id,room_id.clone(),&app_state).await ;
                            continue;
                        }else if text.to_string() == "mute" || text.to_string() == "unmute" {
                            let value = text.to_string() ;
                            tracing::info!("{} message was received", value) ;
                            let x = value + &format!("-{}",participant_id.to_string()) ; // the message will be mute-12, means participant 12 has muted himself
                            broadcast_handler(Message::text(x), room_id.clone(), &app_state).await ;
                        }else if text.to_string() == "start" {

                                    match redis_connection.set_pause_status(&room_id, false).await {
                                        Ok(_) => {
                                            tracing::info!("successfully set the status to pause") ;
                                            send_himself(Message::text("After the Current Bid Auction will be Paused"), participant_id, room_id.clone(), &app_state).await ;
                                        },
                                        Err(err) => {
                                            tracing::error!("error occured while setting the pause status") ;
                                            tracing::error!("err was {}", err) ;
                                            send_himself(Message::text("Technical Problem"), participant_id, room_id.clone(), &app_state).await ;
                                        }
                                    } ;
                            // changing room-status
                            app_state.database_connection.update_room_status(room_id.clone(), "in_progress").await.unwrap() ;
                                    let mut room = redis_connection.get_room_details(&room_id).await.unwrap() ;
                                    if room.bots.list_of_teams.len() == 0 && room.participants.len() != 10 {
                                        tracing::info!("initializing bots") ;
                                        // we need to get remaining teams
                                        tracing::info!("going to get remaining teams") ;
                                        let remaining_teams = app_state.database_connection.get_remaining_teams(room_id.clone()).await.unwrap() ;
                                        let mut list_of_teams :  Vec<BotInformation> = vec![];
                                        for team in remaining_teams {
                                            let mut bot_information = each_team_desired_counts(&team) ;
                                            // now we are going to add this participant to the redis and as well as the postgres
                                            let user_id = get_each_team_user_id(&team) ;
                                            let participant_id = app_state.database_connection.add_participant(user_id, room_id.clone(), team.clone()).await.unwrap() ;
                                            let mut auction_participant = AuctionParticipant::new(participant_id,team, 3) ;
                                            auction_participant.is_bot = true ;
                                            // now let's fill it in redis
                                            room.participants.push(auction_participant.clone()) ;
                                            bot_information.participant_id = participant_id ;
                                            list_of_teams.push(bot_information) ;
                                            broadcast_handler(Message::from(serde_json::to_string(&auction_participant).unwrap()), room_id.clone(), &app_state).await;
                                        }
                                        tracing::info!("adding list of teams to the Bot and assigning it to the room.bots") ;
                                        room.bots = Bot::new(list_of_teams) ;
                                        // now we are going to set the room
                                        redis_connection.set_room(room_id.clone(),room.clone()).await.unwrap() ;
                                    }
                                    let last_player_id = redis_connection.last_player_id(room_id.clone()).await ;
                                    tracing::info!("---------------------------------------") ;
                                    let last_player_id = match last_player_id {
                                        Ok(last_player_id) => {
                                            tracing::info!("got the last player-id as {}", last_player_id) ;
                                            last_player_id
                                        },
                                        Err(err) => {
                                            tracing::error!("error while getting last player-id was {}", err) ;
                                            1
                                        }
                                    } ;
                                    // we are going to return the first player from the auction
                                    let player = redis_connection.get_player(last_player_id).await ;
                                    let mut message ;
                                    match player {
                                        Ok(player) => {

                                            // here we are going to add the player as Bid to the redis
                                            let mut bid = Bid::new(0, player.id, 0.0, player.base_price, false, false) ; // no one yet bidded
                                            // here we are going to send the player along with bots bid
                                            let result = room.bots.decide_bid(&RatingPlayer {
                                                role: player.role.clone(),
                                                rating: player.player_rating
                                            }, player.base_price, HashSet::new()) ;
                                            room.skip_count = result.2 ;
                                            room.current_player = Some(player.clone()) ;
                                            if result.0 != "None" {
                                                bid.bid_amount = player.base_price ;
                                                bid.participant_id = result.1 ;
                                            }
                                            redis_connection.set_room(room_id.clone(), room).await.unwrap() ;
                                            redis_connection.update_current_bid(room_id.clone(),bid, expiry_time).await.expect("unable to update the bid") ;
                                            message = Message::from(serde_json::to_string(&player).unwrap()) ;
                                            broadcast_handler(message,room_id.clone(),&app_state).await ;
                                            if result.0 != "None" {
                                                tracing::info!("bot has been bided") ;
                                                message = Message::from(serde_json::to_string(&BidOutput{
                                                    bid_amount: player.base_price,
                                                    team: result.0
                                                }).unwrap()) ;
                                                broadcast_handler(message,room_id.clone(),&app_state).await ;
                                            }
                                        } ,
                                        Err(err) => {
                                            tracing::info!("Unable to get the player-id, may be a technical Issue") ;
                                            tracing::error!("{}", err) ;
                                            message = Message::text("Technical Glitch") ;
                                            broadcast_handler(message,room_id.clone(),&app_state).await ;
                                        }
                                    } ;



                        }else if text.to_string() == "bid" {
                            /*
                                If previously the same participant has send the bid, then that shouldn't be considered

                            */
                            // if this key exists in the redis then only bids takes place
                            if !redis_connection.check_key_exists(&timer_key).await.unwrap() {
                                tracing::info!("as the key doesn't exists we are not going to take this bid") ;
                                send_himself(Message::text("Bid is Invalid, RTM is taking place"), participant_id, room_id.clone(), &app_state).await ;
                            }else {
                                let mut room = redis_connection.get_room_details(&room_id).await.unwrap() ;
                                if room.current_bid.clone().unwrap().participant_id == participant_id {
                                    tracing::info!("already the highest bidder") ;
                                    send_himself(Message::text("You are already the highest bidder"), participant_id,room_id.clone(),&app_state).await ;
                                    continue;
                                }
                                if room.skip_count.contains(&participant_id) {
                                    tracing::info!("skipped the player, the bid is not valid any more") ;
                                    send_himself(Message::text("Bid is Invalid, you skipped the player"), participant_id, room_id.clone(), &app_state).await ;
                                }else{
                                    // before proceeding with the participant, id let's see bot bids or not
                                    tracing::info!("here we are -----------------> in bid logic") ;
                                    let bot = room.bots.clone() ;
                                    let mut bid_amount =
                                        redis_connection.new_bid(participant_id, room_id.clone(),expiry_time).await.expect("new bid unwrap failed") ;
                                    let mut message = Message::from(serde_json::to_string(&BidOutput{
                                        bid_amount,
                                        team: team_name.clone()
                                    }).unwrap()) ;
                                    broadcast_handler(message,room_id.clone(),&app_state).await ;
                                    tracing::info!("here we are -----------------> broadcasted the user bid now") ;
                                    if bot.list_of_teams.len() != 0 {
                                        let mut bid = room.current_bid.clone().unwrap() ;

                                        let future_bid = get_next_bid_increment(bid_amount, &bid) ;
                                        let future_bid = future_bid + bid_amount ;
                                        tracing::info!("future bid was {}", future_bid) ;
                                        // we are taking 2 future bids because
                                        let current_player = room.current_player.clone().unwrap() ;
                                        tracing::info!("deciding the bid by bots") ;
                                        let res = bot.decide_bid(&RatingPlayer {
                                            role: current_player.role.clone(),
                                            rating: current_player.player_rating,
                                        }, bid.bid_amount, room.skip_count) ;
                                        room.skip_count = res.2 ;
                                        redis_connection.set_room(room_id.clone(), room).await.unwrap() ;
                                        tracing::info!("The result of the decide_bot was team_name {} , participant_id {}", res.0, res.1) ;
                                        if res.0 != "None" {
                                            // now the new bid was by bot
                                            tracing::info!("Bid accepted by the Bot") ;
                                            let amount =  redis_connection.new_bid(res.1, room_id.clone(),expiry_time).await.expect("new bid unwrap failed") ;
                                            message = Message::from(serde_json::to_string(&BidOutput{
                                                bid_amount: amount,
                                                team: res.0
                                            }).unwrap()) ;
                                            broadcast_handler(message,room_id.clone(),&app_state).await ;
                                        }
                                    }

                                }
                            }

                        } else if text.to_string() == "end" {
                            // ending the auction
                            // when we click on end we are getting only exit as the message without any reason
                            // check whether he was the creator of the room

                            let result = app_state.database_connection.is_room_creator(participant_id, room_id.clone()).await ;
                            match result {
                                Ok(result) => {
                                    if !result {
                                        send_himself(Message::text("Only Creator can have permission"), participant_id, room_id.clone(), &app_state).await ;
                                    }else{
                                        // second, check whether all the participants having least 15 players in their squad
                                        let res = redis_connection.check_end_auction(room_id.clone()).await ;
                                        let message ;
                                        match res {
                                            Ok(res) => {
                                                if res {
                                                    // when front-end has disconnected automatically it's going to be the end.
                                                    // we are going to change the state of the auction to completed such that this room get's invalid
                                                    match app_state.database_connection.update_room_status(room_id.clone(), "completed").await {
                                                        Ok(result) => {
                                                            tracing::info!("room status changed to completed") ;
                                                            // here we are going to remove the data from redis
                                                            match redis_connection.atomic_delete(&room_id).await {
                                                                Ok(_) => {
                                                                    tracing::info!("successfully removed the room from redis") ;
                                                                    message = Message::text("exit") ; // in front-end when this message was executed then it must stop the ws connection with server
                                                                },
                                                                Err(err) => {
                                                                    tracing::info!("got error while removing the room from redis") ;
                                                                    tracing::error!("{}", err) ;
                                                                    message = Message::text("Technical Issue")
                                                                }
                                                            }
                                                        },
                                                        Err(err) => {
                                                            tracing::info!("unable to update the room status to completed") ;
                                                            tracing::error!("{}",err) ;
                                                            message = Message::text("Technical Issue") ;
                                                        }
                                                    }

                                                }else {
                                                    message = Message::text("Not enough players brought by each team") ;
                                                }
                                            },
                                            Err(err) => {
                                                tracing::info!("Unable to get the room") ;
                                                message = Message::text("Till all participants brought at least 15 player") ;
                                            }
                                        } ;
                                        broadcast_handler(message,room_id.clone(),&app_state).await ;
                                    }

                                },
                                Err(err) => {
                                    tracing::info!("getting error while is room_creator") ;
                                    send_himself(Message::text("Technical Issue"), participant_id, room_id.clone(), &app_state).await ;
                                }
                            }

                        }else if text.to_string() == "pause" {

                            // we are going to pause the auction, such that when clicked create again, going to start from the last player
                            let result = app_state.database_connection.is_room_creator(participant_id, room_id.clone()).await ;
                            match result {
                                Ok(result) => {
                                    if result {
                                        // we are going to pause auction after the current bid
                                        match redis_connection.set_pause_status(&room_id, true).await {
                                            Ok(_) => {
                                                tracing::info!("sucessfully set the status to pause") ;
                                                send_himself(Message::text("After the Current Bid Auction will be Paused"), participant_id, room_id.clone(), &app_state).await ;
                                            },
                                            Err(err) => {
                                                tracing::error!("error occured while setting the pause status") ;
                                                tracing::error!("err was {}", err) ;
                                                send_himself(Message::text("Technical Problem"), participant_id, room_id.clone(), &app_state).await ;
                                            }
                                        } ;
                                    }else {
                                        send_himself(Message::text("Only Creator can have permission"), participant_id, room_id.clone(), &app_state).await ;
                                    }
                                },
                                Err(err) => {
                                    tracing::error!("got error while check is the creator") ;
                                    tracing::error!("{}", err) ;
                                }
                            };

                        }else if text.to_string() == "rtm-accept" { // need to check RTM , why even timer was there it was failing and also need to check whether the RTM timer was the expiry time
                            // can only be called, if the key was rtms
                            if redis_connection.check_key_exists(&rtm_timer_key).await.unwrap() {
                                tracing::info!("rtm was being accepted") ;
                                redis_connection.atomic_delete(&rtm_timer_key).await.unwrap();
                                // accepting the bid
                                let room = redis_connection.get_room_details(&room_id).await.unwrap() ;
                                let bid = room.current_bid.unwrap() ;
                                let bid = Bid::new(participant_id, bid.player_id, bid.bid_amount, bid.base_price, false, true) ;
                                // adding the bid to the redis
                                redis_connection.update_current_bid(room_id.clone(), bid, 1).await.unwrap() ;
                            }else {
                                send_message_to_participant(participant_id, String::from("Invalid RTM was not taken place"), room_id.clone(), &app_state).await ;
                            }
                        }else if text.to_string().contains("rtm-cancel") {
                            tracing::info!("cancelling the offer by the highest bidder") ;
                            redis_connection.atomic_delete(&rtm_timer_key).await.unwrap() ;
                            // now we are going to send the same bid with expiry 0
                            let room = redis_connection.get_room_details(&room_id).await.unwrap() ;
                            let mut current_bid = room.current_bid.unwrap() ;
                            current_bid.is_rtm = true ;  // where the last bided person is the person who used rtm, so we need to keep it as rtm only, such that his rtms will decreased
                            redis_connection.update_current_bid(room_id.clone(), current_bid, 1).await.unwrap() ;
                            send_message_to_participant(participant_id, String::from("Cancelled the RTM Price"), room_id.clone(), &app_state).await ;
                        }
                        else if text.to_string().contains("rtm") {
                            tracing::info!("rtm was accepted with the following {}",text.to_string()) ;
                            // we need to check
                            // if this key exists in the redis then no bids takes place
                            if redis_connection.check_key_exists(&rtm_timer_key).await.unwrap() { // if normal bids were not taking place on in that scenario
                                redis_connection.atomic_delete(&rtm_timer_key).await.unwrap();
                                // rtm-amount eg : rtm-5.00 means increasing 5.00cr from the current price
                                let mut room = redis_connection.get_room_details(&room_id).await.unwrap() ;
                                let mut bid = room.current_bid.unwrap() ;
                                let amount = text.to_string().split("-").collect::<Vec<&str>>()[1].parse::<f32>().unwrap() ;

                                // now we are going to check whether the specific participant, has the authority to use the rtm, means the current player
                                // previous team should be the participant playing team
                                let rtm_placer_participant = get_participant_details(participant_id, &room.participants).unwrap() ;
                                let previous_player = redis_connection.get_player(bid.player_id).await.unwrap() ;
                                let full_team_name = get_previous_team_full_name(&previous_player.previous_team);
                                let current_participant_team = rtm_placer_participant.0.team_name ;

                                if full_team_name == current_participant_team {
                                    let new_amount = amount + bid.bid_amount ;
                                    // here we need to check whether the rtm placer having that much enough money and as well the same other guy having that much enough money
                                    if rtm_placer_participant.0.remaining_rtms > 0 {
                                        let highest_bidder_participant = get_participant_details(bid.participant_id, &room.participants).unwrap() ;
                                        let rtm_placer_participant_bid_allowance = bid_allowance_handler(room_id.clone(),new_amount, rtm_placer_participant.0.balance, rtm_placer_participant.0.total_players_brought).await ;
                                        let highest_bidder_participant_allowance = bid_allowance_handler(room_id.clone(),new_amount, highest_bidder_participant.0.balance, highest_bidder_participant.0.total_players_brought).await ;
                                        if rtm_placer_participant_bid_allowance && highest_bidder_participant_allowance {
                                            tracing::info!("both having money") ;
                                            let mut bot_not_approved = true ;
                                            if room.bots.clone().is_bot_participant(bid.participant_id) {
                                                for x in room.bots.list_of_teams.clone().iter() {
                                                    if x.participant_id != bid.participant_id {
                                                        room.skip_count.insert(x.participant_id) ;
                                                    }
                                                } // now except the highest bidder every one will be skipped
                                                let result = room.bots.decide_bid(&RatingPlayer {
                                                    role: room.current_player.clone().unwrap().role.clone(),
                                                    rating: room.current_player.clone().unwrap().player_rating,
                                                }, new_amount, room.skip_count) ;
                                                if result.0 != "None" {
                                                    tracing::info!("the bot has decided to accept the RTM amount") ;
                                                    let bid_ = Bid::new(bid.participant_id, bid.player_id, new_amount, bid.base_price, true, false) ;
                                                    // adding the bid to the redis
                                                    let _ = redis_connection.update_current_bid(room_id.clone(), bid_, 1).await.unwrap() ;
                                                    bot_not_approved = false ;
                                                }
                                            }

                                            if bot_not_approved {
                                                tracing::info!("bot not approved the bid") ;
                                                // creating the new Bid
                                                let bid_ = Bid::new(participant_id, bid.player_id, new_amount, bid.base_price, true, false) ;
                                                // adding the bid to the redis
                                                let _ = redis_connection.update_current_bid(room_id.clone(), bid_, expiry_time).await.unwrap() ;
                                                send_message_to_participant(bid.participant_id, format!("rtm-amount-{}", new_amount), room_id.clone(), &app_state).await ;
                                                continue;
                                            }

                                        }else if rtm_placer_participant_bid_allowance {
                                            tracing::info!("rtm bidder has enough money, so bid goes to him") ;
                                            // delete the key and add the new bid with expiry 0 seconds

                                            // new bid
                                            redis_connection.update_current_bid(room_id.clone(), Bid::new(participant_id, bid.player_id, new_amount, bid.base_price, true, false),1).await.unwrap() ;
                                            // send to the highest bidder the reason
                                            send_message_to_participant(bid.participant_id, format!("no balance to accept the bid price of {}",new_amount), room_id.clone(), &app_state).await ;
                                            continue;
                                        }else {
                                            tracing::info!("only the person having the rtm having the enough money") ;
                                            send_himself(Message::text("Invalid Price, You Lost RTM for this Bid"), participant_id, room_id.clone(), &app_state).await ;
                                        }
                                    }else{
                                        send_himself(Message::text("All RTMS were used"), participant_id, room_id.clone(), &app_state).await ;
                                    }
                                }else {
                                    send_himself(Message::text("The current player is not in ur team previously"), participant_id, room_id.clone(), &app_state).await ;
                                }
                                bid.rtm_bid = true ; // it will be rtm_bid , but for remaining rtms will be same, only thing is in subscriber making sure no infinite loop takes place, where we are going to inifinetly if there previous
                                let _ = redis_connection.update_current_bid(room_id.clone(), bid, 1).await.unwrap() ;
                            }else {
                                tracing::info!("Now no RTM bids were taking place") ;
                                send_himself(Message::text("No RTM Bids are taking place"), participant_id, room_id.clone(), &app_state).await ;
                            }

                        }else if text.to_string() == "skip" {
                            tracing::info!("message skip was received") ;
                            // we need to add a state in redis
                            let mut room = redis_connection.get_room_details(&room_id).await.unwrap() ;

                                if !room.skip_count.contains(&participant_id) {
                                    room.skip_count.insert(participant_id) ;
                                    let mut message ;
                                    tracing::info!("{} are equal {}", room.skip_count.len(),room.participants.len()) ;
                                    redis_connection.set_room(room_id.clone(), room.clone()).await.unwrap() ;
                                    if room.skip_count.len() == room.participants.len() {
                                        tracing::info!("All teams Skipped the Player") ;
                                        message = String::from("All teams skipped") ;
                                        // we are going to delete the current timer,  before that we are going to check timers exists or not
                                        if redis_connection.check_key_exists(&timer_key).await.unwrap() {
                                            tracing::info!("key exists we are going to delete the key") ;
                                            redis_connection.atomic_delete(&timer_key).await.expect("unable to delete the room inside skip");
                                            let current_bid = room.current_bid.unwrap() ;
                                            if current_bid.bid_amount == 0.0 {
                                                message = "Player going tobe Unsold".to_string() ;
                                            }else {
                                                message = "Player going to sold to highest bidder".to_string() ;
                                            }
                                            // giving to the highest bidder
                                            redis_connection.update_current_bid(room_id.clone(), current_bid,1).await.unwrap() ;
                                        }else {
                                            message = "At this Stage Skip won't work".to_string();
                                        }
                                    }else {
                                        message = format!("till now {} skipped the player",room.skip_count.len()) ;
                                    }
                                    broadcast_handler(Message::text(&message),room_id.clone(),&app_state).await ;
                                }else{
                                    tracing::info!("already you skipped the player") ;
                                    send_himself(Message::text("you Already Skipped"), participant_id, room_id.clone(), &app_state).await ;
                                }

                            /*
                                if a participant skips, then, that participant cannot be bid again
                            */

                        }else {
                            let message ;
                            let to_participant ;
                            tracing::info!("******************* Message for WeB RTC was ***************************") ;
                            tracing::info!("here is the front-end passed message {}", text.to_string()) ;
                            tracing::info!("****************************** *****************************************") ;
                            if !(text.to_string().starts_with('{') && text.to_string().ends_with('}')) {
                                tracing::warn!("Ignoring non-JSON message: {}", text);
                                continue;
                            }
                            let parsed: SignalingMessage = match serde_json::from_str(&text) {
                                Ok(msg) => msg,
                                Err(err) => {
                                    eprintln!("Failed to parse signaling message: {}", err);
                                    send_himself(Message::text("Unable to parse what you have sent"), participant_id, room_id.clone(), &app_state).await ;
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

                            send_himself(message, to_participant, room_id.clone(), &app_state).await ;

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

    for participant in value.get(room_id).unwrap().iter() {
        if participant.0 == participant_id {
            break
        }
        index += 1 ;
    }

    /*
        need to broadcast which participant has been disconnected, and when joins we are any way sending the message
    */

    value.get_mut(room_id).unwrap().remove(index as usize);
    drop(value) ;
    broadcast_handler(Message::from(serde_json::to_string(&Participant { participant_id, team_name }).unwrap()), room_id.to_string(), &app_state).await ;
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

pub async fn send_message_to_participant(participant_id: i32, message: String, room_id: String, state: &AppState) {
    let mut rooms = state.rooms.read().await;
    for sender in rooms.get(&room_id).unwrap().iter() {
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

/*

now implement new logic in the AuctionRoom , if the message sent by the back-end was "Use RTM",
then ask the user you want to use RTM, then if he says yes , then give him an input box and ask
him to how much do you want to add to the current bid amount , what ever amount he enters, send that
 amount via web socket as message , "rtm-amount" eg: "rtm-10.00" , and then when ever the user get's
  the following message "rtm-amount-{}" example : "rtm-amount-25.00" , then ask him whether you want
  to accept 25.00 cr , if they click on accept then send ws message as rtm-accept, else rtm-cancel.
  this is the new logic need to be implemented now.

*/