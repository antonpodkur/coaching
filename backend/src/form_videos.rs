//! Clients' own videos of how they did an exercise ("form checks"), for Dasha
//! to look at their technique. One workout exercise takes up to three.
//!
//! They live in a separate, private Bunny library: its files are only served
//! through signed links that expire, so a forwarded or leaked link stops
//! working within hours. Like exercise videos, Bunny is the source of truth for
//! encoding: the webhook, the phone's "upload done" and the jobs all just ask
//! Bunny's API and move the video along.

use std::collections::HashMap;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    cdn_token,
    error::{AppError, AppResult},
    notify::{self, Kind},
    state::AppState,
    video::{Encoding, StreamClient, UploadGrant},
};

/// Per exercise in one workout, not counting failed uploads.
pub const MAX_PER_EXERCISE: i64 = 3;
/// How long an upload signature works; it has to outlast a slow upload.
pub(crate) const UPLOAD_TTL: Duration = Duration::hours(24);
/// At most this many encodings are checked with Bunny per jobs round.
const REFRESH_LIMIT: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "form_video_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FormVideoStatus {
    /// The phone is sending the file.
    Uploading,
    /// The file arrived; Bunny is encoding it.
    Processing,
    Ready,
    /// Bunny could not take or encode the file.
    Failed,
}

#[derive(Serialize, ToSchema)]
pub struct FormVideo {
    pub id: Uuid,
    pub status: FormVideoStatus,
    /// HLS playlist, signed for a few hours. Only once ready.
    pub hls_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub length_secs: Option<i32>,
    pub created_at: DateTime<Utc>,
    /// Dasha has opened the report since it was ready.
    pub seen: bool,
}

pub fn stream(state: &AppState) -> AppResult<&StreamClient> {
    state
        .client_videos
        .as_ref()
        .ok_or(AppError::Unavailable("client_videos_not_configured"))
}

