use anyhow::Result;
use futures::future;

use crate::{
    config::{
        get_redis_client, REDIS_LAST_UPDATE_KEY, REDIS_LEADERBOARD_MEMBERS_KEY, WAKAPI_DOMAIN,
    },
    models::{UserInfo, UserPayload},
    utils::get_current_datetime,
};

pub async fn get_leaderboard_users(try_cache: bool) -> Result<Vec<String>> {
    let mut leaderboard_users: Vec<String> = Vec::new();

    let mut client = get_redis_client().await?;

    if try_cache {
        if let Some(value) = client.get::<String>(REDIS_LEADERBOARD_MEMBERS_KEY)? {
            for username in value.split(":") {
                leaderboard_users.push(username.to_string());
            }

            return Ok(leaderboard_users);
        }
    }

    let leaderboard_endpoint = format!("https://{}/leaderboard", *WAKAPI_DOMAIN);

    let response = reqwest::get(leaderboard_endpoint).await?.text().await?;

    let re =
        regex::Regex::new(r#"<strong class="text-ellipsis truncate">@(\w+)</strong>"#).unwrap();

    for cap in re.captures_iter(&response) {
        leaderboard_users.push(cap[1].to_string());
    }

    if !leaderboard_users.is_empty() {
        client.set_ex(
            REDIS_LEADERBOARD_MEMBERS_KEY,
            leaderboard_users.join(":").as_str(),
            60 * 60 * 6,
        )?;
    }

    Ok(leaderboard_users)
}

pub async fn scrape_leaderboard(try_cache: bool) -> Result<Vec<UserInfo>> {
    let users = get_leaderboard_users(try_cache).await?;

    let mut client = get_redis_client().await?;

    let mut leaderboard: Vec<UserInfo> = Vec::new();
    let mut usernames_for_fetch: Vec<String> = Vec::new();

    for username in users {
        if try_cache {
            if let Some(total_seconds) = client.get::<usize>(&username)? {
                leaderboard.push(UserInfo {
                    username: username.clone(),
                    total_seconds,
                    languages: vec![],
                });
                continue;
            }
        }

        usernames_for_fetch.push(username);
    }

    let results = future::join_all(usernames_for_fetch.into_iter().map(|u| async move {
        let api_url = format!(
            "https://{}/api/compat/wakatime/v1/users/{}/stats/month",
            WAKAPI_DOMAIN.as_str(),
            u
        );
        let response = reqwest::get(api_url).await.unwrap();

        return match response.json::<UserPayload>().await {
            Ok(val) => Some(UserInfo {
                username: String::from(val.data.username),
                total_seconds: val.data.total_seconds,
                languages: vec![],
            }),
            Err(_) => None,
        };
    }))
    .await;

    for result in results.iter() {
        if let Some(user_info) = result {
            leaderboard.push(UserInfo {
                username: user_info.username.clone(),
                total_seconds: user_info.total_seconds,
                languages: vec![],
            });
            client
                .set_ex::<usize>(&user_info.username, user_info.total_seconds, 60 * 15)
                .unwrap();

            client
                .set::<&str>(REDIS_LAST_UPDATE_KEY, &get_current_datetime()?)
                .unwrap();
        }
    }

    leaderboard.sort_by_key(|user| -(user.total_seconds as i64));
    Ok(leaderboard)
}
