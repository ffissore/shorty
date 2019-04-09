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

#[macro_use]
extern crate serde_derive;

use core::fmt;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[cfg(not(test))]
use crate::redis_facade::RedisFacade;
#[cfg(test)]
use tests::MockRedisFacade as RedisFacade;

#[cfg(not(test))]
pub mod redis_facade;

#[derive(Debug)]
pub struct ShortenerError {
    message: &'static str,
    cause: Option<Box<Error>>,
}

impl ShortenerError {
    fn new(message: &'static str) -> ShortenerError {
        ShortenerError {
            message,
            cause: None,
        }
    }
    fn new_with_cause(message: &'static str, error: Box<Error>) -> ShortenerError {
        ShortenerError {
            message,
            cause: Some(error),
        }
    }
}

impl Display for ShortenerError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.write_str(&self.message)?;

        if let Some(err) = &self.cause {
            return f.write_str(&format!(" - {}", &err));
        }

        Ok(())
    }
}

impl Error for ShortenerError {}

pub struct Shortener {
    id_length: usize,
    id_alphabet: Vec<char>,
    redis: RedisFacade,
    rate_limit_period: usize,
    rate_limit: i64,
}

#[derive(Serialize)]
pub struct ShortenerResult {
    id: String,
    url: String,
}

impl Shortener {
    pub fn new(
        id_length: usize,
        id_alphabet: Vec<char>,
        redis: RedisFacade,
        rate_limit_period: usize,
        rate_limit: i64,
    ) -> Shortener {
        Shortener {
            id_length,
            id_alphabet,
            redis,
            rate_limit_period,
            rate_limit,
        }
    }

    pub fn lookup(&mut self, id: &str) -> Option<String> {
        match self.redis.get_string(id) {
            Ok(url) => Some(url),
            Err(_) => None,
        }
    }

    fn verify_api_key(&mut self, api_key: &str) -> Result<(), ShortenerError> {
        let api_key = format!("API_KEY_{}", api_key);
        log::trace!("verifying api key '{}'", api_key);

        let verify_and_increment = self.redis.get_bool(&api_key).and_then(|_| {
            if self.rate_limit <= 0 {
                return Ok(-1);
            }

            let rate_key = format!("RATE_{}", api_key);
            log::trace!("verifying rate key '{}'", rate_key);

            self.redis.exists(&rate_key).and_then(|exists| {
                log::trace!("rate key exists {}", exists);

                self.redis
                    .increment(&format!("RATE_{}", api_key))
                    .and_then(|number_of_calls| {
                        log::trace!("rate key {} number of calls {}", rate_key, number_of_calls);

                        if !exists {
                            self.redis.expire(&api_key, self.rate_limit_period).unwrap();
                        }

                        Ok(number_of_calls)
                    })
            })
        });

        match verify_and_increment {
            Ok(call_rate) if self.rate_limit > 0 && call_rate > self.rate_limit => {
                Err(ShortenerError::new("Rate limit exceeded"))
            }
            Ok(_) => Ok(()),
            Err(err) => Err(ShortenerError::new_with_cause(
                "Invalid API key",
                Box::new(err),
            )),
        }
    }

    pub fn shorten(
        &mut self,
        api_key: &Option<&str>,
        url: &str,
    ) -> Result<ShortenerResult, ShortenerError> {
        let id = nanoid::custom(self.id_length, &self.id_alphabet);

        let verify_result = api_key
            .as_ref()
            .map(|api_key| self.verify_api_key(api_key))
            .unwrap_or(Ok(()));

        verify_result.and_then(|_| {
            let mut url = url.to_owned();
            if !url.to_lowercase().starts_with("http") {
                url = format!("http://{}", url);
            }

            self.redis
                .set(&id, url.as_str())
                .map(|_| ShortenerResult { id, url })
                .map_err(|err| ShortenerError::new_with_cause("redis error", Box::new(err)))
        })
    }
}

#[cfg(test)]
mod tests {
    use redis::{ErrorKind, RedisError, RedisResult};

    use super::*;

    pub struct MockRedisFacade {
        get_string_answers: Vec<RedisResult<String>>,
        get_bool_answers: Vec<RedisResult<bool>>,
        exists_answers: Vec<RedisResult<bool>>,
        set_answers: Vec<RedisResult<()>>,
        incr_answers: Vec<RedisResult<i64>>,
        expire_answers: Vec<RedisResult<()>>,
    }

    impl MockRedisFacade {
        fn new() -> Self {
            MockRedisFacade {
                get_string_answers: vec![],
                get_bool_answers: vec![],
                exists_answers: vec![],
                set_answers: vec![],
                incr_answers: vec![],
                expire_answers: vec![],
            }
        }

        pub fn get_string(&mut self, _key: &str) -> RedisResult<String> {
            match self.get_string_answers.pop() {
                Some(s) => s,
                None => Err(RedisError::from((
                    ErrorKind::IoError,
                    "unexpected get_string call",
                ))),
            }
        }

        pub fn get_bool(&mut self, _key: &str) -> RedisResult<bool> {
            match self.get_bool_answers.pop() {
                Some(s) => s,
                None => Err(RedisError::from((
                    ErrorKind::IoError,
                    "unexpected get call",
                ))),
            }
        }

