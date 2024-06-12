use std::sync::Arc;

use anyhow::{Ok, Result};
use poise::serenity_prelude::prelude::TypeMapKey;
use redis::{
    aio::MultiplexedConnection, AsyncCommands, Client, Commands, FromRedisValue,
    ToRedisArgs,
};
use tokio::sync::RwLock;

pub type SharedRedisClient = Arc<RwLock<RedisClient>>;

pub struct RedisClient {
    client: Client,
}
impl TypeMapKey for RedisClient {
    type Value = SharedRedisClient;
}

impl RedisClient {
    pub fn new(host: String, port: u16, username: String, password: String) -> Result<Self> {
        Ok(Self {
            client: Client::open(format!(
                "redis://{}:{}@{}:{}/",
                username, password, host, port
            ))?,
        })
    }

    async fn get_connection(&self) -> Result<MultiplexedConnection> {
        Ok(self.client.get_multiplexed_async_connection().await?)
    }

    pub async fn get<RV: FromRedisValue>(&mut self, key: &str) -> Result<Option<RV>> {
        let mut connection = self.get_connection().await?;
        Ok(connection.get(key).await?)
    }

    pub async fn set<V: ToRedisArgs + Send + Sync>(&mut self, key: &str, value: V) -> Result<()> {
        let mut connection = self.get_connection().await?;
        connection.set(key, value).await?;

        Ok(())
    }

    pub async fn set_ex<V: ToRedisArgs + Send + Sync>(
        &mut self,
        key: &str,
        value: V,
        expiration: usize,
    ) -> Result<()> {
        let mut connection = self.get_connection().await?;
        connection.set_ex(key, value, expiration).await?;

        Ok(())
    }

    pub async fn del(&mut self, key: &str) -> Result<()> {
        let mut connection = self.get_connection().await?;
        connection.del(key).await?;

        Ok(())
    }
}
