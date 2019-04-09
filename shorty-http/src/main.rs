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

use std::env;

use actix_web::http::Method;
use actix_web::middleware::cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{server, App};

use shorty_conf::Config;
use shorty_http::AppState;

fn main() {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or_else(|_| String::from("info")),
    );
    env_logger::init();

    let config = Config::new();
    let host = config.host.clone();
    let port = config.port.clone();

    server::new(move || {
        let app_state = AppState::new(
            &config.redis_host,
            &config.redis_port,
            config.id_length,
            config.rate_limit_period,
            config.rate_limit,
            config.api_key_mandatory,
        );

        App::with_state(app_state)
            .middleware(Logger::default())
            .middleware(Cors::default())
            .route("/{shorty_id}", Method::GET, shorty_http::goto)
            .route("/", Method::POST, shorty_http::shorten)
    })
    .bind(format!("{}:{}", host, port))
    .unwrap()
    .run();
}
