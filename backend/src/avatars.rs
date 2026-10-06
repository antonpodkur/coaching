//! Avatars. A client's is a photo they add in their profile, or else a copy
//! of their Telegram profile photo; a coach's is a photo she adds, which her
//! clients see. All live in the private photo storage and are shown through
//! signed links; Telegram's own file links carry the bot token, so its photos
//! are copied, never linked.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    cdn_token,
    error::{AppError, AppResult},
    photos,
    state::AppState,
    storage::StorageClient,
};

/// A Telegram photo is looked at again after this long, for a new or removed one.
const TELEGRAM_RECHECK_DAYS: i64 = 7;
/// After Telegram failed, it is tried again after this long.
const TELEGRAM_RETRY_HOURS: i64 = 24;
/// Clients looked at per background round, to go easy on Telegram.
const BATCH: i64 = 10;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Avatar {
    /// Signed for a few hours.
    pub url: String,
    /// A copy of their Telegram profile photo, not one they added here.
    pub from_telegram: bool,
}

fn avatar_dir(client_id: Uuid) -> String {
    format!("clients/{client_id}/avatar/")
}

/// A signed link to the avatar stored at `path`; `None` without a path or storage.
pub fn signed_url(state: &AppState, client_id: Uuid, path: Option<&str>) -> Option<String> {
    let storage = state.storage.as_ref()?;
    let expires = cdn_token::link_expiry(Utc::now());
    Some(storage.signed_url(&avatar_dir(client_id), path?, expires))
}

/// The avatar stored at `path`, as the client's profile shows it.
pub fn avatar(
    state: &AppState,
    client_id: Uuid,
    path: Option<&str>,
    from_telegram: bool,
) -> Option<Avatar> {
    signed_url(state, client_id, path).map(|url| Avatar { url, from_telegram })
}

/// Deletes an avatar file that is no longer used; a leftover only costs storage.
async fn forget(storage: &StorageClient, path: Option<String>) {
    if let Some(path) = path
        && let Err(err) = storage.delete(&path).await
    {
        tracing::warn!(error = ?err, "could not delete an old avatar");
    }
}

/// The client's own photo, a square JPEG the phone already shrank. It replaces
/// whatever was there, a Telegram copy included, and Telegram never replaces it.
pub async fn upload(state: &AppState, client_id: Uuid, jpeg: Vec<u8>) -> AppResult<Avatar> {
    let storage = photos::storage(state)?;
    photos::check(&jpeg)?;
    let path = format!("{}{}.jpg", avatar_dir(client_id), Uuid::new_v4());
    storage.put(&path, jpeg, "image/jpeg").await?;
    // Joined with itself, the row gives back the path it had before.
    let old = sqlx::query_scalar!(
        "UPDATE clients c SET avatar_path = $2, avatar_from_telegram = false
         FROM clients old
         WHERE c.id = $1 AND old.id = c.id
         RETURNING old.avatar_path",
        client_id,
        path,
    )
    .fetch_one(&state.db)
    .await?;
    forget(storage, old).await;
    avatar(state, client_id, Some(&path), false).ok_or(AppError::NotFound)
}

/// Removes the client's own photo. Their Telegram photo, if bots may see it,
/// comes back at the next background round.
pub async fn remove(state: &AppState, client_id: Uuid) -> AppResult<()> {
    let old = sqlx::query_scalar!(
        "UPDATE clients c
         SET avatar_path = NULL, avatar_from_telegram = false,
             telegram_photo_id = NULL, telegram_photo_checked_at = NULL
         FROM clients old
         WHERE c.id = $1 AND old.id = c.id
         RETURNING old.avatar_path",
        client_id,
    )
    .fetch_one(&state.db)
    .await?;
    if let Some(storage) = &state.storage {
        forget(storage, old).await;
    }
    Ok(())
}

fn coach_avatar_dir(coach_id: Uuid) -> String {
    format!("coaches/{coach_id}/avatar/")
}

/// A signed link to the coach's photo stored at `path`; `None` without one.
pub fn coach_url(state: &AppState, coach_id: Uuid, path: Option<&str>) -> Option<String> {
    let storage = state.storage.as_ref()?;
    let expires = cdn_token::link_expiry(Utc::now());
    Some(storage.signed_url(&coach_avatar_dir(coach_id), path?, expires))
}

/// The coach's photo, a square JPEG the phone already shrank, replacing hers.
pub async fn upload_coach(state: &AppState, coach_id: Uuid, jpeg: Vec<u8>) -> AppResult<String> {
    let storage = photos::storage(state)?;
    photos::check(&jpeg)?;
    let path = format!("{}{}.jpg", coach_avatar_dir(coach_id), Uuid::new_v4());
    storage.put(&path, jpeg, "image/jpeg").await?;
    // Joined with itself, the row gives back the path it had before.
    let old = sqlx::query_scalar!(
        "UPDATE coaches c SET avatar_path = $2
         FROM coaches old
         WHERE c.id = $1 AND old.id = c.id
         RETURNING old.avatar_path",
        coach_id,
        path,
    )
    .fetch_one(&state.db)
    .await?;
    forget(storage, old).await;
    coach_url(state, coach_id, Some(&path)).ok_or(AppError::NotFound)
}

