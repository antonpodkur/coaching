//! The questionnaire a client fills in for Dasha: birth year, sex and height,
//! and photos and videos of their gym, so she knows what they can train with.
//! Every answer is optional.
//!
//! Photos are shrunk on the phone, sent through the backend and kept in the
//! private storage zone. Videos go straight from the phone to the private
//! client video library, like technique videos, and Bunny encodes them. Both
//! are shown only through signed links that expire.

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    cdn_token,
    error::{AppError, AppResult},
    form_videos::{self, FormVideoStatus, UPLOAD_TTL, Viewer},
    photos,
    state::AppState,
    video::{Encoding, UploadGrant},
};

pub const MAX_PHOTOS: i64 = 10;
pub const MAX_VIDEOS: i64 = 3;
/// At most this many encodings are checked with Bunny per jobs round.
const REFRESH_LIMIT: i64 = 10;
/// Clients are at least this old.
const MIN_AGE: i32 = 10;
const OLDEST_BIRTH_YEAR: i32 = 1930;
const MIN_HEIGHT_CM: i32 = 100;
const MAX_HEIGHT_CM: i32 = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "client_sex", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Sex {
    Female,
    Male,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "gym_media_kind", rename_all = "snake_case")]
enum MediaKind {
    Photo,
    Video,
}

#[derive(Serialize, ToSchema)]
pub struct Questionnaire {
    pub birth_year: Option<i32>,
    pub sex: Option<Sex>,
    pub height_cm: Option<i32>,
    /// Oldest first.
    pub photos: Vec<GymPhoto>,
    /// Oldest first. Dasha sees only the ones that arrived (encoding or ready).
    pub videos: Vec<GymVideo>,
}

