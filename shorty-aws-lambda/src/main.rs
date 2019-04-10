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

use std::env;
use std::error::Error;
use std::str::FromStr;

use http::{Method, StatusCode};
use lambda_http::{lambda, Body, Request, Response};
use lambda_runtime::error::HandlerError;
use lambda_runtime::Context;
use redis::Client;

use shorty::redis_facade::RedisFacade;
use shorty::Shortener;
use shorty_conf::Config;

fn main() -> Result<(), Box<dyn Error>> {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or_else(|_| String::from("info")),
    );
    env_logger::init();
    lambda!(handler);

    Ok(())
}

fn goto(shortener: &mut Shortener, key: &str) -> Result<Response<Body>, HandlerError> {
    log::trace!("resolving key '{}'", key);

    match shortener.lookup(key) {
        Some(url) => {
            log::trace!("Url found {}", url);

            Ok(Response::builder()
                .status(StatusCode::FOUND)
                .header("Location", url)
                .body(Body::Empty)
                .expect("failed to render 302 response"))
        }
        None => {
            log::trace!("NO Url found");

            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::Empty)
                .expect("failed to render 404 response"))
        }
    }
}

#[derive(Serialize)]
struct ShortenerError {
    err: String,
}

fn shorten(
    shortener: &mut Shortener,
    api_key_mandatory: bool,
    api_key: &Option<String>,
    url: &str,
) -> Result<Response<Body>, HandlerError> {
    if api_key.is_none() && api_key_mandatory {
        return Ok(Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::Text(
                serde_json::to_string(&ShortenerError {
                    err: String::from("Missing API key"),
                })
                .unwrap(),
            ))
            .expect("failed to render response"));
    }

    let api_key = &api_key.as_ref().map(String::as_str);

    match shortener.shorten(api_key, url) {
        Ok(shorten_result) => Ok(Response::builder()
            .body(Body::Text(serde_json::to_string(&shorten_result).unwrap()))
            .expect("failed to render response")),

        Err(err) => Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::Text(
                serde_json::to_string(&ShortenerError {
                    err: err.to_string(),
                })
                .unwrap(),
            ))
            .expect("failed to render response")),
    }
}

#[derive(Deserialize)]
struct ShortenRequest {
    api_key: Option<String>,
    url: String,
}

impl FromStr for ShortenRequest {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

fn handler(e: Request, _c: Context) -> Result<Response<Body>, HandlerError> {
    let config = Config::new();

    let redis =
        Client::open(format!("redis://{}:{}/", config.redis_host, config.redis_port).as_str())
            .unwrap()
            .get_connection()
            .unwrap();

    let mut shortener = Shortener::new(
        config.id_length,
        config.id_alphabet,
        config.id_generation_max_attempts,
        RedisFacade::new(redis),
        config.rate_limit_period,
        config.rate_limit,
    );

    let path = e.uri().path().split('/').last();

    match (path, e.method(), e.body()) {
        (Some(key), &Method::GET, Body::Empty) => goto(&mut shortener, key),
        (Some(""), &Method::POST, Body::Text(body)) => {
            let shorten_request = body.parse::<ShortenRequest>().unwrap();
            shorten(
                &mut shortener,
                config.api_key_mandatory,
                &shorten_request.api_key,
                &shorten_request.url,
            )
        }
        _ => {
            log::error!(
                "unable to handle path {:?} and method {:?}",
                path,
                e.method()
            );
            Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::Empty)
                .expect("failed to render response"))
        }
    }
}
