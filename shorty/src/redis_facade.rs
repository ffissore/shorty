use redis::Commands;
use redis::{Connection, RedisResult};

pub struct RedisFacade {
    redis: Connection,
}

impl RedisFacade {
    pub fn new(redis: Connection) -> RedisFacade {
        RedisFacade { redis }
    }

    pub fn get_string(&self, key: &str) -> RedisResult<String> {
        self.redis.get::<_, String>(key)
    }

    pub fn get_bool(&self, key: &str) -> RedisResult<bool> {
        self.redis.get::<_, bool>(key)
    }

    pub fn exists(&self, key: &str) -> RedisResult<bool> {
        self.redis.exists::<_, bool>(key)
    }

    pub fn increment(&self, key: &str) -> RedisResult<i64> {
        self.redis.incr::<_, _, i64>(key, 1)
    }

    pub fn expire(&self, key: &str, period: usize) -> RedisResult<()> {
        self.redis.expire::<_, ()>(key, period)
    }

    pub fn set(&self, key: &str, value: &str) -> RedisResult<()> {
        self.redis.set::<_, _, ()>(key, value)
    }
}
