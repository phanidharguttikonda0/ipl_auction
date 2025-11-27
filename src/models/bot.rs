use std::collections::HashSet;
use rand::Rng;
use redis_derive::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};
use crate::auction::bid_allowance_handler;

#[derive(Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize, Default)]
pub struct RolePrefs {
    pub batsman: f32,
    pub bowler: f32,
    pub all_rounder: f32,
}

#[derive(Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize)]
pub struct BotInformation {
    pub team_name: String,
    pub participant_id: i32,
    pub aggressiveness: f32,
    pub risk_taking: f32,
    pub budget_total: f32,
    pub budget_left: f32,
    pub star_player_cap: f32,
    pub bargain_threshold: f32,
    pub acquired_count: RoleCounts,
    pub desired_count: RoleCounts, // every team based on their home pitch should have a desired count of each role
    pub role_prefs: RolePrefs,
}

pub struct RatingPlayer {
    pub role: String, // batsman, bowler, all_rounder, wicketkeeper
    pub rating: i32, // 0 to 100
}

#[derive(Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize, Default)]
pub struct RoleCounts {
    pub batsman: i32,
    pub bowler: i32,
    pub all_rounder: i32,
}

pub fn each_team_desired_counts(team_name: &str) -> BotInformation {

    match team_name {
        "Mumbai Indians" => BotInformation {
            team_name: String::from("Mumbai Indians"),
            participant_id: 0,
            aggressiveness: 0.8,
            risk_taking: 0.6,
            budget_total: 100.00,
            budget_left: 100.00,
            star_player_cap: 0.40,
            bargain_threshold: 0.7,
            acquired_count: RoleCounts::default(),
            desired_count: RoleCounts {
                batsman: 5,
                bowler: 6,
                all_rounder: 4,
            },
            role_prefs: RolePrefs {
                batsman: 0.40,
                bowler: 0.40,
                all_rounder: 0.20,
            },
        },

        "Chennai Super Kings" => BotInformation {
            team_name: String::from("Chennai Super Kings"),
            participant_id: 0,
            aggressiveness: 0.6,
            risk_taking: 0.4,
            budget_total: 100.00,
            budget_left: 100.00,
            star_player_cap: 0.35,
            bargain_threshold: 0.75,
            acquired_count: RoleCounts::default(),
            desired_count: RoleCounts {
                batsman: 6,
                bowler: 6,
                all_rounder: 3,
            },
            role_prefs: RolePrefs {
                batsman: 0.40,
                bowler: 0.40,
                all_rounder: 0.20,
            },
        },

        "Royal Challengers Bangalore" => BotInformation {
            team_name: String::from("Royal Challengers Bangalore"),
            participant_id: 0,
            aggressiveness: 0.7,
            risk_taking: 0.75,
            budget_total: 100.00,
            budget_left: 100.00,
            star_player_cap: 0.75,
            bargain_threshold: 0.6,
            acquired_count: RoleCounts::default(),
            desired_count: RoleCounts {
                batsman: 6,
                bowler: 6,
                all_rounder: 3,
            },
            role_prefs: RolePrefs {
                batsman: 0.45,
                bowler: 0.40,
                all_rounder: 0.15,
            },
        },

        "Sun Risers Hyderabad" => BotInformation {
            team_name: String::from("Sun Risers Hyderabad"),
            participant_id: 0,
            aggressiveness: 0.7,
            risk_taking: 0.58,
            budget_total: 100.00,
            budget_left: 100.00,
            star_player_cap: 0.40,
            bargain_threshold: 0.7,
            acquired_count: RoleCounts::default(),
            desired_count: RoleCounts {
                batsman: 7,
                bowler: 6,
                all_rounder: 2,
            },
            role_prefs: RolePrefs {
                batsman: 0.55,
                bowler: 0.35,
                all_rounder: 0.10,
            },
        },

        "Delhi Capitals" => BotInformation {
            team_name: String::from("Delhi Capitals"),
            participant_id: 0,
            aggressiveness: 0.8,
            risk_taking: 0.55,
            budget_total: 100.00,
            budget_left: 100.00,
            star_player_cap: 0.80,
            bargain_threshold: 0.65,
            acquired_count: RoleCounts::default(),
            desired_count: RoleCounts {
                batsman: 5,
                bowler: 6,
                all_rounder: 4,
            },
            role_prefs: RolePrefs {
                batsman: 0.30,
                bowler: 0.40,
                all_rounder: 0.30,
            },
        },

        "Kolkata Knight Riders" => BotInformation {
            team_name: String::from("Kolkata Knight Riders"),
            participant_id: 0,
            aggressiveness: 0.55,
            risk_taking: 0.5,
            budget_total: 100.00,
            budget_left: 100.00,
            star_player_cap: 0.40,
            bargain_threshold: 0.65,
            acquired_count: RoleCounts::default(),
            desired_count: RoleCounts {
                batsman: 4,
                bowler: 6,
                all_rounder: 5,
            },
            role_prefs: RolePrefs {
                batsman: 0.25,
                bowler: 0.35,
                all_rounder: 0.40,
            },
        },

        "Lucknow Super Gaints" => BotInformation {
            team_name: String::from("Lucknow Super Gaints"),
            participant_id: 0,
            aggressiveness: 0.5,
            risk_taking: 0.3,
            budget_total: 100.00,
            budget_left: 100.00,
            star_player_cap: 0.45,
            bargain_threshold: 0.80,
            acquired_count: RoleCounts::default(),
            desired_count: RoleCounts {
                batsman: 7,
                bowler: 5,
                all_rounder: 3,
            },
            role_prefs: RolePrefs {
                batsman: 0.55,
                bowler: 0.25,
                all_rounder: 0.20,
            },
        },

        "Punjab Kings" => BotInformation {
            team_name: String::from("Punjab Kings"),
            participant_id: 0,
            aggressiveness: 0.85,
            risk_taking: 0.8,
            budget_total: 100.00,
            budget_left: 100.00,
            star_player_cap: 0.70,
            bargain_threshold: 0.60,
            acquired_count: RoleCounts::default(),
            desired_count: RoleCounts {
                batsman: 5,
                bowler: 4,
                all_rounder: 6,
            },
            role_prefs: RolePrefs {
                batsman: 0.35,
                bowler: 0.20,
                all_rounder: 0.45,
            },
        },

        "Gujarat Titans" => BotInformation {
            team_name: String::from("Gujarat Titans"),
            participant_id: 0,
            aggressiveness: 0.6,
            risk_taking: 0.35,
            budget_total: 100.00,
            budget_left: 100.00,
            star_player_cap: 0.35,
            bargain_threshold: 0.75,
            acquired_count: RoleCounts::default(),
            desired_count: RoleCounts {
                batsman: 6,
                bowler: 6,
                all_rounder: 3,
            },
            role_prefs: RolePrefs {
                batsman: 0.35,
                bowler: 0.30,
                all_rounder: 0.25,
            },
        },

        "Rajasthan Royals" => BotInformation {
            team_name: String::from("Rajasthan Royals"),
            participant_id: 0,
            aggressiveness: 0.55,
            risk_taking: 0.35,
            budget_total: 100.00,
            budget_left: 100.00,
            star_player_cap: 0.30,
            bargain_threshold: 0.80,
            acquired_count: RoleCounts::default(),
            desired_count: RoleCounts {
                batsman: 7,
                bowler: 5,
                all_rounder: 3,
            },
            role_prefs: RolePrefs {
                batsman: 0.60,
                bowler: 0.25,
                all_rounder: 0.15,
            },
        },

        _ => panic!("Unknown team"),
    }
}


