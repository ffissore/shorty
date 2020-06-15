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

//! # Shorty
//!
//! `shorty` is a URL shortener: it assigns a short ID to a URL of any length, and when people will
//! access the URL with that short ID, they will be redirected to the original URL.
//!
#[macro_use]
extern crate serde_derive;

use core::fmt;
use std::error::Error;
use std::fmt::{Display, Formatter};

use redis::{ErrorKind, RedisError};
use url::Url;

#[cfg(test)]
use tests::StubRedisFacade as RedisFacade;

#[cfg(not(test))]
use crate::redis_facade::RedisFacade;

#[cfg(not(test))]
pub mod redis_facade;

#[derive(Debug)]
pub struct ShortenerError {
    message: &'static str,
    cause: Option<Box<dyn Error>>,
}

impl ShortenerError {
    fn new(message: &'static str) -> ShortenerError {
        ShortenerError {
            message,
            cause: None,
        }
    }
    fn new_with_cause(message: &'static str, error: Box<dyn Error>) -> ShortenerError {
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

/// `Shortener` is the struct exposing methods `lookup` and `shorten`.
///
/// `lookup` attempts to resolve an ID to a URL. If no URL is found or an error occurs, it returns
/// `None`, otherwise it returns `Some(url)`.
///
/// `shorten` takes an optional API key and a URL to shorten. If the API key is present, it will
/// validate it and shorten the URL only if validation passes. Otherwise, it will just shorten the
/// URL.
///
/// `Shortener` interacts with a `RedisFacade`, which makes it easier to work with the `redis` crate
/// and simplifies testing.
pub struct Shortener {
    id_length: usize,
    id_alphabet: Vec<char>,
    id_generation_max_attempts: u8,
    redis: RedisFacade,
    rate_limit_period: usize,
    rate_limit: i64,
}

/// A struct with the successful result of a URL shortening. It holds the original `url` and the
/// resulting `id`
#[derive(Serialize)]
pub struct ShortenerResult {
    id: String,
    url: String,
}

impl Shortener {
    /// Creates a new Shortener
    ///
    /// `id_length` is the length of the generated ID.
    ///
    /// `id_alphabet` is the alphabet used in the ID: a decent one is `a-zA-Z0-9` as each entry has
    /// 62 possible values and is ASCII
    ///
    /// `id_generation_max_attempts` is the number of attempts to generate an unique ID when a
    /// conflict is detected.
    ///
    /// `redis` is a `RedisFacade` instance.
    ///
    /// `rate_limit_period` is the amount of seconds during which calls to `shorten` will be counted.
    ///
    /// `rate_limit` is the max number of calls that can be made to `shorten` in a period.
    pub fn new(
        id_length: usize,
        id_alphabet: Vec<char>,
        id_generation_max_attempts: u8,
        redis: RedisFacade,
        rate_limit_period: usize,
        rate_limit: i64,
    ) -> Shortener {
        Shortener {
            id_length,
            id_alphabet,
            id_generation_max_attempts,
            redis,
            rate_limit_period,
            rate_limit,
        }
    }

    /// Looks up a URL by the given ID. If no URL is found or an error occurs, it returns `None`,
    /// otherwise it returns `Some(url)`.
    pub fn lookup(&self, id: &str) -> Option<String> {
        match self.redis.get_string(id) {
            Ok(url) => Some(url),
            Err(_) => None,
        }
    }

    fn verify_api_key(&self, api_key: &str) -> Result<(), ShortenerError> {
        let api_key = format!("API_KEY_{}", api_key);
        log::trace!("verifying api key '{}'", api_key);

        let verify_and_increment = self.redis.get_bool(&api_key).and_then(|valid| {
            if !valid {
                return Err(RedisError::from((
                    ErrorKind::ExtensionError,
                    "API key expired",
                )));
            }

            if self.rate_limit <= 0 {
                return Ok(-1);
            }

            let rate_key = format!("RATE_{}", api_key);
            log::trace!("verifying rate key '{}'", rate_key);

            self.redis.exists(&rate_key).and_then(|exists| {
                log::trace!("rate key exists {}", exists);

                self.redis.increment(&rate_key).and_then(|number_of_calls| {
                    log::trace!("rate key {} number of calls {}", rate_key, number_of_calls);

                    if !exists {
                        self.redis
                            .expire(&rate_key, self.rate_limit_period)
                            .unwrap();
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

    fn generate_id(&self) -> Result<String, ShortenerError> {
        for _ in 1..=self.id_generation_max_attempts {
            let id = nanoid::custom(self.id_length, &self.id_alphabet);

            let exists = self.redis.exists(&id).unwrap_or(false);

            if !exists {
                return Ok(id);
            }
        }

        Err(ShortenerError::new(
            "Failed to generate an ID: too many attempts. Consider using a longer ID",
        ))
    }

    /// Shortens an URL, returning a `ShortenerResult` holding the provided URL and the generated ID.
    ///
    /// If the optional API key is present, it will validate it and shorten the URL only if
    /// validation passes.
    ///
    /// If the optional host is present, it will ensure that the url to shorten is not a url from
    /// the same host that's running shorty (which would create a link loop)
    ///
    /// Otherwise, it will just shorten the URL.
    pub fn shorten(
        &self,
        api_key: &Option<&str>,
        host: Option<&str>,
        url: &str,
    ) -> Result<ShortenerResult, ShortenerError> {
        let verify_result = api_key
            .as_ref()
            .map(|api_key| self.verify_api_key(api_key))
            .unwrap_or(Ok(()));

        verify_result
            .and_then(|_| self.generate_id())
            .and_then(|id| {
                let mut url = url.to_owned();
                if !url.to_lowercase().starts_with("http") {
                    url = format!("http://{}", url);
                }
                Url::parse(&url)
                    .and_then(|parsed_url| Ok((id, url, parsed_url)))
                    .map_err(|parse_err| {
                        ShortenerError::new_with_cause("Unable to parse url", Box::new(parse_err))
                    })
            })
            .and_then(|(id, url, parsed_url)| {
                if host.is_none() {
                    return Ok((id, url));
                }

                if parsed_url.host_str().unwrap().eq(host.unwrap()) {
                    return Err(ShortenerError::new("Link loop is not allowed"));
                }

                Ok((id, url))
            })
            .and_then(|(id, url)| {
                self.redis
                    .set(&id, url.as_str())
                    .map(|_| ShortenerResult { id, url })
                    .map_err(|err| ShortenerError::new_with_cause("Redis error", Box::new(err)))
            })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use redis::RedisResult;

    use super::*;

    pub struct StubRedisFacade {
        get_string_answers: RefCell<Vec<RedisResult<String>>>,
        get_bool_answers: RefCell<Vec<RedisResult<bool>>>,
        exists_answers: RefCell<Vec<RedisResult<bool>>>,
        set_answers: RefCell<Vec<RedisResult<()>>>,
        incr_answers: RefCell<Vec<RedisResult<i64>>>,
        expire_answers: RefCell<Vec<RedisResult<()>>>,
    }

    impl StubRedisFacade {
        fn new() -> Self {
            StubRedisFacade {
                get_string_answers: RefCell::new(vec![]),
                get_bool_answers: RefCell::new(vec![]),
                exists_answers: RefCell::new(vec![]),
                set_answers: RefCell::new(vec![]),
                incr_answers: RefCell::new(vec![]),
                expire_answers: RefCell::new(vec![]),
            }
        }

        pub fn get_string(&self, _key: &str) -> RedisResult<String> {
            if self.get_string_answers.borrow().len() > 0 {
                return self.get_string_answers.borrow_mut().remove(0);
            }
            panic!("unexpected get_string call");
        }

        pub fn get_bool(&self, _key: &str) -> RedisResult<bool> {
            if self.get_bool_answers.borrow().len() > 0 {
                return self.get_bool_answers.borrow_mut().remove(0);
            }
            panic!("unexpected get_bool call");
        }

        pub fn exists(&self, _key: &str) -> RedisResult<bool> {
            if self.exists_answers.borrow().len() > 0 {
                return self.exists_answers.borrow_mut().remove(0);
            }
            panic!("unexpected exists call");
        }

        pub fn set(&self, _key: &str, _value: &str) -> RedisResult<()> {
            if self.set_answers.borrow().len() > 0 {
                return self.set_answers.borrow_mut().remove(0);
            }
            panic!("unexpected set call");
        }

        pub fn increment(&self, _key: &str) -> RedisResult<i64> {
            if self.incr_answers.borrow().len() > 0 {
                return self.incr_answers.borrow_mut().remove(0);
            }
            panic!("unexpected increment call");
        }

        pub fn expire(&self, _key: &str, _seconds: usize) -> RedisResult<()> {
            if self.expire_answers.borrow().len() > 0 {
                return self.expire_answers.borrow_mut().remove(0);
            }
            panic!("unexpected expire call");
        }
    }

    #[test]
    fn test_lookup() {
        let redis = StubRedisFacade::new();
        &redis
            .get_string_answers
            .borrow_mut()
            .push(Ok(String::from("test url")));

        let shortener = Shortener::new(10, vec!['a', 'b', 'c'], 10, redis, 600, 10);
        assert_eq!(shortener.lookup("id").unwrap(), "test url");
    }

    #[test]
    fn test_shorten_happy_path_first_call() {
        let redis = StubRedisFacade::new();
        // api key verification
        &redis.get_bool_answers.borrow_mut().push(Ok(true));
        &redis.exists_answers.borrow_mut().push(Ok(false));
        &redis.incr_answers.borrow_mut().push(Ok(1));
        &redis.expire_answers.borrow_mut().push(Ok(()));

        // id generation
        &redis.exists_answers.borrow_mut().push(Ok(false));

        // shortened url storage
        &redis.set_answers.borrow_mut().push(Ok(()));

        let shortener = Shortener::new(10, vec!['a', 'b', 'c'], 10, redis, 600, 10);
        let shorten_result = shortener
            .shorten(&Some("api key"), Some("with.lv"), "example.com")
            .unwrap();
        assert_eq!(10, shorten_result.id.len());
        assert_eq!("http://example.com", shorten_result.url);
    }

    #[test]
    fn test_shorten_happy_path_no_rate_limit() {
        let redis = StubRedisFacade::new();
        // api key verification
        &redis.get_bool_answers.borrow_mut().push(Ok(true));

        // id generation
        &redis.exists_answers.borrow_mut().push(Ok(false));

        // shortened url storage
        &redis.set_answers.borrow_mut().push(Ok(()));

        let shortener = Shortener::new(10, vec!['a', 'b', 'c'], 10, redis, 600, -1);
        let shorten_result = shortener
            .shorten(&Some("api key"), Some("with.lv"), "example.com")
            .unwrap();
        assert_eq!(10, shorten_result.id.len());
        assert_eq!("http://example.com", shorten_result.url);
    }

    #[test]
    fn test_shorten_happy_path_second_call() {
        let redis = StubRedisFacade::new();
        // api key verification
        &redis.get_bool_answers.borrow_mut().push(Ok(true));
        &redis.exists_answers.borrow_mut().push(Ok(true));
        &redis.incr_answers.borrow_mut().push(Ok(2));

        // id generation
        &redis.exists_answers.borrow_mut().push(Ok(false));

        // shortened url storage
        &redis.set_answers.borrow_mut().push(Ok(()));

        let shortener = Shortener::new(10, vec!['a', 'b', 'c'], 10, redis, 600, 10);
        let shorten_result = shortener
            .shorten(&Some("api key"), Some("with.lv"), "example.com")
            .unwrap();
        assert_eq!(10, shorten_result.id.len());
        assert_eq!("http://example.com", shorten_result.url);
    }

    #[test]
    fn test_shorten_happy_path_no_api_key() {
        let redis = StubRedisFacade::new();
        // id generation
        &redis.exists_answers.borrow_mut().push(Ok(false));

        // shortened url storage
        &redis.set_answers.borrow_mut().push(Ok(()));

        let shortener = Shortener::new(10, vec!['a', 'b', 'c'], 10, redis, 600, 10);
        let shorten_result = shortener
            .shorten(&None, Some("with.lv"), "example.com")
            .unwrap();
        assert_eq!(10, shorten_result.id.len());
        assert_eq!("http://example.com", shorten_result.url);
    }

    #[test]
    fn test_shorten_unhappy_path_rate_limit_exceeded() {
        let rate_limit = 10;
        let redis = StubRedisFacade::new();
        // api key verification
        &redis.get_bool_answers.borrow_mut().push(Ok(true));
        &redis.exists_answers.borrow_mut().push(Ok(true));
        &redis.incr_answers.borrow_mut().push(Ok(rate_limit + 1));

        let shortener = Shortener::new(10, vec!['a', 'b', 'c'], 10, redis, 600, rate_limit);
        let shorten_result_err = shortener
            .shorten(&Some("api key"), Some("with.lv"), "example.com")
            .err()
            .unwrap();
        assert_eq!("Rate limit exceeded", shorten_result_err.message);
    }

    #[test]
    fn test_shorten_unhappy_path_bad_url() {
        let redis = StubRedisFacade::new();
        // id generation
        &redis.exists_answers.borrow_mut().push(Ok(false));

        let shortener = Shortener::new(10, vec!['a', 'b', 'c'], 10, redis, 600, -1);
        let shorten_result_err = shortener
            .shorten(&None, Some("with.lv"), "wrong domain.com")
            .err()
            .unwrap();
        assert_eq!("Unable to parse url", shorten_result_err.message);
    }

    #[test]
    fn test_shorten_unhappy_path_same_domain() {
        let redis = StubRedisFacade::new();
        // id generation
        &redis.exists_answers.borrow_mut().push(Ok(false));

        let shortener = Shortener::new(10, vec!['a', 'b', 'c'], 10, redis, 600, -1);
        let shorten_result_err = shortener
            .shorten(&None, Some("example.com"), "example.com")
            .err()
            .unwrap();
        assert_eq!("Link loop is not allowed", shorten_result_err.message);
    }

    #[test]
    fn test_shorten_happy_path_rate_limit_expired() {
        let redis = StubRedisFacade::new();

        // api key verification
        &redis.get_bool_answers.borrow_mut().push(Ok(true));
        &redis.exists_answers.borrow_mut().push(Ok(true));
        &redis.incr_answers.borrow_mut().push(Ok(1));

        // id generation
        &redis.exists_answers.borrow_mut().push(Ok(false));

        // shortened url storage
        &redis.set_answers.borrow_mut().push(Ok(()));

        // api key verification
        &redis.get_bool_answers.borrow_mut().push(Ok(true));
        &redis.exists_answers.borrow_mut().push(Ok(false));
        &redis.incr_answers.borrow_mut().push(Ok(1));
        &redis.expire_answers.borrow_mut().push(Ok(()));

        // id generation
        &redis.exists_answers.borrow_mut().push(Ok(false));

        // shortened url storage
        &redis.set_answers.borrow_mut().push(Ok(()));

        let shortener = Shortener::new(10, vec!['a', 'b', 'c'], 10, redis, 600, 10);

        let shorten_result = shortener
            .shorten(&Some("api key"), Some("with.lv"), "example.com")
            .unwrap();
        assert_eq!(10, shorten_result.id.len());
        assert_eq!("http://example.com", shorten_result.url);

        let shorten_result = shortener
            .shorten(&Some("api key"), Some("with.lv"), "www.wikipedia.org")
            .unwrap();
        assert_eq!(10, shorten_result.id.len());
        assert_eq!("http://www.wikipedia.org", shorten_result.url);
    }

    #[test]
    fn test_shorten_unhappy_path_invalid_api_key() {
        let redis = StubRedisFacade::new();

        // api key verification
        &redis.get_bool_answers.borrow_mut().push(Ok(false));

        let shortener = Shortener::new(10, vec!['a', 'b', 'c'], 10, redis, 600, 10);
        let shorten_result_err = shortener
            .shorten(&Some("api key"), Some("with.lv"), "example.com")
            .err()
            .unwrap();
        assert_eq!("Invalid API key", shorten_result_err.message);
    }

    #[test]
    fn test_shorten_unhappy_path_too_many_attempts_generating_id() {
        let redis = StubRedisFacade::new();

        // id generation attempts
        &redis.exists_answers.borrow_mut().push(Ok(true));
        &redis.exists_answers.borrow_mut().push(Ok(true));

        let shortener = Shortener::new(10, vec!['a', 'b', 'c'], 2, redis, 600, 10);
        let shorten_result_err = shortener
            .shorten(&None, Some("with.lv"), "example.com")
            .err()
            .unwrap();
        assert_eq!(
            "Failed to generate an ID: too many attempts. Consider using a longer ID",
            shorten_result_err.message
        );
    }
}
