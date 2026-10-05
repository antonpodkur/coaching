use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    auth::jwt::JwtKeys, config::Config, push::PushClient, storage::StorageClient,
    telegram::TelegramClient, video::StreamClient,
};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub jwt: Arc<JwtKeys>,
    pub telegram: TelegramClient,
    /// Bunny Stream; `None` when it is not configured, which turns video uploads off.
    pub video: Option<StreamClient>,
    /// The private library for clients' technique videos; `None` turns them off.
    pub client_videos: Option<StreamClient>,
    /// The private photo storage; `None` turns photos off.
    pub storage: Option<StorageClient>,
    /// Web push from the installed app; `None` turns it off.
    pub push: Option<PushClient>,
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
            client_videos: None,
            storage: None,
            push: None,
        }
    }

    pub fn with_video(mut self, video: Option<StreamClient>) -> Self {
        self.video = video;
        self
    }

    pub fn with_client_videos(mut self, library: Option<StreamClient>) -> Self {
        self.client_videos = library;
        self
    }

    pub fn with_storage(mut self, storage: Option<StorageClient>) -> Self {
        self.storage = storage;
        self
    }

    pub fn with_push(mut self, push: Option<PushClient>) -> Self {
        self.push = push;
        self
    }
}
