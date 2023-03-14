use std::fmt::format;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{StandardFramework, CommandResult};
use regex::Regex;
use redis::Commands;
use futures::prelude::*;

const REDIS_URL: &str= "redis://default:qJI0nbg5dpzEt5jrr2R1@containers-us-west-179.railway.app:7784";

#[derive(Debug, Serialize, Deserialize)]
struct UserInfo {
    username: String,
    total_seconds: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserPayload {
    data: UserInfo
}

async fn get_leaderboard_members() -> Vec<String> {
    let mut leaderboard_members: Vec<String> = Vec::new();

    let client = redis::Client::open(REDIS_URL).unwrap();
    let mut con = client.get_connection().expect("Coooo");

    if let Ok(value) = con.get::<&str, String>("members") {
        for username in value.split(":") {
            leaderboard_members.push(username.to_string());
        }
    } else {
        let response = reqwest::get("https://wakapi.krejzac.cz/leaderboard").await.unwrap().text().await.unwrap();
        let re = Regex::new(r#"<strong class="text-ellipsis truncate">@(\w+)</strong>"#).unwrap();

        for cap in re.captures_iter(&response) {
            leaderboard_members.push(cap[1].to_string());
        }

        con.set_ex::<&str, String, String>("members", leaderboard_members.join(":"), 60*60 * 6).unwrap();
    }

    leaderboard_members
}

async fn scrape_leaderboard() -> Vec<UserInfo> {
    let users = get_leaderboard_members().await;

    let client = redis::Client::open(REDIS_URL).unwrap();
    let mut con = client.get_connection().expect("Coooo");

    let mut leaderboard: Vec<UserInfo> = Vec::new();
    let mut usernames_for_fetch: Vec<String> = Vec::new();

    for username in users {
        if let Ok(total_seconds) = con.get::<&str, i32>(&username) {
            leaderboard.push(UserInfo { username: username.clone(), total_seconds });
            println!("{} - cached", username);
        } else {
            println!("{} - fetched", username);
            usernames_for_fetch.push(username);
        }
    }

    // let futures = usernames_for_fetch.iter().map(|u| async move {
    //     get_user_stats(u).await;
    // });

    // let stream = futures::stream::iter(futures).buffered(10);
    // let results = stream.collect::<Vec<_>>().await;


    let results = future::join_all(usernames_for_fetch.into_iter().map(|u| async move { 
            let api_url = format!("https://wakapi.krejzac.cz/api/compat/wakatime/v1/users/{}/stats/month", u);
            let response = reqwest::get(api_url).await.unwrap();

            return match response.json::<UserPayload>().await {
                Ok(val) =>  {
                    Some(UserInfo { username: String::from(val.data.username), total_seconds: val.data.total_seconds})
                },
                Err(_) => None
            }
    })).await;

    for result in results.iter() {
        if let Some(user_info) = result {
            leaderboard.push(UserInfo { username: user_info.username.clone(), total_seconds: user_info.total_seconds });
            con.set_ex::<String, i32, String>(user_info.username.clone(), user_info.total_seconds, 60 * 15).unwrap();
        }
    }

    leaderboard.sort_by_key(|user| -user.total_seconds);
    leaderboard
}

// async fn get_user_stats(username: &str) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>>  {
    // let api_url = format!("https://wakapi.krejzac.cz/api/compat/wakatime/v1/users/{}/stats/month", username);
    // let response = reqwest::get(api_url).await.unwrap();
    // reqwest::get(api_url)


    // return match response.json::<UserPayload>().await {
    //     Ok(val) =>  {
    //         Some(UserInfo { username: String::from(val.data.username), total_seconds: val.data.total_seconds})
    //     },
    //     Err(_) => None
    // }
// }

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
        message.push_str(format!("{}) {} - {:.2} hours\n", i + 1, user_info.username, user_info.total_seconds as f64 / (60 * 60) as f64).as_str());
    }

    msg.channel_id.send_message(&ctx.http, |m| {
            m.embed(|e| e
                .colour(0x00ff00)
                .field("Leaderboard", message, false)
            )
        }).await?;

    Ok(())
}