use std::collections::HashMap;

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::CurrentCoach,
    error::{AppError, AppResult, ErrorBody},
    import::{normalize_name, parse_plan},
    state::AppState,
};

/// Longer than any real plan; stops accidental pastes of whole chat exports.
const MAX_TEXT_CHARS: usize = 20_000;

#[derive(Deserialize, ToSchema)]
pub struct ImportParseRequest {
    /// An old plan copied from a Telegram message.
    pub text: String,
}

#[derive(Serialize, ToSchema)]
pub struct ImportPreview {
    pub exercises: Vec<ImportedExercise>,
    /// Lines that could not be read, to show the coach.
    pub unparsed: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ImportedExercise {
    /// The name as written in the plan.
    pub name: String,
    /// The library exercise with this name or alias; `null` means it will be added.
    pub exercise_id: Option<Uuid>,
    pub per_side_label: Option<String>,
    pub note: Option<String>,
    pub sets: Vec<ImportedSet>,
}

#[derive(Serialize, ToSchema)]
pub struct ImportedSet {
    /// `null` for bodyweight.
    pub kg: Option<f64>,
    pub reps_min: u32,
    pub reps_max: u32,
}

/// Reads an old Telegram plan and matches it against the library. Saves nothing.
#[utoipa::path(
    post,
    path = "/coach/import/parse",
    tag = "coach",
    security(("bearer" = [])),
    request_body = ImportParseRequest,
    responses(
        (status = 200, body = ImportPreview),
        (status = 400, body = ErrorBody, description = "`text_too_long`"),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody, description = "`wrong_role`"),
    )
)]
pub async fn parse(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Json(request): Json<ImportParseRequest>,
) -> AppResult<Json<ImportPreview>> {
    if request.text.chars().count() > MAX_TEXT_CHARS {
        return Err(AppError::BadRequest("text_too_long"));
    }
    let plan = parse_plan(&request.text);

    let library = sqlx::query!(
        "SELECT id, name, aliases FROM exercises
         WHERE coach_id = $1 AND archived_at IS NULL",
        coach_id,
    )
    .fetch_all(&state.db)
    .await?;
    let mut by_name = HashMap::new();
    for exercise in &library {
        for name in std::iter::once(&exercise.name).chain(&exercise.aliases) {
            by_name.entry(normalize_name(name)).or_insert(exercise.id);
        }
    }

    let exercises = plan
        .exercises
        .into_iter()
        .map(|exercise| ImportedExercise {
            exercise_id: by_name.get(&normalize_name(&exercise.name)).copied(),
            name: exercise.name,
            per_side_label: exercise.per_side_label,
            note: exercise.note,
            sets: exercise
                .sets
                .into_iter()
                .map(|set| ImportedSet {
                    kg: set.kg,
                    reps_min: set.reps_min,
                    reps_max: set.reps_max,
                })
                .collect(),
        })
        .collect();

    Ok(Json(ImportPreview {
        exercises,
        unparsed: plan.unparsed,
    }))
}