#[derive(Default,Debug,Clone, FromRedisValue, ToRedisArgs, Serialize, Deserialize)]
pub struct Bot {
    pub list_of_teams: Vec<BotInformation>, // all these are bot teams playing the auction
}

impl Bot {
    pub fn new(list_of_teams: Vec<BotInformation>) -> Self {
        Bot { list_of_teams } // while passing list_of_teams to here the participant_ids are being updated beforehand
    }

    // current_bid over here means, the bid that is going to be placed if any bot has be accepted
    pub fn decide_bid(&self, current_player: &RatingPlayer, current_bid: f32, mut skip_count: HashSet<i32>) -> (String, i32, HashSet<i32>) {

        // now here write the logic and return which team bot was going to accept the current_bid and needs to return
        // the team name and the participant_id of the team as the return type
        // use the skip count before to not include the particular teams with the participant-id, because they
        // said not interested in previous bids and also whom said they are not intrested those bots bids also add to the skip_count

        let mut rng = rand::thread_rng();
        let mut best_team: Option<(String, i32)> = None;
        let mut best_score: f32 = 0.0;

        for team in &self.list_of_teams {
            let pid = team.participant_id;

            // 1. Skip if this team already declined
            if skip_count.contains(&pid) {
                continue;
            }

            // 2. Budget safety (your rule)
            let players_bought = team.acquired_count.batsman
                + team.acquired_count.bowler
                + team.acquired_count.all_rounder;

            let total_players_bought = players_bought as u8;
            let total_required = 15 - total_players_bought;

            let money_required = (total_required as f32) * 0.30;

            // we are not using the bid_allowance_handler here because, the bot any buy min of 15 players
            if money_required > (team.budget_left - current_bid) {
                skip_count.insert(pid);
                continue;
            }

            // 3. Star player rule (rating >= 80)
            if current_player.rating >= 95 {
                let max_star_cost = ((team.budget_total * team.star_player_cap)/3.0).round() / 100.0;
                if current_bid > max_star_cost {
                    skip_count.insert(pid);
                    continue;
                }
            }else if current_player.rating >= 90 {
                let max_star_cost = ((team.budget_total * team.star_player_cap)/4.0).round() / 100.0;
                if current_bid > max_star_cost {
                    skip_count.insert(pid);
                    continue;
                }
            }else if current_player.rating >= 85 {
                let max_star_cost = ((team.budget_total * team.star_player_cap)/5.0).round() / 100.0;
                if current_bid > max_star_cost {
                    skip_count.insert(pid);
                }
            }else if current_player.rating >= 80 {
                let max_star_cost = ((team.budget_total * team.star_player_cap)/6.25).round() / 100.0;
                if current_bid > max_star_cost {
                    skip_count.insert(pid);
                }
            }



            // 4. Determine role needs
            let (desired, acquired, role_pref) = match current_player.role.as_str() {
                "batsman" => (team.desired_count.batsman, team.acquired_count.batsman, team.role_prefs.batsman),
                "bowler" => (team.desired_count.bowler, team.acquired_count.bowler, team.role_prefs.bowler),
                "all_rounder" => (team.desired_count.all_rounder, team.acquired_count.all_rounder, team.role_prefs.all_rounder),
                _ => continue,
            };

            let need_for_role = if acquired < desired { 1.0 } else { 0.2 };

            // 5. Player rating score
            let rating_score: f32 = (current_player.rating as f32 / 100.0 * 100.0).round() / 100.0 ; // rounding 2 decimals


            // 6. Base bid score
            let bid_score =
                (role_pref * 0.30) +
                    (rating_score * 0.40) +
                    (team.aggressiveness * 0.20) +
                    (need_for_role * 0.10);

            // 7. Add chaos
            let random_factor = rng.gen_range(0.0..1.0);
            let final_score = (bid_score * 0.7) + (random_factor * team.risk_taking * 0.3);

            // 8. Decide
            if final_score > team.bargain_threshold {
                if final_score > best_score {
                    best_score = final_score;
                    best_team = Some((team.team_name.clone(), pid));
                }
            } else {
                skip_count.insert(pid);
            }
        }

        if let Some((team_name, pid)) = best_team {
            return (team_name, pid, skip_count);
        }

        ("None".into(), -1, skip_count)
    }