/// Creates the Bunny video for a client's new upload under one exercise of
/// their own published workout, and signs the upload.
pub async fn start(
    state: &AppState,
    client_id: Uuid,
    workout_exercise_id: Uuid,
) -> AppResult<(Uuid, String, UploadGrant)> {
    let stream = stream(state)?;
    let target = sqlx::query!(
        r#"SELECT c.name AS client_name, e.name AS exercise_name, w.date,
                  (SELECT count(*) FROM form_videos v
                   WHERE v.workout_exercise_id = we.id AND v.status <> 'failed') AS "videos!"
           FROM workout_exercises we
           JOIN workouts w ON w.id = we.workout_id
           JOIN exercises e ON e.id = we.exercise_id
           JOIN clients c ON c.id = w.client_id
           WHERE we.id = $1 AND w.client_id = $2 AND w.status IN ('published', 'done')"#,
        workout_exercise_id,
        client_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    if target.videos >= MAX_PER_EXERCISE {
        return Err(AppError::Conflict("too_many_videos"));
    }

    let date = target.date.map(|date| date.to_string()).unwrap_or_default();
    // Shown in Bunny's dashboard; the upload repeats it, or Bunny would rename it.
    let title = format!("{} · {} · {date}", target.client_name, target.exercise_name);
    let video_id = stream.create_video(&title).await?;
    let id = sqlx::query_scalar!(
        "INSERT INTO form_videos (workout_exercise_id, client_id, video_uid)
         VALUES ($1, $2, $3) RETURNING id",
        workout_exercise_id,
        client_id,
        video_id,
    )
    .fetch_one(&state.db)
    .await?;

    let expires_at = (Utc::now() + UPLOAD_TTL).timestamp();
    let signature = stream.upload_signature(&video_id, expires_at);
    Ok((
        id,
        title,
        UploadGrant {
            video_id,
            expires_at,
            signature,
        },
    ))
}

/// The phone finished sending the file, so Bunny is encoding it now.
pub async fn finish(state: &AppState, client_id: Uuid, id: Uuid) -> AppResult<()> {
    let video_uid = sqlx::query_scalar!(
        "UPDATE form_videos SET status = 'processing'
         WHERE id = $1 AND client_id = $2 AND status IN ('uploading', 'processing')
         RETURNING video_uid",
        id,
        client_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    // A short video may already be encoded; otherwise the webhook or jobs follow up.
    if let Err(err) = refresh(state, &video_uid, Utc::now()).await {
        tracing::warn!(error = ?err, "could not check a client video with Bunny");
    }
    Ok(())
}

/// A client deletes one of their videos, on Bunny too.
pub async fn delete(state: &AppState, client_id: Uuid, id: Uuid) -> AppResult<()> {
    let video_uid = sqlx::query_scalar!(
        "DELETE FROM form_videos WHERE id = $1 AND client_id = $2 RETURNING video_uid",
        id,
        client_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    if let Ok(stream) = stream(state)
        && let Err(err) = stream.delete_video(&video_uid).await
    {
        tracing::warn!(error = ?err, "could not delete a client video on Bunny");
    }
    Ok(())
}

/// Asks Bunny where a video's encoding is and records it. Once ready, Dasha is told.
pub async fn refresh(state: &AppState, video_uid: &str, now: DateTime<Utc>) -> anyhow::Result<()> {
    let stream = stream(state)?;
    let Some(current) = sqlx::query!(
        r#"SELECT v.id, w.date AS "date?"
           FROM form_videos v
           JOIN workout_exercises we ON we.id = v.workout_exercise_id
           JOIN workouts w ON w.id = we.workout_id
           WHERE v.video_uid = $1 AND v.status IN ('uploading', 'processing')"#,
        video_uid,
    )
    .fetch_optional(&state.db)
    .await?
    else {
        return Ok(());
    };
    let encoding = stream.video(video_uid).await?;
    match encoding {
        Some(video) if video.encoding == Encoding::Finished => {
            sqlx::query!(
                "UPDATE form_videos SET status = 'ready', ready_at = $2, length_secs = $3
                 WHERE id = $1",
                current.id,
                now,
                video.length_secs,
            )
            .execute(&state.db)
            .await?;
            let date = current.date.unwrap_or_else(|| now.date_naive());
            announce(state, current.id, date, now).await;
        }
        Some(video) if video.encoding == Encoding::Failed => mark_failed(state, current.id).await?,
        Some(_) => {}
        // Gone from Bunny.
        None => mark_failed(state, current.id).await?,
    }
    Ok(())
}

async fn mark_failed(state: &AppState, id: Uuid) -> sqlx::Result<()> {
    sqlx::query!("UPDATE form_videos SET status = 'failed' WHERE id = $1", id)
        .execute(&state.db)
        .await?;
    Ok(())
}

/// Tells Dasha about a ready video; a failed send is retried by the jobs.
async fn announce(state: &AppState, id: Uuid, date: NaiveDate, now: DateTime<Utc>) {
    match notify::queue(state, Kind::FormVideo, id, date).await {
        Ok(Some(notification)) => {
            if let Err(err) = notify::deliver(state, notification, now).await {
                tracing::warn!(error = ?err, "could not tell Dasha about a client video; will retry");
            }
        }
        Ok(None) => {}
        Err(err) => tracing::warn!(error = ?err, "could not queue the client video message"),
    }
}

/// Encodings to check this round, for when no webhook arrives.
pub async fn refresh_processing(state: &AppState, now: DateTime<Utc>) -> anyhow::Result<()> {
    if state.client_videos.is_none() {
        return Ok(());
    }
    let pending = sqlx::query_scalar!(
        "SELECT video_uid FROM form_videos
         WHERE status = 'processing' ORDER BY created_at LIMIT $1",
        REFRESH_LIMIT,
    )
    .fetch_all(&state.db)
    .await?;
    for video_uid in pending {
        if let Err(err) = refresh(state, &video_uid, now).await {
            tracing::warn!(error = ?err, "could not check a client video with Bunny");
        }
    }
    Ok(())
}

/// Who is looking: a client sees all of their uploads, Dasha the ones that
/// arrived (encoding or ready).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Viewer {
    Client,
    Coach,
}

/// A workout's videos by workout exercise, oldest first, with fresh links.
pub async fn for_workout(
    state: &AppState,
    workout_id: Uuid,
    viewer: Viewer,
) -> sqlx::Result<HashMap<Uuid, Vec<FormVideo>>> {
    let rows = sqlx::query!(
        r#"SELECT v.id, v.workout_exercise_id, v.video_uid, v.status AS "status: FormVideoStatus",
                  v.length_secs, v.created_at, v.seen_at IS NOT NULL AS "seen!"
           FROM form_videos v
           JOIN workout_exercises we ON we.id = v.workout_exercise_id
           WHERE we.workout_id = $1
             AND ($2 OR v.status IN ('processing', 'ready'))
           ORDER BY v.created_at"#,
        workout_id,
        viewer == Viewer::Client,
    )
    .fetch_all(&state.db)
    .await?;

    let expires = cdn_token::link_expiry(Utc::now());
    let mut by_exercise: HashMap<Uuid, Vec<FormVideo>> = HashMap::new();
    for row in rows {
        let links = state
            .client_videos
            .as_ref()
            .filter(|_| row.status == FormVideoStatus::Ready)
            .map(|stream| {
                (
                    stream.signed_url(&row.video_uid, "playlist.m3u8", expires),
                    stream.signed_url(&row.video_uid, "thumbnail.jpg", expires),
                )
            });
        by_exercise
            .entry(row.workout_exercise_id)
            .or_default()
            .push(FormVideo {
                id: row.id,
                status: row.status,
                hls_url: links.as_ref().map(|(hls, _)| hls.clone()),
                thumbnail_url: links.map(|(_, thumbnail)| thumbnail),
                length_secs: row.length_secs,
                created_at: row.created_at,
                seen: row.seen,
            });
    }
    Ok(by_exercise)
}
