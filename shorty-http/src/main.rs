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
