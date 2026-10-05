//! Dasha's daily nutrition targets for a client: protein, fat and
//! carbohydrates in grams, the same every day until she changes them. Each
//! change is kept, and the client hears about it from the bot.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

const MAX_GRAMS: i32 = 1_000;
const MAX_NOTE_CHARS: usize = 500;

#[derive(Serialize, ToSchema)]
pub struct NutritionTarget {
    pub id: Uuid,
    pub protein_g: i32,
    pub fat_g: i32,
    pub carbs_g: i32,
    /// 4 kcal per gram of protein and of carbohydrates, 9 per gram of fat.
    pub kcal: i32,
    /// Dasha's words to go with it, e.g. "2 л води на день".
    pub note: Option<String>,
    pub set_at: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct NewTarget {
    pub protein_g: i32,
    pub fat_g: i32,
    pub carbs_g: i32,
    pub note: Option<String>,
}

/// Calories from grams: 4 per gram of protein and carbohydrates, 9 of fat.
pub fn kcal(protein_g: i32, fat_g: i32, carbs_g: i32) -> i32 {
    4 * protein_g + 9 * fat_g + 4 * carbs_g
}

struct Row {
    id: Uuid,
    protein_g: i32,
    fat_g: i32,
    carbs_g: i32,
    note: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<Row> for NutritionTarget {
    fn from(row: Row) -> Self {
        Self {
            id: row.id,
            kcal: kcal(row.protein_g, row.fat_g, row.carbs_g),
            protein_g: row.protein_g,
            fat_g: row.fat_g,
            carbs_g: row.carbs_g,
            note: row.note,
            set_at: row.created_at,
        }
    }
}

/// Every target a client has had, newest (the current one) first.
pub async fn history(state: &AppState, client_id: Uuid) -> sqlx::Result<Vec<NutritionTarget>> {
    let rows = sqlx::query_as!(
        Row,
        "SELECT id, protein_g, fat_g, carbs_g, note, created_at
         FROM nutrition_targets WHERE client_id = $1 ORDER BY created_at DESC",
        client_id,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(rows.into_iter().map(NutritionTarget::from).collect())
}

/// The client's target now, if Dasha has set one.
pub async fn current(state: &AppState, client_id: Uuid) -> sqlx::Result<Option<NutritionTarget>> {
    let row = sqlx::query_as!(
        Row,
        "SELECT id, protein_g, fat_g, carbs_g, note, created_at
         FROM nutrition_targets WHERE client_id = $1 ORDER BY created_at DESC LIMIT 1",
        client_id,
    )
    .fetch_optional(&state.db)
    .await?;
    Ok(row.map(NutritionTarget::from))
}

/// Records a new target for the client; it replaces the current one.
pub async fn set(state: &AppState, client_id: Uuid, new: &NewTarget) -> AppResult<NutritionTarget> {
    let grams = [new.protein_g, new.fat_g, new.carbs_g];
    if grams.iter().any(|g| !(0..=MAX_GRAMS).contains(g)) || grams.iter().all(|g| *g == 0) {
        return Err(AppError::BadRequest("invalid_grams"));
    }
    let note = new
        .note
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty());
    if note.is_some_and(|note| note.chars().count() > MAX_NOTE_CHARS) {
        return Err(AppError::BadRequest("note_too_long"));
    }
    let row = sqlx::query_as!(
        Row,
        "INSERT INTO nutrition_targets (client_id, protein_g, fat_g, carbs_g, note)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, protein_g, fat_g, carbs_g, note, created_at",
        client_id,
        new.protein_g,
        new.fat_g,
        new.carbs_g,
        note,
    )
    .fetch_one(&state.db)
    .await?;
    Ok(row.into())
}
