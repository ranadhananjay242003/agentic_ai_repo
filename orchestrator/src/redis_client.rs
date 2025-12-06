use redis::{aio::ConnectionManager, AsyncCommands, Client};
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Clone)]
pub struct RedisClient {
    connection: ConnectionManager,
}

impl RedisClient {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        let connection = ConnectionManager::new(client).await?;
        
        Ok(Self { connection })
    }

    pub async fn publish(&mut self, channel: &str, message: &str) -> Result<()> {
        self.connection.publish(channel, message).await?;
        Ok(())
    }

    pub async fn set_with_expiry<T: Serialize>(
        &mut self,
        key: &str,
        value: &T,
        expiry_secs: usize,
    ) -> Result<()> {
        let json = serde_json::to_string(value)?;
        self.connection
            .set_ex(key, json, expiry_secs as u64)
            .await?;
        Ok(())
    }

    pub async fn get<T: for<'de> Deserialize<'de>>(&mut self, key: &str) -> Result<Option<T>> {
        let json: Option<String> = self.connection.get(key).await?;
        match json {
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
            None => Ok(None),
        }
    }

    pub async fn delete(&mut self, key: &str) -> Result<()> {
        self.connection.del(key).await?;
        Ok(())
    }
}
