use anyhow::{Ok, Result};
use redis::{Commands, Connection, FromRedisValue, ToRedisArgs};

pub struct RedisClient {
    host: String,
    port: u16,
    username: String,
    password: String,
    connection: Option<Connection>,
}

impl RedisClient {
    pub fn new(host: String, port: u16, username: String, password: String) -> Self {
        Self {
            host,
            port,
            username,
            password,
            connection: None,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    pub fn connect(&mut self) -> Result<()> {
        let client = redis::Client::open(format!(
            "redis://{}:{}@{}:{}/",
            self.username, self.password, self.host, self.port
        ))?;

        self.connection = Some(client.get_connection().unwrap());

        Ok(())
    }

    pub fn get<RV: FromRedisValue>(&mut self, key: &str) -> Result<Option<RV>> {
        if let Some(ref mut connection) = self.connection {
            let val = connection.get::<&str, Option<RV>>(key)?;

            Ok(val)
        } else {
            Err(anyhow::anyhow!("No connection to Redis"))
        }
    }

    pub fn set<V: ToRedisArgs>(&mut self, key: &str, value: V) -> Result<()> {
        if let Some(ref mut connection) = self.connection {
            connection.set::<&str, V, String>(key, value)?;

            Ok(())
        } else {
            Err(anyhow::anyhow!("No connection to Redis"))
        }
    }

    pub fn set_ex<V: ToRedisArgs>(&mut self, key: &str, value: V, expiration: usize) -> Result<()> {
        if let Some(ref mut connection) = self.connection {
            connection.set_ex::<&str, V, String>(key, value, expiration)?;

            Ok(())
        } else {
            Err(anyhow::anyhow!("No connection to Redis"))
        }
    }

    pub fn del(&mut self, key: &str) -> Result<()> {
        if let Some(ref mut connection) = self.connection {
            connection.del::<&str, ()>(key)?;

            Ok(())
        } else {
            Err(anyhow::anyhow!("No connection to Redis"))
        }
    }
}
