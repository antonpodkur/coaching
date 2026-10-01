//! "Минулого разу": what a client actually did the last time an exercise came up.

use std::collections::HashMap;

use chrono::NaiveDate;
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

/// A logged set from an earlier workout.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, ToSchema)]
pub struct PastSet {
    /// `null` for bodyweight.
    pub kg: Option<f64>,
    pub reps: Option<i32>,
}

/// For each exercise, the completed sets from the client's latest earlier
/// workout that has any. "Earlier" is by date; undated workouts are skipped.
pub async fn last_time(
    db: &PgPool,
    client_id: Uuid,
    workout_id: Uuid,
    before: Option<NaiveDate>,
    exercise_ids: &[Uuid],
) -> sqlx::Result<HashMap<Uuid, Vec<PastSet>>> {
    let rows = sqlx::query!(
        r#"WITH latest AS (
               SELECT DISTINCT ON (we.exercise_id) we.exercise_id, we.id
               FROM workout_exercises we
               JOIN workouts w ON w.id = we.workout_id
               WHERE w.client_id = $1 AND w.id <> $2 AND w.date IS NOT NULL
                 AND ($3::date IS NULL OR w.date <= $3)
                 AND we.exercise_id = ANY($4)
                 AND EXISTS (
                     SELECT 1 FROM workout_sets s
                     WHERE s.workout_exercise_id = we.id AND s.completed_at IS NOT NULL
                 )
               ORDER BY we.exercise_id, w.date DESC, w.created_at DESC
           )
           SELECT latest.exercise_id AS "exercise_id!", s.actual_kg::float8 AS kg, s.actual_reps
           FROM latest
           JOIN workout_sets s ON s.workout_exercise_id = latest.id
           WHERE s.completed_at IS NOT NULL
           ORDER BY s.position"#,
        client_id,
        workout_id,
        before,
        exercise_ids,
    )
    .fetch_all(db)
    .await?;

    let mut by_exercise: HashMap<Uuid, Vec<PastSet>> = HashMap::new();
    for row in rows {
        by_exercise
            .entry(row.exercise_id)
            .or_default()
            .push(PastSet {
                kg: row.kg,
                reps: row.actual_reps,
            });
    }
    Ok(by_exercise)
}
