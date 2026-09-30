use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;

use crate::{state::AppState, video};

/// Bunny's notice that a video changed state. Its numbers differ from the API's
/// statuses, and it only prompts a check with the API, so `Status` is not read.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StreamEvent {
    video_library_id: i64,
    video_guid: String,
}

/// Receives Bunny Stream webhooks. Not part of the public API.
///
/// Failures that may be temporary answer 500 so Bunny can retry; anything that
/// is not about one of our uploads answers 200.
pub async fn webhook(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> StatusCode {
    let Some(stream) = &state.video else {
        return StatusCode::NOT_FOUND;
    };
    if !stream.webhook_is_authentic(&headers, &body) {
        return StatusCode::UNAUTHORIZED;
    }
    let event: StreamEvent = match serde_json::from_slice(&body) {
        Ok(event) => event,
        Err(err) => {
            tracing::warn!(%err, "ignoring an unreadable Bunny webhook");
            return StatusCode::OK;
        }
    };
    if event.video_library_id.to_string() != stream.settings().library_id {
        return StatusCode::OK;
    }
    match video::on_webhook(&state, &event.video_guid).await {
        Ok(()) => StatusCode::OK,
        Err(err) => {
            tracing::error!(error = ?err, "handling a Bunny webhook failed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
