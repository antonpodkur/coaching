use std::sync::Arc;

use sqlx::PgPool;

use crate::{auth::jwt::JwtKeys, config::Config};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub jwt: Arc<JwtKeys>,
}

impl AppState {
    pub fn new(db: PgPool, config: Config) -> Self {
        let jwt = JwtKeys::new(&config.jwt_secret);
        Self {
            db,
            config: Arc::new(config),
            jwt: Arc::new(jwt),
        }
    }
}