        pub fn exists(&mut self, _key: &str) -> RedisResult<bool> {
            match self.exists_answers.pop() {
                Some(s) => s,
                None => Err(RedisError::from((
                    ErrorKind::IoError,
                    "unexpected exists call",
                ))),
            }
        }

        pub fn set(&mut self, _key: &str, _value: &str) -> RedisResult<()> {
            match self.set_answers.pop() {
                Some(s) => s,
                None => Err(RedisError::from((
                    ErrorKind::IoError,
                    "unexpected set call",
                ))),
            }
        }

        pub fn increment(&mut self, _key: &str) -> RedisResult<i64> {
            match self.incr_answers.pop() {
                Some(s) => s,
                None => Err(RedisError::from((
                    ErrorKind::IoError,
                    "unexpected incr call",
                ))),
            }
        }

        pub fn expire(&mut self, _key: &str, _seconds: usize) -> RedisResult<()> {
            match self.expire_answers.pop() {
                Some(s) => s,
                None => Err(RedisError::from((
                    ErrorKind::IoError,
                    "unexpected expire call",
                ))),
            }
        }
    }

    #[test]
    fn test_lookup() {
        let mut redis = MockRedisFacade::new();
        &redis.get_string_answers.push(Ok(String::from("test url")));

        let mut shortener = Shortener::new(10, vec!['a', 'b', 'c'], redis, 600, 10);
        assert_eq!(shortener.lookup("id").unwrap(), "test url");
    }

    #[test]
    fn test_shorten_happy_path_first_call() {
        let mut redis = MockRedisFacade::new();
        &redis.get_bool_answers.push(Ok(true));
        &redis.exists_answers.push(Ok(false));
        &redis.incr_answers.push(Ok(1));
        &redis.expire_answers.push(Ok(()));
        &redis.set_answers.push(Ok(()));

        let mut shortener = Shortener::new(10, vec!['a', 'b', 'c'], redis, 600, 10);
        let shorten_result = shortener.shorten(&Some("api key"), "A url").unwrap();
        assert_eq!(10, shorten_result.id.len());
        assert_eq!("http://A url", shorten_result.url);
    }

    #[test]
    fn test_shorten_happy_path_no_rate_limit() {
        let mut redis = MockRedisFacade::new();
        &redis.get_bool_answers.push(Ok(true));
        &redis.set_answers.push(Ok(()));

        let mut shortener = Shortener::new(10, vec!['a', 'b', 'c'], redis, 600, -1);
        let shorten_result = shortener.shorten(&Some("api key"), "A url").unwrap();
        assert_eq!(10, shorten_result.id.len());
        assert_eq!("http://A url", shorten_result.url);
    }

    #[test]
    fn test_shorten_happy_path_second_call() {
        let mut redis = MockRedisFacade::new();
        &redis.get_bool_answers.push(Ok(true));
        &redis.exists_answers.push(Ok(true));
        &redis.incr_answers.push(Ok(2));
        &redis.set_answers.push(Ok(()));

        let mut shortener = Shortener::new(10, vec!['a', 'b', 'c'], redis, 600, 10);
        let shorten_result = shortener.shorten(&Some("api key"), "A url").unwrap();
        assert_eq!(10, shorten_result.id.len());
        assert_eq!("http://A url", shorten_result.url);
    }

    #[test]
    fn test_shorten_happy_path_no_api_key() {
        let mut redis = MockRedisFacade::new();
        &redis.get_bool_answers.push(Ok(true));
        &redis.exists_answers.push(Ok(false));
        &redis.incr_answers.push(Ok(1));
        &redis.expire_answers.push(Ok(()));
        &redis.set_answers.push(Ok(()));

        let mut shortener = Shortener::new(10, vec!['a', 'b', 'c'], redis, 600, 10);
        let shorten_result = shortener.shorten(&None, "A url").unwrap();
        assert_eq!(10, shorten_result.id.len());
        assert_eq!("http://A url", shorten_result.url);
    }

    #[test]
    fn test_shorten_unhappy_path_too_many_calls() {
        let rate_limit = 10;
        let mut redis = MockRedisFacade::new();
        &redis.get_bool_answers.push(Ok(true));
        &redis.exists_answers.push(Ok(true));
        &redis.incr_answers.push(Ok(rate_limit + 1));
        &redis.set_answers.push(Ok(()));

        let mut shortener = Shortener::new(10, vec!['a', 'b', 'c'], redis, 600, rate_limit);
        let shorten_result_err = shortener.shorten(&Some("api key"), "A url").err().unwrap();
        assert_eq!("Rate limit exceeded", shorten_result_err.message);
    }

    #[test]
    fn test_shorten_unhappy_path_invalid_api_key() {
        let mut redis = MockRedisFacade::new();
        &redis.get_bool_answers.push(Ok(false));

        let mut shortener = Shortener::new(10, vec!['a', 'b', 'c'], redis, 600, 10);
        let shorten_result_err = shortener.shorten(&Some("api key"), "A url").err().unwrap();
        assert_eq!("Invalid API key", shorten_result_err.message);
    }
}
