use std::net::SocketAddr;

use anyhow::{Context, Result, ensure};
use axum::http::HeaderValue;

use crate::video::StreamSettings;

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
    /// Dasha's Telegram account; her coach profile is created on startup.
    pub coach_telegram_id: Option<i64>,
    pub jwt_secret: String,
    /// Where the frontend is served, e.g. `https://example.com`; used for bot buttons.
    pub frontend_url: String,
    /// The only origin CORS allows: the frontend.
    pub frontend_origin: HeaderValue,
    /// Bunny Stream; without it the library works but videos cannot be uploaded.
    pub bunny: Option<StreamSettings>,
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

        let bunny = match optional_var("BUNNY_STREAM_LIBRARY_ID") {
            None => None,
            Some(library_id) => Some(StreamSettings {
                library_id,
                api_key: var("BUNNY_STREAM_API_KEY")?,
                read_only_api_key: optional_var("BUNNY_STREAM_READ_ONLY_API_KEY"),
                cdn_hostname: var("BUNNY_CDN_HOSTNAME")?
                    .trim_start_matches("https://")
                    .trim_end_matches('/')
                    .to_owned(),
            }),
        };

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
            // On Render the service's own URL is known, so the webhook needs no setting.
            telegram_webhook_url: optional_var("TELEGRAM_WEBHOOK_URL").or_else(|| {
                optional_var("RENDER_EXTERNAL_URL")
                    .map(|url| format!("{}/telegram/webhook", url.trim_end_matches('/')))
            }),
            coach_telegram_id: optional_var("COACH_TELEGRAM_ID")
                .map(|id| id.trim().parse())
                .transpose()
                .context("COACH_TELEGRAM_ID must be a Telegram user ID, digits only")?,
            jwt_secret,
            frontend_url,
            frontend_origin,
            bunny,
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

/// Unset and empty both mean "not configured".
fn optional_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}
