//! Photos Dasha adds to a library exercise: the machine, the handle, the
//! starting position. Clients see them under her video, so a description is
//! not left to carry everything.

use std::collections::HashMap;

use chrono::Utc;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    cdn_token,
    error::{AppError, AppResult},
    photos,
    state::AppState,
};

pub const MAX_PER_EXERCISE: i64 = 5;

#[derive(Clone, Serialize, ToSchema)]
pub struct ExercisePhoto {
    pub id: Uuid,
    /// Signed for a few hours.
    pub url: String,
}

/// Where an exercise's photos live in the storage zone; one signed link
/// covers them all.
fn exercise_dir(exercise_id: Uuid) -> String {
    format!("exercises/{exercise_id}/")
}

/// The photos of these exercises, oldest first, with fresh links. Empty when
/// photo storage is not set up.
pub async fn for_exercises(
    state: &AppState,
    exercise_ids: &[Uuid],
) -> sqlx::Result<HashMap<Uuid, Vec<ExercisePhoto>>> {
    let mut by_exercise: HashMap<Uuid, Vec<ExercisePhoto>> = HashMap::new();
    let Some(storage) = &state.storage else {
        return Ok(by_exercise);
    };
    let rows = sqlx::query!(
        "SELECT id, exercise_id, path FROM exercise_photos
         WHERE exercise_id = ANY($1) ORDER BY created_at",
        exercise_ids,
    )
    .fetch_all(&state.db)
    .await?;
    let expires = cdn_token::link_expiry(Utc::now());
    for row in rows {
        by_exercise
            .entry(row.exercise_id)
            .or_default()
            .push(ExercisePhoto {
                id: row.id,
                url: storage.signed_url(&exercise_dir(row.exercise_id), &row.path, expires),
            });
    }
    Ok(by_exercise)
}

/// Stores a photo of one of Dasha's exercises: a JPEG the phone already shrank.
pub async fn add(
    state: &AppState,
    coach_id: Uuid,
    exercise_id: Uuid,
    jpeg: Vec<u8>,
) -> AppResult<ExercisePhoto> {
    let storage = photos::storage(state)?;
    photos::check(&jpeg)?;
    let count = sqlx::query_scalar!(
        r#"SELECT count(p.id) AS "count!"
           FROM exercises e LEFT JOIN exercise_photos p ON p.exercise_id = e.id
           WHERE e.id = $1 AND e.coach_id = $2
           GROUP BY e.id"#,
        exercise_id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    if count >= MAX_PER_EXERCISE {
        return Err(AppError::Conflict("too_many_photos"));
    }

    let id = Uuid::new_v4();
    let path = format!("{}{id}.jpg", exercise_dir(exercise_id));
    storage.put(&path, jpeg, "image/jpeg").await?;
    sqlx::query!(
        "INSERT INTO exercise_photos (id, exercise_id, path) VALUES ($1, $2, $3)",
        id,
        exercise_id,
        path,
    )
    .execute(&state.db)
    .await?;
    let expires = cdn_token::link_expiry(Utc::now());
    Ok(ExercisePhoto {
        id,
        url: storage.signed_url(&exercise_dir(exercise_id), &path, expires),
    })
}

/// Deletes one of Dasha's exercise photos, on Bunny too.
pub async fn delete(state: &AppState, coach_id: Uuid, id: Uuid) -> AppResult<()> {
    let path = sqlx::query_scalar!(
        "DELETE FROM exercise_photos p USING exercises e
         WHERE p.id = $1 AND e.id = p.exercise_id AND e.coach_id = $2
         RETURNING p.path",
        id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    if let Some(storage) = &state.storage
        && let Err(err) = storage.delete(&path).await
    {
        tracing::warn!(error = ?err, "could not delete an exercise photo on Bunny");
    }
    Ok(())
}
