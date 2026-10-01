//! The client's side of workouts: what Dasha published, logging each set, and
//! the report at the end. Every handler takes `CurrentClient`.

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::coach::workouts::WorkoutStatus,
    auth::CurrentClient,
    error::{AppError, AppResult, ErrorBody},
    history::{self, PastSet},
    notify,
    state::AppState,
};

const MAX_KG: f64 = 999.0;
const MAX_REPS: i32 = 500;
const MAX_COMMENT_CHARS: usize = 2_000;

#[derive(Serialize, ToSchema)]
pub struct ClientWorkoutSummary {
    pub id: Uuid,
    pub title: String,
    pub date: NaiveDate,
    pub status: WorkoutStatus,
    pub exercise_count: i64,
    pub set_count: i64,
    pub done_set_count: i64,
    /// The first exercise's video thumbnail, for the card.
    pub thumbnail_url: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ClientWorkout {
    pub id: Uuid,
    pub title: String,
    pub date: NaiveDate,
    pub status: WorkoutStatus,
    pub exercises: Vec<ClientExercise>,
    /// Present once the client has finished it.
    pub report: Option<Report>,
}

#[derive(Serialize, ToSchema)]
pub struct ClientExercise {
    pub id: Uuid,
    pub name: String,
    pub per_side_label: Option<String>,
    pub note: Option<String>,
    pub video: Option<ClientVideo>,
    pub sets: Vec<ClientSet>,
    /// Completed sets from the last earlier workout with this exercise.
    pub last_time: Vec<PastSet>,
}

#[derive(Serialize, ToSchema)]
pub struct ClientVideo {
    pub hls_url: String,
    pub thumbnail_url: String,
    pub length_secs: Option<i32>,
}

#[derive(Serialize, ToSchema)]
pub struct ClientSet {
    pub id: Uuid,
    /// Dasha's target; `null` kg is bodyweight.
    pub target_kg: Option<f64>,
    pub target_reps_min: i32,
    pub target_reps_max: i32,
    /// What the client logged.
    pub actual_kg: Option<f64>,
    pub actual_reps: Option<i32>,
    pub completed: bool,
    /// The phone's time of the latest logged change, for offline merging.
    pub client_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "effort", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    Easy,
    Ok,
    Hard,
}

#[derive(Serialize, ToSchema)]
pub struct Report {
    pub effort: Effort,
    pub comment: String,
    pub duration_min: Option<i32>,
    pub finished_at: DateTime<Utc>,
}

/// One set as logged on the phone. Sending it again is harmless, and a write
/// older than the stored one (it waited offline) is ignored.
#[derive(Deserialize, ToSchema)]
pub struct SetResult {
    pub actual_kg: Option<f64>,
    pub actual_reps: Option<i32>,
    /// Ticked ✓.
    pub completed: bool,
    /// When the phone recorded this change.
    pub client_updated_at: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct FinishWorkout {
    pub effort: Effort,
    #[serde(default)]
    pub comment: String,
    /// From the first ticked set to finishing, as the phone measured it.
    pub duration_min: Option<i32>,
}

/// The client's published and finished workouts, by date.
#[utoipa::path(
    get,
    operation_id = "list_my_workouts",
    path = "/me/workouts",
    tag = "client",
    security(("bearer" = [])),
    responses(
        (status = 200, body = Vec<ClientWorkoutSummary>),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
) -> AppResult<Json<Vec<ClientWorkoutSummary>>> {
    let rows = sqlx::query!(
        r#"SELECT w.id, w.title, w.date AS "date!", w.status AS "status: WorkoutStatus",
                  count(DISTINCT we.id) AS "exercise_count!", count(s.id) AS "set_count!",
                  count(s.completed_at) AS "done_set_count!",
                  (SELECT e.video_uid FROM workout_exercises first
                   JOIN exercises e ON e.id = first.exercise_id
                   WHERE first.workout_id = w.id AND e.video_uid IS NOT NULL
                   ORDER BY first.position LIMIT 1) AS video_uid
           FROM workouts w
           LEFT JOIN workout_exercises we ON we.workout_id = w.id
           LEFT JOIN workout_sets s ON s.workout_exercise_id = we.id
           WHERE w.client_id = $1 AND w.status IN ('published', 'done') AND w.date IS NOT NULL
           GROUP BY w.id
           ORDER BY w.date, w.created_at"#,
        client_id,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| ClientWorkoutSummary {
                thumbnail_url: row
                    .video_uid
                    .zip(state.video.as_ref())
                    .map(|(uid, stream)| stream.thumbnail_url(&uid)),
                id: row.id,
                title: row.title,
                date: row.date,
                status: row.status,
                exercise_count: row.exercise_count,
                set_count: row.set_count,
                done_set_count: row.done_set_count,
            })
            .collect(),
    ))
}

