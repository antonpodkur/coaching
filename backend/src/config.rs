use std::net::SocketAddr;

use anyhow::{Context, Result, ensure};
use axum::http::HeaderValue;

/// Settings read from the environment (and `backend/.env` in development).
///
/// Deliberately not `Debug`: it holds the bot token and the JWT secret.
#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub bot_token: String,
    pub jwt_secret: String,
    /// The only origin CORS allows: the frontend.
    pub frontend_origin: HeaderValue,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let jwt_secret = var("JWT_SECRET")?;
        ensure!(
            jwt_secret.len() >= 32,
            "JWT_SECRET must be at least 32 characters"
        );

        Ok(Self {
            database_url: var("DATABASE_URL")?,
            bind_addr: var("BIND_ADDR")?
                .parse()
                .context("BIND_ADDR must look like 127.0.0.1:8080")?,
            bot_token: var("BOT_TOKEN")?,
            jwt_secret,
            frontend_origin: HeaderValue::from_str(&var("FRONTEND_ORIGIN")?)
                .context("FRONTEND_ORIGIN is not a valid header value")?,
        })
    }
}

fn var(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} is not set"))
}
