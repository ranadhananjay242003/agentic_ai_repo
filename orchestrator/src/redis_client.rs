use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Clone)]
pub struct RedisClient {
    connection: ConnectionManager,
}

impl RedisClient {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let connection = client.get_tokio_connection_manager().await?;
        Ok(Self { connection })
    }

    pub async fn publish(&mut self, channel: &str, message: &str) -> Result<()> {
        // FIX: Added <_, _, ()> to specify return type
        self.connection.publish::<_, _, ()>(channel, message).await?;
        Ok(())
    }

    pub async fn set_with_expiry<T: Serialize>(
        &mut self,
        key: &str,
        value: &T,
        expiry_secs: usize,
    ) -> Result<()> {
        let json = serde_json::to_string(value)?;
        // FIX: Added <_, _, ()>
        self.connection.set_ex::<_, _, ()>(key, json, expiry_secs as u64).await?;
        Ok(())
    }

    pub async fn get<T: for<'de> Deserialize<'de>>(&mut self, key: &str) -> Result<Option<T>> {
        let result: Option<String> = self.connection.get(key).await?;
        match result {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    pub async fn delete(&mut self, key: &str) -> Result<()> {
        // FIX: Added <_, ()>
        self.connection.del::<_, ()>(key).await?;
        Ok(())
    }
}