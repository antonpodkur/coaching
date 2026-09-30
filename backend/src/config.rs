use std::net::SocketAddr;

use anyhow::{Context, Result, ensure};
use axum::http::HeaderValue;

/// Settings read from the environment (and `backend/.env` in development).
///
/// Deliberately not `Debug`: it holds the bot token and other secrets.
#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub bot_token: String,
    /// Bot username without `@`, for `t.me` links.
    pub bot_username: String,
    /// Telegram sends it back in every webhook request (`X-Telegram-Bot-Api-Secret-Token`).
    pub webhook_secret: String,
    /// When set, the webhook is registered with Telegram on startup.
    pub telegram_webhook_url: Option<String>,
    pub jwt_secret: String,
    /// Where the frontend is served, e.g. `https://example.com`; used for bot buttons.
    pub frontend_url: String,
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

        // Telegram allows 1-256 characters: letters, digits, `_` and `-`.
        let webhook_secret = var("WEBHOOK_SECRET")?;
        ensure!(
            (16..=256).contains(&webhook_secret.len())
                && webhook_secret
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
            "WEBHOOK_SECRET must be 16-256 characters of A-Z, a-z, 0-9, _ or -"
        );

        let frontend_url = var("FRONTEND_ORIGIN")?.trim_end_matches('/').to_owned();
        let frontend_origin = HeaderValue::from_str(&frontend_url)
            .context("FRONTEND_ORIGIN is not a valid header value")?;

        Ok(Self {
            database_url: var("DATABASE_URL")?,
            bind_addr: var("BIND_ADDR")?
                .parse()
                .context("BIND_ADDR must look like 127.0.0.1:8080")?,
            bot_token: var("BOT_TOKEN")?,
            bot_username: var("BOT_USERNAME")?.trim_start_matches('@').to_owned(),
            webhook_secret,
            telegram_webhook_url: std::env::var("TELEGRAM_WEBHOOK_URL")
                .ok()
                .filter(|url| !url.is_empty()),
            jwt_secret,
            frontend_url,
            frontend_origin,
        })
    }

    /// The Mini App, opened by the bot's buttons.
    pub fn mini_app_url(&self) -> String {
        format!("{}/app", self.frontend_url)
    }

    /// A `t.me` link that opens the bot and sends `/start <payload>`.
    pub fn bot_start_url(&self, payload: &str) -> String {
        format!("https://t.me/{}?start={payload}", self.bot_username)
    }
}

fn var(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} is not set"))
}
