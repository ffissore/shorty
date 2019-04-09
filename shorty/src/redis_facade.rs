// Copyright 2019 Federico Fissore
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! redis_facade is a convenience module holding `RedisFacade`

use redis::Commands;
use redis::{Connection, RedisResult};

/// `RedisFacade` is a wrapper around a `redis` `Connection`. It provides convenience methods such
/// as `get_string` and `get_bool` which otherwise would be coded as `get::<_, String>` and
/// `get::<_, bool>`, making it harder to stub the struct and properly test `shorty`.
pub struct RedisFacade {
    redis: Connection,
}

impl RedisFacade {
    /// Creates a new `RedisFacade`, owning an active `redis` `Connection`
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
