use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub redis_host: String,
    pub redis_port: String,
    pub rate_limit_period: usize,
    pub rate_limit: i64,
    pub id_length: usize,
    pub id_alphabet: Vec<char>,
    pub api_key_mandatory: bool,
    pub host: String,
    pub port: String,
}

impl Config {
    pub fn new() -> Config {
        let redis_host =
            env::var("SHORTENER_REDIS_HOST").unwrap_or_else(|_| String::from("127.0.0.1"));
        let redis_port = env::var("SHORTENER_REDIS_PORT").unwrap_or_else(|_| String::from("6379"));

        let rate_limit_period = env::var("SHORTENER_RATE_LIMIT_PERIOD")
            .unwrap_or_else(|_| String::from("600"))
            .parse::<usize>()
            .unwrap();
        let rate_limit = env::var("SHORTENER_RATE_LIMIT")
            .unwrap_or_else(|_| String::from("10"))
            .parse::<i64>()
            .unwrap();

        let id_length = env::var("SHORTENER_ID_LENGTH")
            .unwrap_or_else(|_| String::from("10"))
            .parse::<usize>()
            .unwrap();
        let id_alphabet = vec![
            (b'a'..=b'z').map(char::from).collect::<Vec<_>>(),
            (b'A'..=b'Z').map(char::from).collect::<Vec<_>>(),
            (b'0'..=b'9').map(char::from).collect::<Vec<_>>(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<char>>();

        let host = env::var("SHORTENER_HOST").unwrap_or_else(|_| String::from("127.0.0.1"));
        let port = env::var("SHORTENER_PORT").unwrap_or_else(|_| String::from("8088"));

        let api_key_mandatory = env::var("SHORTENER_API_KEY_MANDATORY")
            .unwrap_or_else(|_| String::from("true"))
            .parse::<bool>()
            .unwrap();

        Config {
            redis_host,
            redis_port,
            rate_limit_period,
            rate_limit,
            id_length,
            id_alphabet,
            api_key_mandatory,
            host,
            port,
        }
    }
}