/// Removes the coach's photo; her clients see her initials again.
pub async fn remove_coach(state: &AppState, coach_id: Uuid) -> AppResult<()> {
    let old = sqlx::query_scalar!(
        "UPDATE coaches c SET avatar_path = NULL
         FROM coaches old
         WHERE c.id = $1 AND old.id = c.id
         RETURNING old.avatar_path",
        coach_id,
    )
    .fetch_one(&state.db)
    .await?;
    if let Some(storage) = &state.storage {
        forget(storage, old).await;
    }
    Ok(())
}

/// Background: copies the Telegram profile photos of clients who have not
/// added their own, and follows changes about weekly. A failure for one
/// client is logged and tried again a day later.
pub async fn refresh_from_telegram(state: &AppState, now: DateTime<Utc>) -> sqlx::Result<()> {
    let Some(storage) = &state.storage else {
        return Ok(());
    };
    let due = sqlx::query!(
        r#"SELECT id, telegram_id AS "telegram_id!", avatar_path, telegram_photo_id
           FROM clients
           WHERE telegram_id IS NOT NULL AND archived_at IS NULL
             AND (avatar_path IS NULL OR avatar_from_telegram)
             AND (telegram_photo_checked_at IS NULL
                  OR telegram_photo_checked_at < $1::timestamptz - make_interval(days => $2))
           ORDER BY telegram_photo_checked_at NULLS FIRST
           LIMIT $3"#,
        now,
        TELEGRAM_RECHECK_DAYS as i32,
        BATCH,
    )
    .fetch_all(&state.db)
    .await?;

    for client in due {
        let checked_at = match copy_telegram_photo(
            state,
            storage,
            client.id,
            client.telegram_id,
            client.avatar_path,
            client.telegram_photo_id,
        )
        .await
        {
            Ok(()) => now,
            Err(err) => {
                tracing::warn!(error = ?err, "could not copy a client's Telegram photo");
                // Due again in a day rather than in the next round.
                now - Duration::days(TELEGRAM_RECHECK_DAYS) + Duration::hours(TELEGRAM_RETRY_HOURS)
            }
        };
        sqlx::query!(
            "UPDATE clients SET telegram_photo_checked_at = $2 WHERE id = $1",
            client.id,
            checked_at,
        )
        .execute(&state.db)
        .await?;
    }
    Ok(())
}

/// Brings one client's Telegram copy up to date: a new photo is copied, an
/// unchanged one left alone, a removed or hidden one taken away.
async fn copy_telegram_photo(
    state: &AppState,
    storage: &StorageClient,
    client_id: Uuid,
    telegram_id: i64,
    current_path: Option<String>,
    current_photo: Option<String>,
) -> anyhow::Result<()> {
    let Some(photo) = state.telegram.profile_photo(telegram_id).await? else {
        let old = sqlx::query_scalar!(
            "UPDATE clients c
             SET avatar_path = NULL, avatar_from_telegram = false, telegram_photo_id = NULL
             FROM clients old
             WHERE c.id = $1 AND old.id = c.id AND c.avatar_from_telegram
             RETURNING old.avatar_path",
            client_id,
        )
        .fetch_optional(&state.db)
        .await?;
        forget(storage, old.flatten()).await;
        return Ok(());
    };
    if current_path.is_some() && current_photo.as_deref() == Some(photo.unique_id.as_str()) {
        return Ok(());
    }

    let jpeg = state
        .telegram
        .download_file(&photo.file_id, photos::MAX_BYTES)
        .await?;
    photos::check(&jpeg).map_err(|_| anyhow::anyhow!("the Telegram photo is not a JPEG"))?;
    let path = format!("{}{}.jpg", avatar_dir(client_id), Uuid::new_v4());
    storage.put(&path, jpeg, "image/jpeg").await?;
    // Unless the client added their own photo meanwhile.
    let old = sqlx::query_scalar!(
        "UPDATE clients c
         SET avatar_path = $2, avatar_from_telegram = true, telegram_photo_id = $3
         FROM clients old
         WHERE c.id = $1 AND old.id = c.id
           AND (c.avatar_path IS NULL OR c.avatar_from_telegram)
         RETURNING old.avatar_path",
        client_id,
        path,
        photo.unique_id,
    )
    .fetch_optional(&state.db)
    .await?;
    match old {
        Some(old) => forget(storage, old).await,
        None => forget(storage, Some(path)).await,
    }
    Ok(())
}
