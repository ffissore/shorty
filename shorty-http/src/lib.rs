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

use std::cell::RefCell;

use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse, Json, Path};
use redis::Client;

use shorty::redis_facade::RedisFacade;
use shorty::Shortener;

pub struct AppState {
    shortener: RefCell<Shortener>,
    api_key_mandatory: bool,
}

impl AppState {
    pub fn new(
        redis_host: &str,
        redis_port: &str,
        id_length: usize,
        rate_limit_period: usize,
        rate_limit: i64,
        api_key_mandatory: bool,
    ) -> AppState {
        let redis = Client::open(format!("redis://{}:{}/", redis_host, redis_port).as_str())
            .unwrap()
            .get_connection()
            .unwrap();

        let alphabet = vec![
            (b'a'..=b'z').map(char::from).collect::<Vec<_>>(),
            (b'A'..=b'Z').map(char::from).collect::<Vec<_>>(),
            (b'0'..=b'9').map(char::from).collect::<Vec<_>>(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<char>>();

        AppState {
            shortener: RefCell::new(Shortener::new(
                id_length,
                alphabet,
                RedisFacade::new(redis),
                rate_limit_period,
                rate_limit,
            )),
            api_key_mandatory,
        }
    }
}

pub fn goto((req, id): (HttpRequest<AppState>, Path<String>)) -> HttpResponse {
    let app_state: &AppState = &req.state();

    match app_state.shortener.borrow_mut().lookup(&id) {
        Some(url) => HttpResponse::Found().header("Location", url).finish(),
        None => HttpResponse::NotFound().finish(),
    }
}

#[derive(Deserialize)]
pub struct ShortenRequest {
    api_key: Option<String>,
    url: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    err: String,
}

pub fn shorten((req, payload): (HttpRequest<AppState>, Json<ShortenRequest>)) -> HttpResponse {
    let app_state: &AppState = &req.state();

    if payload.api_key.is_none() && app_state.api_key_mandatory {
        return HttpResponse::Ok()
            .status(StatusCode::FORBIDDEN)
            .json(ErrorResponse {
                err: String::from("Missing API key"),
            });
    }

    let api_key = payload.api_key.as_ref().map(String::as_str);

    match app_state
        .shortener
        .borrow_mut()
        .shorten(&api_key, &payload.url)
    {
        Ok(shorten_result) => HttpResponse::Ok().json(shorten_result),
        Err(err) => HttpResponse::InternalServerError().json(ErrorResponse {
            err: err.to_string(),
        }),
    }
}
