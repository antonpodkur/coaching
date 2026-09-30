use std::sync::Arc;

use sqlx::PgPool;

use crate::{auth::jwt::JwtKeys, config::Config, telegram::TelegramClient, video::StreamClient};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub jwt: Arc<JwtKeys>,
    pub telegram: TelegramClient,
    /// Bunny Stream; `None` when it is not configured, which turns video uploads off.
    pub video: Option<StreamClient>,
}

impl AppState {
    pub fn new(db: PgPool, config: Config, telegram: TelegramClient) -> Self {
        let jwt = JwtKeys::new(&config.jwt_secret);
        Self {
            db,
            config: Arc::new(config),
            jwt: Arc::new(jwt),
            telegram,
            video: None,
        }
    }

    pub fn with_video(mut self, video: Option<StreamClient>) -> Self {
        self.video = video;
        self
    }
}
