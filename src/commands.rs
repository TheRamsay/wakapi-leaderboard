use poise::serenity_prelude::{CreateEmbed, CreateEmbedFooter, User};

use crate::{
    config::{
        get_redis_client, REDIS_LAST_UPDATE_KEY, REDIS_LEADERBOARD_MEMBERS_KEY, REDIS_PASSWORD,
        REDIS_PORT, REDIS_URL, REDIS_USERNAME, REDIS_WINNER_KEY, WAKAPI_DOMAIN,
    },
    models::UserPayload,
    redis_client::RedisClient,
    scraper::{get_leaderboard_users, scrape_leaderboard},
    Context, Error,
};

#[poise::command(slash_command, prefix_command)]
pub async fn vino(ctx: Context<'_>) -> Result<(), Error> {
    let leaderboard = scrape_leaderboard(true).await?;

    let mut message = String::new();

    for (i, user_info) in leaderboard.iter().enumerate() {
        message.push_str(
            format!(
                "{}) {} - {:.2} hours\n",
                i + 1,
                user_info.username,
                user_info.total_seconds as f64 / (60 * 60) as f64
            )
            .as_str(),
        );
    }

    ctx.send(
        poise::CreateReply::default().embed(
            CreateEmbed::new()
                .title("🍷 Vino leaderboard 🍷")
                .field("", message, false)
                .footer(CreateEmbedFooter::new("Powered by Vino")),
        ),
    )
    .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, owners_only)]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let mut client = get_redis_client().await?;

    let mut keys_to_delete = vec![];
    let users = get_leaderboard_users(true).await?;

    for username in users {
        keys_to_delete.push(username);
    }

    keys_to_delete.push(String::from(REDIS_LAST_UPDATE_KEY));
    keys_to_delete.push(String::from(REDIS_LEADERBOARD_MEMBERS_KEY));

    for ref key in keys_to_delete {
        client.del(key)?;
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command, owners_only)]
pub async fn vinak(
    ctx: Context<'_>,
    #[description = "Vinak username (not based on discord username)"] username: String,
) -> Result<(), Error> {
    let api_url = format!(
        "https://{}/api/compat/wakatime/v1/users/{}/stats/month",
        WAKAPI_DOMAIN.as_str(),
        username
    );

    let response = reqwest::get(api_url).await.unwrap();

    match response.json::<UserPayload>().await {
        Ok(payload) => {
            let user_info = payload.data;

            let mut message = String::new();

            for lang in user_info.languages {
                if lang.total_seconds > 0 {
                    message.push_str(
                        format!(
                            "{} - {:.2} hours\n",
                            lang.name,
                            lang.total_seconds as f64 / (60 * 60) as f64
                        )
                        .as_str(),
                    );
                }
            }

            ctx.send(
                poise::CreateReply::default().embed(
                    CreateEmbed::new()
                        .title(format!("🍷 Víňák {} 🍷", user_info.username))
                        .field("", message, false),
                ),
            )
            .await?;
        }
        Err(_) => {
            ctx.say("User not found").await?;
        }
    };

    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn vitez(ctx: Context<'_>) -> Result<(), Error> {
    let mut client = get_redis_client().await?;

    let winner = client
        .get::<String>(REDIS_WINNER_KEY)?
        .unwrap_or("Zatial neni 😴🍷".to_string());

    ctx.send(
        poise::CreateReply::default().embed(
            CreateEmbed::new()
                .color(0xeda42f)
                .title("🥇 Vino vitez 🥇")
                .field("", winner, false)
                .footer(CreateEmbedFooter::new("🍷 mnam mnam 🍷")),
        ),
    )
    .await?;

    Ok(())
}
