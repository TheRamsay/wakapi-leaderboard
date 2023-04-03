use std::env;
use chrono::{Local, Utc, NaiveDateTime, DateTime, Datelike, NaiveDate};
use dotenv::dotenv;
use lazy_static::lazy_static;
use poise::serenity_prelude::{CreateEmbed, ChannelId, Command, InteractionResponseFlags};
use serde::{Deserialize, Serialize};
use serenity::{async_trait};
use serenity::model::prelude::interaction::{Interaction, InteractionResponseType};
use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{StandardFramework, CommandResult};
use regex::Regex;
use redis::{Commands, Connection};
use futures::prelude::*;

lazy_static! {
    static ref CHANNEL_ID: u64 = env::var("CHANNEL_ID").expect("Expected a channel ID in the environment")
        .parse()
        .expect("Channel ID has to be an integer");
    static ref ADMIN_ID: u64 = env::var("ADMIN_ID").expect("Expected a admin ID in the environment")
        .parse()
        .expect("Admin ID has to be an integer");
    static ref REDIS_URL: String = env::var("REDIS_HOST").expect("Expected a token in the environment");
    static ref REDIS_USERNAME: String = env::var("REDIS_USERNAME").expect("Expected a token in the environment");
    static ref REDIS_PASSWORD: String = env::var("REDIS_PASSWORD").expect("Expected a token in the environment");
    static ref REDIS_PORT: u32 = env::var("REDIS_PORT")
        .expect("Expected a token in the environment")
        .parse()
        .expect("Port has to be an integer.");
}

const REDIS_LEADERBOARD_MEMBERS_KEY: &str = "members";
const REDIS_WINNER_KEY: &str = "winner";
const REDIS_LAST_UPDATE_KEY: &str = "last_update";
const LEADERBOARD_URL: &str = "https://wakapi.krejzac.cz/leaderboard";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UserInfo {
    username: String,
    total_seconds: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UserPayload {
    data: UserInfo
}

fn get_redis_connection() -> Connection {
    let client = redis::Client::open(format!("redis://{}:{}@{}:{}/", *REDIS_USERNAME, *REDIS_PASSWORD, *REDIS_URL, *REDIS_PORT)).unwrap();
    client.get_connection().expect("Can't connect to Redis")
}

fn get_current_datetime() -> String {
    let tz = chrono::FixedOffset::east_opt(2 * 3600).unwrap();
    Local::now().with_timezone(&tz).format("%F %H:%M:%S").to_string()
}

async fn get_leaderboard_users(try_cache: bool) -> Vec<String> {
    let mut leaderboard_users: Vec<String> = Vec::new();

    let mut con = get_redis_connection();

    if try_cache {
        if let Ok(value) = con.get::<&str, String>(REDIS_LEADERBOARD_MEMBERS_KEY) {
            for username in value.split(":") {
                leaderboard_users.push(username.to_string());
            }

            return leaderboard_users;
        }
    }

    let response = reqwest::get(LEADERBOARD_URL).await.unwrap().text().await.unwrap();
    let re = Regex::new(r#"<strong class="text-ellipsis truncate">@(\w+)</strong>"#).unwrap();

    for cap in re.captures_iter(&response) {
        leaderboard_users.push(cap[1].to_string());
    }

    con.set_ex::<&str, String, String>(REDIS_LEADERBOARD_MEMBERS_KEY, leaderboard_users.join(":"), 60*60 * 6).unwrap();
    leaderboard_users

}

async fn scrape_leaderboard(try_cache: bool) -> Vec<UserInfo> {
    let users = get_leaderboard_users(try_cache).await;

    let mut con = get_redis_connection();

    let mut leaderboard: Vec<UserInfo> = Vec::new();
    let mut usernames_for_fetch: Vec<String> = Vec::new();

    for username in users {
        if try_cache {
            if let Ok(total_seconds) = con.get::<&str, i32>(&username) {
                leaderboard.push(UserInfo { username: username.clone(), total_seconds });
                continue;
            }
        }

        usernames_for_fetch.push(username);
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

            con.set::<&str, &str, String>(REDIS_LAST_UPDATE_KEY, &get_current_datetime()).unwrap();
        }
    }

    leaderboard.sort_by_key(|user| -user.total_seconds);
    leaderboard
}

async fn monthly_save(context: Context) {
    loop {
        let now: DateTime<Local> = Local::now();

        let last_day_of_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
            .unwrap()
            .with_month(now.month() + 1)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap())
            .pred_opt().unwrap();

        let duration_until_last_day_of_month = last_day_of_month
            .signed_duration_since(now.date_naive())
            .to_std().unwrap();

        tokio::time::sleep(duration_until_last_day_of_month).await;

        let mut con = get_redis_connection();
        let winner = scrape_leaderboard(false).await.first().unwrap().clone();
        let winner_text = format!("{} - {:.4}", winner.username, winner.total_seconds as f64 / (60 * 60) as f64);
        con.set::<&str, String, String>(REDIS_WINNER_KEY, winner_text).unwrap();

        let text = vino_helper().await;
        ChannelId(*CHANNEL_ID).send_message(&context.http, |m| m.embed(|e| create_winner_embed(text, e))).await.unwrap();
    }
}

