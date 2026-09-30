use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Serialize, ToSchema)]
pub struct Health {
    pub status: String,
    pub database: bool,
}

/// Liveness check for the hosting platform.
#[utoipa::path(get, path = "/health", tag = "system", responses((status = 200, body = Health)))]
pub async fn health(State(state): State<AppState>) -> Json<Health> {
    let database = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();
    Json(Health {
        status: "ok".to_owned(),
        database,
    })
}