#[derive(Serialize, ToSchema)]
pub struct GymPhoto {
    pub id: Uuid,
    /// Signed for a few hours.
    pub url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct GymVideo {
    pub id: Uuid,
    pub status: FormVideoStatus,
    /// HLS playlist, signed for a few hours. Only once ready.
    pub hls_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub length_secs: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// The client's answers; each is optional, and leaving one out clears it.
#[derive(Deserialize, ToSchema)]
pub struct Answers {
    pub birth_year: Option<i32>,
    pub sex: Option<Sex>,
    pub height_cm: Option<i32>,
}

/// A client's questionnaire with fresh links.
pub async fn get(state: &AppState, client_id: Uuid, viewer: Viewer) -> AppResult<Questionnaire> {
    let answers = sqlx::query!(
        r#"SELECT birth_year, sex AS "sex: Sex", height_cm FROM clients WHERE id = $1"#,
        client_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let media = sqlx::query!(
        r#"SELECT id, kind AS "kind: MediaKind", object_key, status AS "status: FormVideoStatus",
                  length_secs, created_at
           FROM gym_media
           WHERE client_id = $1 AND ($2 OR status IN ('processing', 'ready'))
           ORDER BY created_at"#,
        client_id,
        viewer == Viewer::Client,
    )
    .fetch_all(&state.db)
    .await?;

    let expires = cdn_token::link_expiry(Utc::now());
    let mut photos = Vec::new();
    let mut videos = Vec::new();
    for item in media {
        match item.kind {
            // Without the storage settings there is no way to show a photo.
            MediaKind::Photo => {
                if let Some(storage) = &state.storage {
                    photos.push(GymPhoto {
                        id: item.id,
                        url: storage.signed_url(&gym_dir(client_id), &item.object_key, expires),
                        created_at: item.created_at,
                    });
                }
            }
            MediaKind::Video => {
                let links = state
                    .client_videos
                    .as_ref()
                    .filter(|_| item.status == FormVideoStatus::Ready)
                    .map(|stream| {
                        (
                            stream.signed_url(&item.object_key, "playlist.m3u8", expires),
                            stream.signed_url(&item.object_key, "thumbnail.jpg", expires),
                        )
                    });
                videos.push(GymVideo {
                    id: item.id,
                    status: item.status,
                    hls_url: links.as_ref().map(|(hls, _)| hls.clone()),
                    thumbnail_url: links.map(|(_, thumbnail)| thumbnail),
                    length_secs: item.length_secs,
                    created_at: item.created_at,
                });
            }
        }
    }
    Ok(Questionnaire {
        birth_year: answers.birth_year,
        sex: answers.sex,
        height_cm: answers.height_cm,
        photos,
        videos,
    })
}

/// Saves the client's answers, after checking they are plausible.
pub async fn save_answers(state: &AppState, client_id: Uuid, answers: &Answers) -> AppResult<()> {
    let youngest = Utc::now().year() - MIN_AGE;
    if answers
        .birth_year
        .is_some_and(|year| !(OLDEST_BIRTH_YEAR..=youngest).contains(&year))
    {
        return Err(AppError::BadRequest("invalid_birth_year"));
    }
    if answers
        .height_cm
        .is_some_and(|cm| !(MIN_HEIGHT_CM..=MAX_HEIGHT_CM).contains(&cm))
    {
        return Err(AppError::BadRequest("invalid_height"));
    }
    sqlx::query!(
        "UPDATE clients SET birth_year = $2, sex = $3, height_cm = $4 WHERE id = $1",
        client_id,
        answers.birth_year,
        answers.sex as Option<Sex>,
        answers.height_cm,
    )
    .execute(&state.db)
    .await?;
    Ok(())
}

/// Where a client's gym photos live in the storage zone; one signed link
/// covers them all.
fn gym_dir(client_id: Uuid) -> String {
    format!("clients/{client_id}/gym/")
}

/// Stores a photo of the client's gym: a JPEG the phone already shrank.
pub async fn add_photo(state: &AppState, client_id: Uuid, jpeg: Vec<u8>) -> AppResult<GymPhoto> {
    let storage = photos::storage(state)?;
    photos::check(&jpeg)?;
    let count = media_count(state, client_id, MediaKind::Photo).await?;
    if count >= MAX_PHOTOS {
        return Err(AppError::Conflict("too_many_photos"));
    }

    let id = Uuid::new_v4();
    let path = format!("{}{id}.jpg", gym_dir(client_id));
    storage.put(&path, jpeg, "image/jpeg").await?;
    let created_at = sqlx::query_scalar!(
        "INSERT INTO gym_media (id, client_id, kind, object_key, status)
         VALUES ($1, $2, 'photo', $3, 'ready') RETURNING created_at",
        id,
        client_id,
        path,
    )
    .fetch_one(&state.db)
    .await?;
    let expires = cdn_token::link_expiry(Utc::now());
    Ok(GymPhoto {
        id,
        url: storage.signed_url(&gym_dir(client_id), &path, expires),
        created_at,
    })
}

/// Photos, or videos not counting failed uploads.
async fn media_count(state: &AppState, client_id: Uuid, kind: MediaKind) -> sqlx::Result<i64> {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM gym_media
           WHERE client_id = $1 AND kind = $2 AND status <> 'failed'"#,
        client_id,
        kind as MediaKind,
    )
    .fetch_one(&state.db)
    .await
}

/// Creates the Bunny video for a gym video the client is about to send, and
/// signs the upload.
pub async fn start_video(
    state: &AppState,
    client_id: Uuid,
) -> AppResult<(Uuid, String, UploadGrant)> {
    let stream = form_videos::stream(state)?;
    if media_count(state, client_id, MediaKind::Video).await? >= MAX_VIDEOS {
        return Err(AppError::Conflict("too_many_videos"));
    }
    let name = sqlx::query_scalar!("SELECT name FROM clients WHERE id = $1", client_id)
        .fetch_one(&state.db)
        .await?;
    // Shown in Bunny's dashboard; the upload repeats it, or Bunny would rename it.
    let title = format!("{name} · зал");
    let video_id = stream.create_video(&title).await?;
    let id = sqlx::query_scalar!(
        "INSERT INTO gym_media (client_id, kind, object_key, status)
         VALUES ($1, 'video', $2, 'uploading') RETURNING id",
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

/// The phone finished sending a gym video, so Bunny is encoding it now.
pub async fn finish_video(state: &AppState, client_id: Uuid, id: Uuid) -> AppResult<()> {
    let video_id = sqlx::query_scalar!(
        "UPDATE gym_media SET status = 'processing'
         WHERE id = $1 AND client_id = $2 AND kind = 'video'
           AND status IN ('uploading', 'processing')
         RETURNING object_key",
        id,
        client_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    // A short video may already be encoded; otherwise the webhook or jobs follow up.
    if let Err(err) = refresh_video(state, &video_id).await {
        tracing::warn!(error = ?err, "could not check a gym video with Bunny");
    }
    Ok(())
}

/// The client deletes a photo or video of their gym, on Bunny too.
pub async fn delete(state: &AppState, client_id: Uuid, id: Uuid) -> AppResult<()> {
    let item = sqlx::query!(
        r#"DELETE FROM gym_media WHERE id = $1 AND client_id = $2
           RETURNING kind AS "kind: MediaKind", object_key"#,
        id,
        client_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let removed = match item.kind {
        MediaKind::Photo => match &state.storage {
            Some(storage) => storage.delete(&item.object_key).await,
            None => Ok(()),
        },
        MediaKind::Video => match &state.client_videos {
            Some(stream) => stream.delete_video(&item.object_key).await,
            None => Ok(()),
        },
    };
    if let Err(err) = removed {
        tracing::warn!(error = ?err, "could not delete a gym photo or video on Bunny");
    }
    Ok(())
}

/// Asks Bunny where a gym video's encoding is and records it. Nothing to do
/// for a video that is not a gym video in progress.
pub async fn refresh_video(state: &AppState, video_id: &str) -> anyhow::Result<()> {
    let Some(stream) = &state.client_videos else {
        return Ok(());
    };
    let Some(id) = sqlx::query_scalar!(
        "SELECT id FROM gym_media
         WHERE object_key = $1 AND kind = 'video' AND status IN ('uploading', 'processing')",
        video_id,
    )
    .fetch_optional(&state.db)
    .await?
    else {
        return Ok(());
    };
    let (status, length_secs) = match stream.video(video_id).await? {
        Some(video) if video.encoding == Encoding::Finished => {
            (FormVideoStatus::Ready, Some(video.length_secs))
        }
        Some(video) if video.encoding == Encoding::Failed => (FormVideoStatus::Failed, None),
        Some(_) => return Ok(()),
        // Gone from Bunny.
        None => (FormVideoStatus::Failed, None),
    };
    sqlx::query!(
        "UPDATE gym_media SET status = $2, length_secs = $3 WHERE id = $1",
        id,
        status as FormVideoStatus,
        length_secs,
    )
    .execute(&state.db)
    .await?;
    Ok(())
}

/// Encodings to check this round, for when no webhook arrives.
pub async fn refresh_processing(state: &AppState) -> anyhow::Result<()> {
    if state.client_videos.is_none() {
        return Ok(());
    }
    let pending = sqlx::query_scalar!(
        "SELECT object_key FROM gym_media
         WHERE kind = 'video' AND status = 'processing' ORDER BY created_at LIMIT $1",
        REFRESH_LIMIT,
    )
    .fetch_all(&state.db)
    .await?;
    for video_id in pending {
        if let Err(err) = refresh_video(state, &video_id).await {
            tracing::warn!(error = ?err, "could not check a gym video with Bunny");
        }
    }
    Ok(())
}