#[group]
#[commands(vitez, vino)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {

            if command.data.name == "clear"{

                if *command.user.id.as_u64() != *ADMIN_ID {
                    command.create_interaction_response(&ctx.http, |response| {
                        response.kind(InteractionResponseType::ChannelMessageWithSource).interaction_response_data(|message| {
                            message.flags(InteractionResponseFlags::EPHEMERAL).content("A ven ⚠️")
                        })
                    }).await.unwrap();
                    return;
                }

                clear().await;
                command.create_interaction_response(&ctx.http, |response| {
                    response.kind(InteractionResponseType::ChannelMessageWithSource).interaction_response_data(|message| {
                        message.flags(InteractionResponseFlags::EPHEMERAL).content("Vycisteno 🧹")
                    })
                }).await.unwrap();

                return;
            }

            let text = match command.data.name.as_str() {
                "vino" => vino_helper().await,
                "vitez" => vitez_helper().await,
                _ => "".to_string()
            };

            command.create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.embed(|e| {
                            match command.data.name.as_str() {
                                "vino" => create_leaderboard_embed(text, e),
                                "vitez" => create_winner_embed(text, e),
                                _ => unimplemented!("Coooo")
                            }
                        })
                    })
            }).await.expect("Cannot respond to slash command");
        }
    }

    async fn ready(&self, ctx: Context, msg: poise::serenity_prelude::Ready) {
        
        
        Command::create_global_application_command(&ctx.http, |command| command.name("vino").description("Tabulka frajeru")).await.unwrap();
        Command::create_global_application_command(&ctx.http, |command| command.name("vitez").description("Majitel vina")).await.unwrap();
        Command::create_global_application_command(&ctx.http, |command| command.name("clear").description("Vycisteni cache")).await.unwrap();

        tokio::spawn(monthly_save(ctx.clone()));
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("vim ").case_insensitivity(true))
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

async fn clear() { 
    let mut con = get_redis_connection();

    let mut keys_to_delete = vec![];
    let users = get_leaderboard_users(true).await;

    for username in users {
        keys_to_delete.push(username);
    }

    keys_to_delete.push(String::from(REDIS_LAST_UPDATE_KEY));
    keys_to_delete.push(String::from(REDIS_LEADERBOARD_MEMBERS_KEY));

    for key in keys_to_delete {
        con.del::<String, String>(key);
    }
}

fn create_leaderboard_embed(text: String, embed: &mut CreateEmbed) -> &mut CreateEmbed {
    let mut con = get_redis_connection();

    embed.colour(0xa0517d)
    .title("🍷 Vino leaderboard 🍷")
    .field("", text, false)
    .footer(|f| {
        f.text(format!("Last update: {}", con.get::<&str, String>(REDIS_LAST_UPDATE_KEY).unwrap_or("unknown".to_string())))
    })
}

fn create_winner_embed(text: String, embed: &mut CreateEmbed) -> &mut CreateEmbed {
    embed.color(0xeda42f)
    .title("🥇 Vino vitez 🥇")
    .field("", text, false)
    .footer(|f| f.text("🍷 mnam mnam 🍷"))
}

async fn vitez_helper() -> String {
    let mut con = get_redis_connection();
    con.get::<&str, String>(REDIS_WINNER_KEY).unwrap_or("Zatial neni 😴🍷".to_string())
}

async fn vino_helper() -> String {
    let leaderboard = scrape_leaderboard(true).await;

    let mut message = String::new();

    for (i, user_info) in leaderboard.iter().enumerate() {
        message.push_str(format!("{}) {} - {:.2} hours\n", i + 1, user_info.username, user_info.total_seconds as f64 / (60 * 60) as f64).as_str());
    }

    message
}

#[command]
async fn vitez(ctx: &Context, msg: &Message) -> CommandResult {
    let winner = vitez_helper().await;

    msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| create_winner_embed(winner, e))).await?;

    Ok(())
}

#[command]
async fn vino(ctx: &Context, msg: &Message) -> CommandResult { 
    let text = vino_helper().await;

    msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| create_leaderboard_embed(text, e))).await?;

    Ok(())
}