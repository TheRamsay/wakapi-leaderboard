use std::fmt::format;
use serde::{Deserialize, Serialize};

use regex::Regex;

fn main() {
    let leaderboard = scrape_leaderboard();
    print_leaderboard(&leaderboard);
}

// Names 
// <strong class="text-ellipsis truncate">@(\w+)<\/strong>

#[derive(Debug, Serialize, Deserialize)]
struct UserInfo {
    username: String,
    total_seconds: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserPayload {
    data: UserInfo
}

fn scrape_leaderboard() -> Vec<UserInfo> {
    let mut leaderboard: Vec<UserInfo> = Vec::new();

    let response = reqwest::blocking::get("https://wakapi.krejzac.cz/leaderboard").unwrap().text().unwrap();
    let re = Regex::new(r#"<strong class="text-ellipsis truncate">@(\w+)</strong>"#).unwrap();

    for cap in re.captures_iter(&response) {
        println!("Fetching info for {:?}...", &cap[1]);
        if let Some(user_info) = get_user_stats(&cap[1]) {
            leaderboard.push(user_info);
        }
    }
    
    leaderboard.sort_by_key(|user| -user.total_seconds);
    leaderboard
}

fn get_user_stats(username: &str) -> Option<UserInfo> {
    let api_url = format!("https://wakapi.krejzac.cz/api/compat/wakatime/v1/users/{}/stats/month", username);
    let response = reqwest::blocking::get(api_url).unwrap();

    return match response.json::<UserPayload>() {
        Ok(val) =>  Some(val.data),
        Err(_) => None
    }
}

fn print_leaderboard(leaderboard: &Vec<UserInfo>) {
    for (i, user_info) in leaderboard.iter().enumerate() {
        println!("{}) {} - {:.2} hours", i + 1, user_info.username, user_info.total_seconds  as f64 / (60 * 60) as f64);
    }
}
