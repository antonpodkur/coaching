use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};

use crate::{bot, codes, state::AppState, telegram::Update};

/// Receives bot updates from Telegram. Not part of the public API.
///
/// Telegram retries an update until it gets a 2xx, so failures that may be
/// temporary (database, Bot API) answer 500, and unreadable updates answer 200.
pub async fn webhook(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> StatusCode {
    let secret = headers
        .get("x-telegram-bot-api-secret-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if !codes::secrets_match(secret, &state.config.webhook_secret) {
        return StatusCode::UNAUTHORIZED;
    }

    let update: Update = match serde_json::from_slice(&body) {
        Ok(update) => update,
        Err(err) => {
            tracing::warn!(%err, "ignoring an unreadable Telegram update");
            return StatusCode::OK;
        }
    };
    match bot::handle_update(&state, update).await {
        Ok(()) => StatusCode::OK,
        Err(err) => {
            tracing::error!(error = ?err, "handling a Telegram update failed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
