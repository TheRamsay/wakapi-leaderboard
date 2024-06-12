use lazy_static::lazy_static;

use std::env;

pub const REDIS_LEADERBOARD_MEMBERS_KEY: &str = "members";
pub const REDIS_WINNER_KEY: &str = "winner";
pub const REDIS_LAST_UPDATE_KEY: &str = "last_update";

lazy_static! {
    pub static ref DISCORD_TOKEN: String =
        env::var("DISCORD_TOKEN").expect("Expected discord token in the environment");
    pub static ref WAKAPI_DOMAIN: String =
        env::var("WAKAPI_DOMAIN").expect("Expected wakapi domain in the environment");
    pub static ref CHANNEL_ID: u64 = env::var("CHANNEL_ID")
        .expect("Expected a channel ID in the environment")
        .parse()
        .expect("Channel ID has to be an integer");
    pub static ref ADMIN_ID: u64 = env::var("ADMIN_ID")
        .expect("Expected a admin ID in the environment")
        .parse()
        .expect("Admin ID has to be an integer");
    pub static ref REDIS_URL: String =
        env::var("REDIS_HOST").expect("Expected a token in the environment");
    pub static ref REDIS_USERNAME: String =
        env::var("REDIS_USERNAME").expect("Expected a token in the environment");
    pub static ref REDIS_PASSWORD: String =
        env::var("REDIS_PASSWORD").expect("Expected a token in the environment");
    pub static ref REDIS_PORT: u16 = env::var("REDIS_PORT")
        .expect("Expected a token in the environment")
        .parse()
        .expect("Port has to be an integer.");
    pub static ref TIMEZONE: String =
        env::var("TIMEZONE").expect("Expected a timezone in the environment");
}
