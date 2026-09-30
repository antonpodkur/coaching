//! The life of an exercise video: uploading → processing → ready, or failed.
//!
//! Bunny is the source of truth for encoding. Its webhook, the phone saying
//! "upload done", and Dasha looking at the exercise all lead to the same step:
//! ask Bunny's API about the upload and move the exercise along. A replayed or
//! forged webhook therefore cannot mark a video ready.
//! See docs/ARCHITECTURE.md, "Video pipeline".

pub mod stream;

use chrono::{Duration, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

pub use stream::{Encoding, StreamClient, StreamSettings, TUS_ENDPOINT};

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

/// How long an upload signature works. Bunny checks it on every tus request,
/// so it has to outlast a slow upload over gym Wi-Fi.
const UPLOAD_TTL: Duration = Duration::hours(24);
/// At most this many encodings are checked with Bunny per library view.
const REFRESH_LIMIT: i64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "upload_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UploadStatus {
    /// The video exists on Bunny; the phone is sending the file.
    Uploading,
    /// The file arrived; Bunny is encoding it.
    Processing,
    /// Bunny could not take or encode the file.
    Failed,
}

/// What the phone needs to upload one file straight to Bunny.
pub struct UploadGrant {
    pub video_id: String,
    pub expires_at: i64,
    pub signature: String,
}

pub fn stream(state: &AppState) -> AppResult<&StreamClient> {
    state
        .video
        .as_ref()
        .ok_or(AppError::Unavailable("video_not_configured"))
}

/// Creates the Bunny video for a new upload and records it on the exercise. An
/// earlier upload that never finished is dropped, on Bunny too.
pub async fn start_upload(
    state: &AppState,
    coach_id: Uuid,
    exercise_id: Uuid,
) -> AppResult<UploadGrant> {
    let stream = stream(state)?;
    let name = sqlx::query_scalar!(
        "SELECT name FROM exercises WHERE id = $1 AND coach_id = $2 AND archived_at IS NULL",
        exercise_id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let video_id = stream.create_video(&name).await?;
    let replaced = sqlx::query_scalar!(
        r#"WITH old AS (
               SELECT id, upload_video_uid FROM exercises
               WHERE id = $1 AND coach_id = $2 AND archived_at IS NULL
               FOR UPDATE
           )
           UPDATE exercises e
           SET upload_video_uid = $3, upload_status = 'uploading', upload_started_at = now()
           FROM old WHERE e.id = old.id
           RETURNING old.upload_video_uid"#,
        exercise_id,
        coach_id,
        video_id,
    )
    .fetch_optional(&state.db)
    .await?;
    match replaced {
        // Archived while Bunny was creating the video.
        None => {
            delete_quietly(stream, &video_id).await;
            return Err(AppError::NotFound);
        }
        Some(Some(abandoned)) => delete_quietly(stream, &abandoned).await,
        Some(None) => {}
    }

    let expires_at = (Utc::now() + UPLOAD_TTL).timestamp();
    let signature = stream.upload_signature(&video_id, expires_at);
    Ok(UploadGrant {
        video_id,
        expires_at,
        signature,
    })
}

