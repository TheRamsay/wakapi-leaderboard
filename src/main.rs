use std::fmt::format;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{StandardFramework, CommandResult};
use regex::Regex;
use redis::Commands;

#[derive(Debug, Serialize, Deserialize)]
struct UserInfo {
    username: String,
    total_seconds: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserPayload {
    data: UserInfo
}

async fn scrape_leaderboard() -> Vec<UserInfo> {
    let mut leaderboard: Vec<UserInfo> = Vec::new();

    let client = redis::Client::open("redis://default:qJI0nbg5dpzEt5jrr2R1@containers-us-west-179.railway.app:7784").unwrap();
    let mut con = client.get_connection().expect("Coooo");

    if let Ok(value) = con.get::<&str, String>("members") {
        for username in value.split(":") {
            if let Some(user_info) = get_user_stats(username).await {
                leaderboard.push(user_info);
            }
        }

        leaderboard.sort_by_key(|user| -user.total_seconds);
        leaderboard
    } else {
        let mut leaderboard_string = String::new();
        let response = reqwest::get("https://wakapi.krejzac.cz/leaderboard").await.unwrap().text().await.unwrap();
        let re = Regex::new(r#"<strong class="text-ellipsis truncate">@(\w+)</strong>"#).unwrap();

        for cap in re.captures_iter(&response) {
            if let Some(user_info) = get_user_stats(&cap[1]).await {
                leaderboard.push(user_info);
                leaderboard_string.push_str(format!("{}:", &cap[1]).as_str());
            }
        }
        
        con.set_ex::<&str, String, String>("members", leaderboard_string, 60*60 * 6).unwrap();
        leaderboard.sort_by_key(|user| -user.total_seconds);
        leaderboard
    }
}

async fn get_user_stats(username: &str) -> Option<UserInfo> {
    let client = redis::Client::open("redis://default:qJI0nbg5dpzEt5jrr2R1@containers-us-west-179.railway.app:7784").unwrap();
    let mut con = client.get_connection().expect("Coooo");

    if let Ok(value) = con.get::<&str, i32>(username) {
        Some(UserInfo { username: username.to_string(), total_seconds: value })
    } else {
        let api_url = format!("https://wakapi.krejzac.cz/api/compat/wakatime/v1/users/{}/stats/month", username);
        let response = reqwest::get(api_url).await.unwrap();

        return match response.json::<UserPayload>().await {
            Ok(val) =>  {
                con.set_ex::<String, i32, String>(String::from(&val.data.username), val.data.total_seconds, 60 * 15).unwrap();
                Some(UserInfo { username: String::from(val.data.username), total_seconds: val.data.total_seconds})
            },
            Err(_) => None
        }
    }
}

#[group]
#[commands(vino)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("vim ")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = "***REMOVED***";
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn vino(ctx: &Context, msg: &Message) -> CommandResult { 
    let leaderboard = scrape_leaderboard().await;

    let mut message = String::new();

    for (i, user_info) in leaderboard.iter().enumerate() {
        message.push_str(format!("{}) {} - {} hours\n", i + 1, user_info.username, user_info.total_seconds / (60 * 60) ).as_str());
    }

    msg.channel_id.send_message(&ctx.http, |m| {
            m.embed(|e| e
                .colour(0x00ff00)
                .field("Leaderboard", message, false)
            )
        }).await?;

    Ok(())
}