/// One workout with everything the gym needs: targets, logged results,
/// Dasha's videos and notes, and last time's numbers.
#[utoipa::path(
    get,
    operation_id = "get_my_workout",
    path = "/workouts/{id}",
    tag = "client",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Workout id")),
    responses(
        (status = 200, body = ClientWorkout),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody, description = "Not theirs, or not published"),
    )
)]
pub async fn get(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ClientWorkout>> {
    let workout = sqlx::query!(
        r#"SELECT w.id, w.title, w.date AS "date!", w.status AS "status: WorkoutStatus",
                  r.effort AS "effort?: Effort", r.comment AS "comment?", r.duration_min,
                  r.finished_at AS "finished_at?"
           FROM workouts w
           LEFT JOIN workout_reports r ON r.workout_id = w.id
           WHERE w.id = $1 AND w.client_id = $2 AND w.status IN ('published', 'done')
             AND w.date IS NOT NULL"#,
        id,
        client_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let sets = sqlx::query!(
        r#"SELECT s.id, s.workout_exercise_id, s.target_kg::float8 AS target_kg,
                  s.target_reps_min, s.target_reps_max, s.actual_kg::float8 AS actual_kg,
                  s.actual_reps, s.completed_at IS NOT NULL AS "completed!", s.client_updated_at
           FROM workout_sets s
           JOIN workout_exercises we ON we.id = s.workout_exercise_id
           WHERE we.workout_id = $1
           ORDER BY s.position"#,
        id,
    )
    .fetch_all(&state.db)
    .await?;
    let mut sets_by_row: HashMap<Uuid, Vec<ClientSet>> = HashMap::new();
    for set in sets {
        sets_by_row
            .entry(set.workout_exercise_id)
            .or_default()
            .push(ClientSet {
                id: set.id,
                target_kg: set.target_kg,
                target_reps_min: set.target_reps_min,
                target_reps_max: set.target_reps_max,
                actual_kg: set.actual_kg,
                actual_reps: set.actual_reps,
                completed: set.completed,
                client_updated_at: set.client_updated_at,
            });
    }

    let rows = sqlx::query!(
        "SELECT we.id, we.exercise_id, e.name, e.video_uid, e.video_length_secs,
                we.per_side_label, we.note
         FROM workout_exercises we
         JOIN exercises e ON e.id = we.exercise_id
         WHERE we.workout_id = $1
         ORDER BY we.position",
        id,
    )
    .fetch_all(&state.db)
    .await?;
    let exercise_ids: Vec<Uuid> = rows.iter().map(|row| row.exercise_id).collect();
    let mut last_time =
        history::last_time(&state.db, client_id, id, Some(workout.date), &exercise_ids).await?;

    let exercises = rows
        .into_iter()
        .map(|row| ClientExercise {
            video: row
                .video_uid
                .zip(state.video.as_ref())
                .map(|(uid, stream)| ClientVideo {
                    hls_url: stream.hls_url(&uid),
                    thumbnail_url: stream.thumbnail_url(&uid),
                    length_secs: row.video_length_secs,
                }),
            sets: sets_by_row.remove(&row.id).unwrap_or_default(),
            last_time: last_time.remove(&row.exercise_id).unwrap_or_default(),
            id: row.id,
            name: row.name,
            per_side_label: row.per_side_label,
            note: row.note,
        })
        .collect();

    let report = match (workout.effort, workout.finished_at) {
        (Some(effort), Some(finished_at)) => Some(Report {
            effort,
            comment: workout.comment.unwrap_or_default(),
            duration_min: workout.duration_min,
            finished_at,
        }),
        _ => None,
    };
    Ok(Json(ClientWorkout {
        id: workout.id,
        title: workout.title,
        date: workout.date,
        status: workout.status,
        exercises,
        report,
    }))
}

/// Logs one set. Safe to repeat; a change older than what is stored is ignored,
/// so sets queued offline cannot undo newer ones.
#[utoipa::path(
    put,
    operation_id = "log_set",
    path = "/sets/{id}/result",
    tag = "client",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Set id")),
    request_body = SetResult,
    responses(
        (status = 204),
        (status = 400, body = ErrorBody, description = "`invalid_result`"),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn log_set(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
    Path(set_id): Path<Uuid>,
    Json(result): Json<SetResult>,
) -> AppResult<StatusCode> {
    let kg_ok = result
        .actual_kg
        .is_none_or(|kg| kg.is_finite() && kg > 0.0 && kg <= MAX_KG);
    let reps_ok = result
        .actual_reps
        .is_none_or(|reps| (0..=MAX_REPS).contains(&reps));
    if !kg_ok || !reps_ok {
        return Err(AppError::BadRequest("invalid_result"));
    }

    let owned = sqlx::query_scalar!(
        "SELECT s.id FROM workout_sets s
         JOIN workout_exercises we ON we.id = s.workout_exercise_id
         JOIN workouts w ON w.id = we.workout_id
         WHERE s.id = $1 AND w.client_id = $2 AND w.status IN ('published', 'done')",
        set_id,
        client_id,
    )
    .fetch_optional(&state.db)
    .await?;
    if owned.is_none() {
        return Err(AppError::NotFound);
    }

    // The phone's clock may be off; a completion time is never in the future.
    sqlx::query!(
        "UPDATE workout_sets SET
             actual_kg = $2::float8::numeric(6, 2),
             actual_reps = $3,
             completed_at = CASE
                 WHEN $4 THEN COALESCE(completed_at, LEAST($5, now()))
                 ELSE NULL
             END,
             client_updated_at = $5
         WHERE id = $1 AND (client_updated_at IS NULL OR client_updated_at <= $5)",
        set_id,
        result.actual_kg,
        result.actual_reps,
        result.completed,
        result.client_updated_at,
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Finishes the workout with a short report, and has the bot tell Dasha.
/// Sending it again updates the report without messaging her twice.
#[utoipa::path(
    post,
    operation_id = "finish_workout",
    path = "/workouts/{id}/finish",
    tag = "client",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Workout id")),
    request_body = FinishWorkout,
    responses(
        (status = 204),
        (status = 400, body = ErrorBody, description = "`invalid_report`"),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn finish(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
    Path(id): Path<Uuid>,
    Json(body): Json<FinishWorkout>,
) -> AppResult<StatusCode> {
    let comment = body.comment.trim();
    let duration_ok = body
        .duration_min
        .is_none_or(|minutes| (0..=24 * 60).contains(&minutes));
    if comment.chars().count() > MAX_COMMENT_CHARS || !duration_ok {
        return Err(AppError::BadRequest("invalid_report"));
    }

    let mut tx = state.db.begin().await?;
    let finished = sqlx::query_scalar!(
        "UPDATE workouts SET status = 'done', updated_at = now()
         WHERE id = $1 AND client_id = $2 AND status IN ('published', 'done')
         RETURNING id",
        id,
        client_id,
    )
    .fetch_optional(&mut *tx)
    .await?;
    if finished.is_none() {
        return Err(AppError::NotFound);
    }
    sqlx::query!(
        "INSERT INTO workout_reports (workout_id, effort, comment, duration_min)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (workout_id) DO UPDATE SET
             effort = EXCLUDED.effort,
             comment = EXCLUDED.comment,
             duration_min = EXCLUDED.duration_min",
        id,
        body.effort as Effort,
        comment,
        body.duration_min,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // The report is saved either way; a failed message is retried by finishing again.
    if let Err(err) = notify::workout_finished(&state, id).await {
        tracing::warn!(error = ?err, "could not tell the coach about a finished workout");
    }
    Ok(StatusCode::NO_CONTENT)
}