/// The phone finished sending the file, so Bunny is encoding it now.
pub async fn finish_upload(state: &AppState, coach_id: Uuid, exercise_id: Uuid) -> AppResult<()> {
    let upload = sqlx::query_scalar!(
        "UPDATE exercises SET upload_status = 'processing'
         WHERE id = $1 AND coach_id = $2 AND upload_status = 'uploading'
         RETURNING upload_video_uid",
        exercise_id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?
    .flatten();
    match upload {
        // Short clips may already be encoded.
        Some(video_id) => refresh_quietly(state, &video_id).await,
        None => {
            // Already moved on (a webhook was faster), or there is no upload.
            let exists = sqlx::query_scalar!(
                "SELECT id FROM exercises WHERE id = $1 AND coach_id = $2",
                exercise_id,
                coach_id,
            )
            .fetch_optional(&state.db)
            .await?;
            if exists.is_none() {
                return Err(AppError::NotFound);
            }
        }
    }
    Ok(())
}

/// Asks Bunny about an upload and moves its exercise along: to processing,
/// to ready (the new video replaces the old one, which is then deleted), or to
/// failed.
pub async fn refresh(state: &AppState, video_id: &str) -> anyhow::Result<()> {
    let Some(stream) = &state.video else {
        return Ok(());
    };
    let reported = stream.video(video_id).await?;
    let Some(video) = reported else {
        sqlx::query!(
            "UPDATE exercises SET upload_status = 'failed' WHERE upload_video_uid = $1",
            video_id,
        )
        .execute(&state.db)
        .await?;
        return Ok(());
    };

    match video.encoding {
        // The phone is still sending, or Bunny has not registered the end yet.
        Encoding::AwaitingUpload => {}
        Encoding::InProgress => {
            sqlx::query!(
                "UPDATE exercises SET upload_status = 'processing'
                 WHERE upload_video_uid = $1 AND upload_status = 'uploading'",
                video_id,
            )
            .execute(&state.db)
            .await?;
        }
        Encoding::Failed => {
            sqlx::query!(
                "UPDATE exercises SET upload_status = 'failed' WHERE upload_video_uid = $1",
                video_id,
            )
            .execute(&state.db)
            .await?;
        }
        Encoding::Finished => {
            let replaced = sqlx::query_scalar!(
                r#"WITH old AS (
                       SELECT id, video_uid FROM exercises WHERE upload_video_uid = $1 FOR UPDATE
                   )
                   UPDATE exercises e
                   SET video_uid = $1, video_length_secs = $2,
                       upload_video_uid = NULL, upload_status = NULL, upload_started_at = NULL
                   FROM old WHERE e.id = old.id
                   RETURNING old.video_uid"#,
                video_id,
                video.length_secs,
            )
            .fetch_optional(&state.db)
            .await?
            .flatten();
            if let Some(old) = replaced {
                delete_quietly(stream, &old).await;
            }
        }
    }
    Ok(())
}

/// Checks this exercise's encoding with Bunny, if it is encoding.
pub async fn refresh_exercise(state: &AppState, exercise_id: Uuid) -> AppResult<()> {
    let upload = sqlx::query_scalar!(
        "SELECT upload_video_uid FROM exercises WHERE id = $1 AND upload_status = 'processing'",
        exercise_id,
    )
    .fetch_optional(&state.db)
    .await?
    .flatten();
    if let Some(video_id) = upload {
        refresh_quietly(state, &video_id).await;
    }
    Ok(())
}

/// Checks the coach's oldest encodings with Bunny, in case a webhook got lost.
pub async fn refresh_processing(state: &AppState, coach_id: Uuid) -> AppResult<()> {
    if state.video.is_none() {
        return Ok(());
    }
    let uploads = sqlx::query_scalar!(
        r#"SELECT upload_video_uid AS "upload_video_uid!" FROM exercises
           WHERE coach_id = $1 AND upload_status = 'processing' AND archived_at IS NULL
           ORDER BY upload_started_at
           LIMIT $2"#,
        coach_id,
        REFRESH_LIMIT,
    )
    .fetch_all(&state.db)
    .await?;
    for video_id in uploads {
        refresh_quietly(state, &video_id).await;
    }
    Ok(())
}

/// Bunny's webhook named this video: check it if it is one of our uploads.
pub async fn on_webhook(state: &AppState, video_id: &str) -> anyhow::Result<()> {
    let ours = sqlx::query_scalar!(
        "SELECT id FROM exercises WHERE upload_video_uid = $1",
        video_id,
    )
    .fetch_optional(&state.db)
    .await?;
    if ours.is_some() {
        refresh(state, video_id).await?;
    }
    Ok(())
}

/// Bunny being slow or down must not break the screen that asked; the next
/// look, or the webhook, tries again.
async fn refresh_quietly(state: &AppState, video_id: &str) {
    if let Err(err) = refresh(state, video_id).await {
        tracing::warn!(error = ?err, video_id, "could not check a video with Bunny");
    }
}

/// Unused videos cost storage, not correctness; a failed delete is only logged.
async fn delete_quietly(stream: &StreamClient, video_id: &str) {
    if let Err(err) = stream.delete_video(video_id).await {
        tracing::warn!(error = ?err, video_id, "could not delete a Bunny video");
    }
}
