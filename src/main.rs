use std::env;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::model::prelude::{Ready, GuildId};
use serenity::model::prelude::interaction::{Interaction, InteractionResponseType};
use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{StandardFramework, CommandResult};
use regex::Regex;
use redis::Commands;
use futures::prelude::*;

const REDIS_URL: &str= "redis://default:qJI0nbg5dpzEt5jrr2R1@containers-us-west-179.railway.app:7784";
const LEADERBOARD_URL: &str = "https://wakapi.krejzac.cz/leaderboard";

#[derive(Debug, Serialize, Deserialize)]
struct UserInfo {
    username: String,
    total_seconds: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserPayload {
    data: UserInfo
}

async fn get_leaderboard_users() -> Vec<String> {
    let mut leaderboard_users: Vec<String> = Vec::new();

    let client = redis::Client::open(REDIS_URL).unwrap();
    let mut con = client.get_connection().expect("Coooo");

    if let Ok(value) = con.get::<&str, String>("members") {
        for username in value.split(":") {
            leaderboard_users.push(username.to_string());
        }
    } else {
        let response = reqwest::get(LEADERBOARD_URL).await.unwrap().text().await.unwrap();
        let re = Regex::new(r#"<strong class="text-ellipsis truncate">@(\w+)</strong>"#).unwrap();

        for cap in re.captures_iter(&response) {
            leaderboard_users.push(cap[1].to_string());
        }

        con.set_ex::<&str, String, String>("members", leaderboard_users.join(":"), 60*60 * 6).unwrap();
    }

    leaderboard_users
}

async fn scrape_leaderboard() -> Vec<UserInfo> {
    let users = get_leaderboard_users().await;

    let client = redis::Client::open(REDIS_URL).unwrap();
    let mut con = client.get_connection().expect("Coooo");

    let mut leaderboard: Vec<UserInfo> = Vec::new();
    let mut usernames_for_fetch: Vec<String> = Vec::new();

    for username in users {
        if let Ok(total_seconds) = con.get::<&str, i32>(&username) {
            leaderboard.push(UserInfo { username: username.clone(), total_seconds });
        } else {
            usernames_for_fetch.push(username);
        }
    }

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


#[group]
#[commands(vino)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let txt = vino_helper().await;

            command.create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.embed(|e| e
                            .colour(0x00ff00)
                            .field("Leaderboard", txt, false)
                        )
                    })
            }).await.expect("Cannot respond to slash command");
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        let guild_id = GuildId(
            env::var("GUILD_ID")
                .expect("Expected GUILD_ID in environment")
                .parse()
                .expect("GUILD_ID must be an integer")
        );
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("vim ")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
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

async fn vino_helper() -> String {
    let leaderboard = scrape_leaderboard().await;

    let mut message = String::new();

    for (i, user_info) in leaderboard.iter().enumerate() {
        message.push_str(format!("{}) {} - {:.2} hours\n", i + 1, user_info.username, user_info.total_seconds as f64 / (60 * 60) as f64).as_str());
    }

    message
}

#[command]
async fn vino(ctx: &Context, msg: &Message) -> CommandResult { 
    let message = vino_helper().await;

    msg.channel_id.send_message(&ctx.http, |m| {
            m.embed(|e| e
                .colour(0x00ff00)
                .field("Leaderboard", message, false)
            )
        }).await?;

    Ok(())
}