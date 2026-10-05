use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;

use chrono::Utc;

use crate::{
    form_videos, questionnaire,
    state::AppState,
    video::{self, StreamClient},
};

/// Bunny's notice that a video changed state. Its numbers differ from the API's
/// statuses, and it only prompts a check with the API, so `Status` is not read.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StreamEvent {
    video_library_id: i64,
    video_guid: String,
}

/// Receives Bunny Stream webhooks for the exercise library. Not part of the
/// public API.
///
/// Failures that may be temporary answer 500 so Bunny can retry; anything that
/// is not about one of our uploads answers 200.
pub async fn webhook(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> StatusCode {
    let Some(stream) = &state.video else {
        return StatusCode::NOT_FOUND;
    };
    let video_guid = match accept(stream, &headers, &body) {
        Ok(Some(guid)) => guid,
        Ok(None) => return StatusCode::OK,
        Err(status) => return status,
    };
    match video::on_webhook(&state, &video_guid).await {
        Ok(()) => StatusCode::OK,
        Err(err) => {
            tracing::error!(error = ?err, "handling a Bunny webhook failed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

/// The same for the private library of clients' own videos: technique videos
/// and gym videos from the questionnaire.
pub async fn client_videos_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let Some(stream) = &state.client_videos else {
        return StatusCode::NOT_FOUND;
    };
    let video_guid = match accept(stream, &headers, &body) {
        Ok(Some(guid)) => guid,
        Ok(None) => return StatusCode::OK,
        Err(status) => return status,
    };
    let refreshed = match form_videos::refresh(&state, &video_guid, Utc::now()).await {
        Ok(()) => questionnaire::refresh_video(&state, &video_guid).await,
        Err(err) => Err(err),
    };
    match refreshed {
        Ok(()) => StatusCode::OK,
        Err(err) => {
            tracing::error!(error = ?err, "handling a client video webhook failed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

/// Checks the signature and the library. The video it is about, or `None` for
/// anything to ignore.
fn accept(
    stream: &StreamClient,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Option<String>, StatusCode> {
    if !stream.webhook_is_authentic(headers, body) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let event: StreamEvent = match serde_json::from_slice(body) {
        Ok(event) => event,
        Err(err) => {
            tracing::warn!(%err, "ignoring an unreadable Bunny webhook");
            return Ok(None);
        }
    };
    if event.video_library_id.to_string() != stream.settings().library_id {
        return Ok(None);
    }
    Ok(Some(event.video_guid))
}
