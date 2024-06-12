use std::{collections::HashSet, str::FromStr, sync::Arc};

use anyhow::Result;
use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use commands::{clear, vinak, vino, vitez};
use config::{
    CHANNEL_ID, DISCORD_TOKEN, REDIS_PASSWORD, REDIS_PORT, REDIS_URL, REDIS_USERNAME,
    REDIS_WINNER_KEY, TIMEZONE,
};
use dotenv::dotenv;
use poise::serenity_prelude::{
    self as serenity, ChannelId, CreateEmbed, CreateEmbedFooter, CreateMessage, UserId,
};
use redis_client::{RedisClient, SharedRedisClient};
use scraper::WakapiScraper;
use tokio::sync::{Mutex, RwLock};

mod commands;
mod config;
mod models;
mod redis_client;
mod scraper;
mod utils;

pub struct Data {
    pub redis_client: SharedRedisClient,
    pub wakapi_scraper: Arc<Mutex<WakapiScraper>>,
} // User data, which is stored and accessible in all command invocations
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let options = poise::FrameworkOptions {
        commands: vec![vino(), clear(), vinak(), vitez()],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("aa".into()),
            ..Default::default()
        },
        owners: HashSet::from([UserId::new(314659552505102347)]),
        // The global error handler for all error cases that may occur
        on_error: |error| Box::pin(on_error(error)),
        // This code is run before every command
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        // This code is run after a command if it was successful (returned Ok)
        post_command: |ctx| {
            Box::pin(async move {
                println!("Executed command {}!", ctx.command().qualified_name);
            })
        },
        event_handler: |_ctx, event, _framework, _data| {
            Box::pin(async move {
                println!(
                    "Got an event in event handler: {:?}",
                    event.snake_case_name()
                );
                Ok(())
            })
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .options(options)
        .setup(
            move |ctx, _ready, framework: &poise::Framework<Data, Error>| {
                Box::pin(async move {
                    println!("Logged in as {}", _ready.user.name);
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    let redis_client = Arc::new(RwLock::new(RedisClient::new(
                        (*REDIS_URL).to_owned(),
                        *REDIS_PORT,
                        (*REDIS_USERNAME).to_owned(),
                        (*REDIS_PASSWORD).to_owned(),
                    )?));

                    let wakapi_scraper =
                        Arc::new(Mutex::new(WakapiScraper::new(redis_client.clone())));

                    let data = Data {
                        redis_client: redis_client.clone(),
                        wakapi_scraper: wakapi_scraper.clone(),
                    };

                    ctx.data
                        .write()
                        .await
                        .insert::<WakapiScraper>(wakapi_scraper.clone());

                    ctx.data
                        .write()
                        .await
                        .insert::<RedisClient>(redis_client.clone());

                    tokio::spawn(monthly_save(ctx.clone()));

                    Ok(data)
                })
            },
        )
        .build();

    // Login with a bot token from the environment
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let token = (*DISCORD_TOKEN).to_owned();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    if let Err(why) = client {
        println!("An error occurred while creating the client: {:?}", why);
        return;
    }

    if let Err(why) = client.unwrap().start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

async fn monthly_save(ctx: serenity::Context) -> Result<()> {
    loop {
        let timezone = Tz::from_str((*TIMEZONE).as_str())?;
        let now = Utc::now().with_timezone(&timezone);

        let last_day_of_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
            .unwrap()
            .with_month(now.month() + 1)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap())
            .pred_opt()
            .unwrap();

        let duration_until_last_day_of_month = last_day_of_month
            .signed_duration_since(now.date_naive())
            .to_std()
            .unwrap();

        tokio::time::sleep(duration_until_last_day_of_month).await;

        let winner_text: String;

        let wakapi_client = ctx
            .data
            .write()
            .await
            .get::<WakapiScraper>()
            .unwrap()
            .clone();

        if let Some(winner) = wakapi_client
            .lock()
            .await
            .scrape_leaderboard(false)
            .await?
            .first()
        {
            winner_text = format!(
                "{} - {:.4}",
                winner.username,
                winner.total_seconds as f64 / (60 * 60) as f64
            );

            {
                let redis_client = ctx.data.write().await.get::<RedisClient>().unwrap().clone();
                redis_client
                    .write()
                    .await
                    .set::<String>(REDIS_WINNER_KEY, winner_text.clone())
                    .await?;
            }
        } else {
            winner_text = "Zatial neni 😴🍷".to_string();
        }

        ChannelId::new(*CHANNEL_ID)
            .send_message(
                &ctx.http,
                CreateMessage::default().embed(
                    CreateEmbed::new()
                        .color(0xeda42f)
                        .title("🥇 Vino vitez 🥇")
                        .field("", winner_text, false)
                        .footer(CreateEmbedFooter::new("🍷 mnam mnam 🍷")),
                ),
            )
            .await?;
    }
}