    // we need to update the acquired_count and budget left of the bot teams
    pub fn update_acquired_count(&mut self, participant_id: i32, role: &str) {
        for team in &mut self.list_of_teams {
            if team.participant_id == participant_id {
                match role {
                    "batsman" => team.acquired_count.batsman += 1,
                    "bowler" => team.acquired_count.bowler += 1,
                    "all_rounder" => team.acquired_count.all_rounder += 1,
                    _ => continue,
                }
                break
            }
        }
    }

    pub fn update_budget_left(&mut self, participant_id: i32, bid_amount: f32) {
        for team in &mut self.list_of_teams {
            if team.participant_id == participant_id {
                team.budget_left -= bid_amount;
            }
        }
    }
    
    pub fn is_bot_participant(&self, participant_id: i32) -> bool {
        self.list_of_teams.iter().any(|team| team.participant_id == participant_id)
    }

    pub fn get_team_name(&self, participant_id: i32) -> Option<String> {
        self.list_of_teams.iter().find(|team| team.participant_id == participant_id).map(|team| team.team_name.clone())
    }
}


pub fn get_each_team_user_id(team_name: &str) -> i32 {
    let is_prod = std::env::var("PROD").unwrap().parse::<bool>().unwrap();
    match team_name {
        "Mumbai Indians" => {
            if is_prod {
                74
            }else {
                7
            }
        },
        "Chennai Super Kings" => {
            if is_prod {
                75
            }else {
                8
            }
        },
        "Royal Challengers Bangalore" => {
            if is_prod {
                79
            }else {
                12
            }
        },
        "Sun Risers Hyderabad" => {
            if is_prod {
                76
            }else {
                9
            }
        },
        "Delhi Capitals" => {
            if is_prod {
                81
            }else {
                14
            }
        },
        "Kolkata Knight Riders" => {
            if is_prod {
                80
            }else {
                13
            }
        },
        "Lucknow Super Gaints" => {
            if is_prod {
                82
            }else {
                15
            }
        },
        "Punjab Kings" => {
            if is_prod {
                77
            }else {
                10
            }
        },
        "Rajasthan Royals" => {
            if is_prod {
                78
            }else {
                11
            }
        },
        "Gujarat Titans" => {
            if is_prod {
                83
            }else {
                16
            }
        },

        _ => panic!("Unknown team"),
    }
}