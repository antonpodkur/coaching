use std::collections::{HashMap, HashSet};

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::IF_MATCH},
};
use chrono::{DateTime, Days, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::{coach::exercises::Measure, workouts::Effort},
    auth::CurrentCoach,
    error::{AppError, AppResult, ErrorBody},
    history::{self, PastSet},
    notify,
    state::AppState,
};

const MAX_TITLE_CHARS: usize = 120;
const MAX_LABEL_CHARS: usize = 40;
const MAX_NOTE_CHARS: usize = 500;
const MAX_EXERCISES: usize = 40;
const MAX_SETS_PER_EXERCISE: usize = 30;
const MAX_KG: f64 = 999.0;
/// Reps, or seconds for a timed exercise (up to an hour on a bike).
const MAX_REPS: i32 = 3_600;
/// "Copy to the next one" lands on the same weekday of the next week.
const COPY_DAYS_LATER: u64 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "workout_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WorkoutStatus {
    Draft,
    /// The client can see it.
    Published,
    /// The client finished it and sent a report.
    Done,
}

/// A workout as one document: what the builder loads and saves.
#[derive(Serialize, ToSchema)]
pub struct Workout {
    pub id: Uuid,
    /// `null` for a template.
    pub client_id: Option<Uuid>,
    pub title: String,
    /// A calendar date for the client. Needed to publish.
    pub date: Option<NaiveDate>,
    pub status: WorkoutStatus,
    /// Send it back as `If-Match` when saving.
    pub version: i32,
    pub published_at: Option<DateTime<Utc>>,
    pub exercises: Vec<WorkoutExercise>,
}

#[derive(Serialize, ToSchema)]
pub struct WorkoutExercise {
    pub id: Uuid,
    pub exercise_id: Uuid,
    /// From the library, for display.
    pub name: String,
    /// From the library: kg × reps, reps, or seconds.
    pub measure: Measure,
    pub thumbnail_url: Option<String>,
    /// E.g. `на кожну руку`: the targets are per arm or leg.
    pub per_side_label: Option<String>,
    pub note: Option<String>,
    pub sets: Vec<WorkoutSet>,
    /// What the client did the last time this exercise came up ("Минулого разу").
    pub last_time: Vec<PastSet>,
}

/// One set's target, as Dasha writes it.
#[derive(Serialize, Deserialize, ToSchema, Clone, Copy)]
pub struct WorkoutSet {
    pub id: Uuid,
    /// `null` for no weight. For bodyweight and timed exercises, extra weight.
    pub kg: Option<f64>,
    /// `8-10` is 8 and 10; a plain `12` is 12 and 12. Seconds for a timed exercise.
    pub reps_min: i32,
    pub reps_max: i32,
}

/// The whole workout as the builder has it now. Rows keep their IDs across
/// saves, so results the client already logged stay attached.
#[derive(Deserialize, ToSchema)]
pub struct WorkoutChanges {
    pub title: String,
    pub date: Option<NaiveDate>,
    /// In order.
    pub exercises: Vec<WorkoutExerciseChanges>,
}

#[derive(Deserialize, ToSchema)]
pub struct WorkoutExerciseChanges {
    /// A new row gets its ID from the app, so a retried save cannot duplicate it.
    pub id: Uuid,
    pub exercise_id: Uuid,
    pub per_side_label: Option<String>,
    pub note: Option<String>,
    /// In order.
    pub sets: Vec<WorkoutSet>,
}

#[derive(Deserialize, ToSchema)]
pub struct NewWorkout {
    pub client_id: Uuid,
    /// Start as a copy of this workout (any of the coach's), instead of blank.
    pub copy_from: Option<Uuid>,
    /// For a copy, defaults to a week after the copied workout's date.
    pub date: Option<NaiveDate>,
}

#[derive(Serialize, ToSchema)]
pub struct WorkoutSummary {
    pub id: Uuid,
    pub title: String,
    pub date: Option<NaiveDate>,
    pub status: WorkoutStatus,
    pub published_at: Option<DateTime<Utc>>,
    pub exercise_count: i64,
    pub set_count: i64,
    /// Sets the client ticked.
    pub done_set_count: i64,
    /// Ticked sets with other numbers than planned.
    pub different_count: i64,
    /// The client has opened it in the app.
    pub opened: bool,
    /// Present once the client finished it.
    pub report: Option<ReportSummary>,
}

#[derive(Serialize, ToSchema)]
pub struct ReportSummary {
    pub effort: Effort,
    pub has_comment: bool,
    pub finished_at: DateTime<Utc>,
    /// Dasha has opened it.
    pub seen: bool,
}

/// Targets next to what the client did, for Dasha's report view.
#[derive(Serialize, ToSchema)]
pub struct WorkoutResults {
    pub id: Uuid,
    pub client_id: Option<Uuid>,
    pub title: String,
    pub date: Option<NaiveDate>,
    pub status: WorkoutStatus,
    pub report: Option<CoachReport>,
    pub exercises: Vec<ResultExercise>,
}

#[derive(Serialize, ToSchema)]
pub struct CoachReport {
    pub effort: Effort,
    pub comment: String,
    pub duration_min: Option<i32>,
    pub finished_at: DateTime<Utc>,
    pub seen: bool,
}

#[derive(Serialize, ToSchema)]
pub struct ResultExercise {
    pub id: Uuid,
    pub name: String,
    pub measure: Measure,
    pub per_side_label: Option<String>,
    pub sets: Vec<ResultSet>,
}

#[derive(Serialize, ToSchema)]
pub struct ResultSet {
    pub id: Uuid,
    pub target_kg: Option<f64>,
    pub target_reps_min: i32,
    pub target_reps_max: i32,
    pub actual_kg: Option<f64>,
    pub actual_reps: Option<i32>,
    pub completed: bool,
    /// Ticked with other numbers than planned.
    pub differs: bool,
}

#[derive(Serialize, ToSchema)]
pub struct PublishedWorkout {
    pub workout: Workout,
    /// The bot has told the client; `false` while they have not joined the app or
    /// not allowed the bot to message them. They see the workout in the app either way.
    pub client_notified: bool,
}

/// Starts a workout for a client: blank, or as a copy of an earlier one.
#[utoipa::path(
    post,
    operation_id = "create_workout",
    path = "/coach/workouts",
    tag = "coach",
    security(("bearer" = [])),
    request_body = NewWorkout,
    responses(
        (status = 201, body = Workout),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody, description = "No such client, or no such workout to copy"),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Json(body): Json<NewWorkout>,
) -> AppResult<(StatusCode, Json<Workout>)> {
    let client = sqlx::query_scalar!(
        "SELECT id FROM clients WHERE id = $1 AND coach_id = $2 AND archived_at IS NULL",
        body.client_id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?;
    if client.is_none() {
        return Err(AppError::NotFound);
    }

    let mut tx = state.db.begin().await?;
    let id =
        match body.copy_from {
            None => sqlx::query_scalar!(
                "INSERT INTO workouts (coach_id, client_id, date) VALUES ($1, $2, $3) RETURNING id",
                coach_id,
                body.client_id,
                body.date,
            )
            .fetch_one(&mut *tx)
            .await?,
            Some(source_id) => {
                let source = sqlx::query!(
                    "SELECT title, date FROM workouts WHERE id = $1 AND coach_id = $2",
                    source_id,
                    coach_id,
                )
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(AppError::NotFound)?;
                let date = body.date.or_else(|| {
                    source
                        .date
                        .and_then(|date| date.checked_add_days(Days::new(COPY_DAYS_LATER)))
                });
                let id = sqlx::query_scalar!(
                "INSERT INTO workouts (coach_id, client_id, title, date, source, copied_from_id)
                 VALUES ($1, $2, $3, $4, 'copy', $5)
                 RETURNING id",
                coach_id,
                body.client_id,
                source.title,
                date,
                source_id,
            )
                .fetch_one(&mut *tx)
                .await?;
                copy_contents(&mut tx, source_id, id).await?;
                id
            }
        };
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(fetch(&state, coach_id, id).await?),
    ))
}

/// Duplicates every exercise and set target; results are not copied.
async fn copy_contents(
    tx: &mut sqlx::PgConnection,
    source_id: Uuid,
    target_id: Uuid,
) -> sqlx::Result<()> {
    let exercises = sqlx::query_scalar!(
        "SELECT id FROM workout_exercises WHERE workout_id = $1",
        source_id,
    )
    .fetch_all(&mut *tx)
    .await?;
    let copies: Vec<Uuid> = exercises.iter().map(|_| Uuid::new_v4()).collect();
    sqlx::query!(
        "INSERT INTO workout_exercises (id, workout_id, exercise_id, position, per_side_label, note)
         SELECT m.new_id, $2, we.exercise_id, we.position, we.per_side_label, we.note
         FROM UNNEST($3::uuid[], $4::uuid[]) AS m (old_id, new_id)
         JOIN workout_exercises we ON we.id = m.old_id AND we.workout_id = $1",
        source_id,
        target_id,
        &exercises,
        &copies,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO workout_sets
             (workout_exercise_id, position, target_kg, target_reps_min, target_reps_max)
         SELECT m.new_id, s.position, s.target_kg, s.target_reps_min, s.target_reps_max
         FROM UNNEST($1::uuid[], $2::uuid[]) AS m (old_id, new_id)
         JOIN workout_sets s ON s.workout_exercise_id = m.old_id",
        &exercises,
        &copies,
    )
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// One workout, for the builder.
#[utoipa::path(
    get,
    operation_id = "get_workout",
    path = "/coach/workouts/{id}",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Workout id")),
    responses(
        (status = 200, body = Workout),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn get(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Workout>> {
    Ok(Json(fetch(&state, coach_id, id).await?))
}

/// Saves the whole workout. `If-Match` must carry the version the builder
/// loaded; if the workout changed since (say, on her laptop), the save is
/// refused with 409 instead of overwriting.
#[utoipa::path(
    put,
    operation_id = "save_workout",
    path = "/coach/workouts/{id}",
    tag = "coach",
    security(("bearer" = [])),
    params(
        ("id" = Uuid, Path, description = "Workout id"),
        ("If-Match" = String, Header, description = "The `version` the changes are based on"),
    ),
    request_body = WorkoutChanges,
    responses(
        (status = 200, body = Workout),
        (status = 400, body = ErrorBody, description = "`version_required`, `invalid_title`, `invalid_text`, `invalid_set`, `too_many_exercises`, `too_many_sets`, `invalid_ids` or `unknown_exercise`"),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody, description = "`version_conflict`: reload and apply the changes again"),
    )
)]
pub async fn save(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(changes): Json<WorkoutChanges>,
) -> AppResult<Json<Workout>> {
    let version = if_match(&headers)?;
    let doc = Cleaned::from(changes)?;

    let mut tx = state.db.begin().await?;
    let saved = sqlx::query_scalar!(
        "UPDATE workouts SET title = $3, date = $4, version = version + 1, updated_at = now()
         WHERE id = $1 AND coach_id = $2 AND version = $5
         RETURNING id",
        id,
        coach_id,
        doc.title,
        doc.date,
        version,
    )
    .fetch_optional(&mut *tx)
    .await?;
    if saved.is_none() {
        let exists = sqlx::query_scalar!(
            "SELECT id FROM workouts WHERE id = $1 AND coach_id = $2",
            id,
            coach_id,
        )
        .fetch_optional(&mut *tx)
        .await?;
        return Err(match exists {
            Some(_) => AppError::Conflict("version_conflict"),
            None => AppError::NotFound,
        });
    }

    let library: HashSet<Uuid> = doc.exercise_ids.iter().copied().collect();
    let library: Vec<Uuid> = library.into_iter().collect();
    let owned = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM exercises WHERE coach_id = $1 AND id = ANY($2)"#,
        coach_id,
        &library,
    )
    .fetch_one(&mut *tx)
    .await?;
    if owned != library.len() as i64 {
        return Err(AppError::BadRequest("unknown_exercise"));
    }

    // IDs may be new or this workout's own, never another workout's rows.
    let foreign = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM workout_exercises WHERE id = ANY($2) AND workout_id <> $1
           ) OR EXISTS (
               SELECT 1 FROM workout_sets s
               JOIN workout_exercises we ON we.id = s.workout_exercise_id
               WHERE s.id = ANY($3) AND we.workout_id <> $1
           ) AS "foreign!""#,
        id,
        &doc.row_ids,
        &doc.set_ids,
    )
    .fetch_one(&mut *tx)
    .await?;
    if foreign {
        return Err(AppError::BadRequest("invalid_ids"));
    }

    sqlx::query!(
        "DELETE FROM workout_sets s USING workout_exercises we
         WHERE s.workout_exercise_id = we.id AND we.workout_id = $1 AND NOT (s.id = ANY($2))",
        id,
        &doc.set_ids,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM workout_exercises WHERE workout_id = $1 AND NOT (id = ANY($2))",
        id,
        &doc.row_ids,
    )
    .execute(&mut *tx)
    .await?;
    // Positions are unique per workout, checked at commit, so rows can swap places.
    sqlx::query!(
        "INSERT INTO workout_exercises (id, workout_id, exercise_id, position, per_side_label, note)
         SELECT row.id, $1, row.exercise_id, row.position, row.label, row.note
         FROM UNNEST($2::uuid[], $3::uuid[], $4::int4[], $5::text[], $6::text[])
             AS row (id, exercise_id, position, label, note)
         ON CONFLICT (id) DO UPDATE SET
             exercise_id = EXCLUDED.exercise_id,
             position = EXCLUDED.position,
             per_side_label = EXCLUDED.per_side_label,
             note = EXCLUDED.note",
        id,
        &doc.row_ids,
        &doc.exercise_ids,
        &doc.row_positions,
        &doc.labels as &[Option<String>],
        &doc.notes as &[Option<String>],
    )
    .execute(&mut *tx)
    .await?;
    // Only targets change; results the client logged stay on their rows.
    sqlx::query!(
        "INSERT INTO workout_sets
             (id, workout_exercise_id, position, target_kg, target_reps_min, target_reps_max)
         SELECT set.id, set.row_id, set.position, set.kg::numeric(6, 2), set.reps_min, set.reps_max
         FROM UNNEST($1::uuid[], $2::uuid[], $3::int4[], $4::float8[], $5::int4[], $6::int4[])
             AS set (id, row_id, position, kg, reps_min, reps_max)
         ON CONFLICT (id) DO UPDATE SET
             workout_exercise_id = EXCLUDED.workout_exercise_id,
             position = EXCLUDED.position,
             target_kg = EXCLUDED.target_kg,
             target_reps_min = EXCLUDED.target_reps_min,
             target_reps_max = EXCLUDED.target_reps_max",
        &doc.set_ids,
        &doc.set_rows,
        &doc.set_positions,
        &doc.set_kg as &[Option<f64>],
        &doc.reps_min,
        &doc.reps_max,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(fetch(&state, coach_id, id).await?))
}

/// Makes the workout visible to the client and has the bot tell them. Safe to
/// repeat: the client is told once, e.g. after they join the app.
#[utoipa::path(
    post,
    operation_id = "publish_workout",
    path = "/coach/workouts/{id}/publish",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Workout id")),
    responses(
        (status = 200, body = PublishedWorkout),
        (status = 400, body = ErrorBody, description = "`template`, `date_required`, `empty_workout` or `exercise_without_sets`"),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn publish(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PublishedWorkout>> {
    let workout = fetch(&state, coach_id, id).await?;
    if workout.client_id.is_none() {
        return Err(AppError::BadRequest("template"));
    }
    if workout.date.is_none() {
        return Err(AppError::BadRequest("date_required"));
    }
    if workout.exercises.is_empty() {
        return Err(AppError::BadRequest("empty_workout"));
    }
    if workout
        .exercises
        .iter()
        .any(|exercise| exercise.sets.is_empty())
    {
        return Err(AppError::BadRequest("exercise_without_sets"));
    }

    sqlx::query!(
        "UPDATE workouts SET status = 'published', published_at = now(), updated_at = now()
         WHERE id = $1 AND status = 'draft'",
        id,
    )
    .execute(&state.db)
    .await?;
    // The workout is published either way; a failed message is retried by publishing again.
    let client_notified = match notify::workout_published(&state, id).await {
        Ok(notified) => notified,
        Err(err) => {
            tracing::warn!(error = ?err, "could not tell the client about a published workout");
            false
        }
    };
    Ok(Json(PublishedWorkout {
        workout: fetch(&state, coach_id, id).await?,
        client_notified,
    }))
}

/// Deletes a workout, published or not.
#[utoipa::path(
    delete,
    operation_id = "delete_workout",
    path = "/coach/workouts/{id}",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Workout id")),
    responses(
        (status = 204),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let deleted = sqlx::query_scalar!(
        "DELETE FROM workouts WHERE id = $1 AND coach_id = $2 RETURNING id",
        id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?;
    match deleted {
        Some(_) => Ok(StatusCode::NO_CONTENT),
        None => Err(AppError::NotFound),
    }
}

/// A client's workouts: undated drafts first, then newest date first.
#[utoipa::path(
    get,
    operation_id = "list_client_workouts",
    path = "/coach/clients/{id}/workouts",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Client id")),
    responses(
        (status = 200, body = Vec<WorkoutSummary>),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn list_for_client(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(client_id): Path<Uuid>,
) -> AppResult<Json<Vec<WorkoutSummary>>> {
    let client = sqlx::query_scalar!(
        "SELECT id FROM clients WHERE id = $1 AND coach_id = $2",
        client_id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?;
    if client.is_none() {
        return Err(AppError::NotFound);
    }
    let rows = sqlx::query!(
        r#"SELECT w.id, w.title, w.date, w.status AS "status: WorkoutStatus", w.published_at,
                  w.opened_at IS NOT NULL AS "opened!",
                  count(DISTINCT we.id) AS "exercise_count!", count(s.id) AS "set_count!",
                  count(s.completed_at) AS "done_set_count!",
                  count(*) FILTER (WHERE set_differs(s)) AS "different_count!",
                  r.effort AS "effort?: Effort", r.comment AS "comment?",
                  r.finished_at AS "finished_at?", r.seen_at
           FROM workouts w
           LEFT JOIN workout_exercises we ON we.workout_id = w.id
           LEFT JOIN workout_sets s ON s.workout_exercise_id = we.id
           LEFT JOIN workout_reports r ON r.workout_id = w.id
           WHERE w.client_id = $1 AND w.coach_id = $2
           GROUP BY w.id, r.workout_id
           ORDER BY w.date DESC NULLS FIRST, w.created_at DESC"#,
        client_id,
        coach_id,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| WorkoutSummary {
                report: row
                    .effort
                    .zip(row.finished_at)
                    .map(|(effort, finished_at)| ReportSummary {
                        effort,
                        has_comment: row.comment.as_deref().is_some_and(|c| !c.is_empty()),
                        finished_at,
                        seen: row.seen_at.is_some(),
                    }),
                id: row.id,
                title: row.title,
                date: row.date,
                status: row.status,
                published_at: row.published_at,
                exercise_count: row.exercise_count,
                set_count: row.set_count,
                done_set_count: row.done_set_count,
                different_count: row.different_count,
                opened: row.opened,
            })
            .collect(),
    ))
}

/// What the client did, set by set next to the plan, and their report.
#[utoipa::path(
    get,
    operation_id = "get_workout_results",
    path = "/coach/workouts/{id}/results",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Workout id")),
    responses(
        (status = 200, body = WorkoutResults),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn results(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(id): Path<Uuid>,
) -> AppResult<Json<WorkoutResults>> {
    let workout = sqlx::query!(
        r#"SELECT w.id, w.client_id, w.title, w.date, w.status AS "status: WorkoutStatus",
                  r.effort AS "effort?: Effort", r.comment AS "comment?", r.duration_min,
                  r.finished_at AS "finished_at?", r.seen_at
           FROM workouts w
           LEFT JOIN workout_reports r ON r.workout_id = w.id
           WHERE w.id = $1 AND w.coach_id = $2"#,
        id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let sets = sqlx::query!(
        r#"SELECT s.id, s.workout_exercise_id, s.target_kg::float8 AS target_kg,
                  s.target_reps_min, s.target_reps_max, s.actual_kg::float8 AS actual_kg,
                  s.actual_reps, s.completed_at IS NOT NULL AS "completed!",
                  set_differs(s) AS "differs!"
           FROM workout_sets s
           JOIN workout_exercises we ON we.id = s.workout_exercise_id
           WHERE we.workout_id = $1
           ORDER BY s.position"#,
        id,
    )
    .fetch_all(&state.db)
    .await?;
    let mut sets_by_row: HashMap<Uuid, Vec<ResultSet>> = HashMap::new();
    for set in sets {
        sets_by_row
            .entry(set.workout_exercise_id)
            .or_default()
            .push(ResultSet {
                id: set.id,
                target_kg: set.target_kg,
                target_reps_min: set.target_reps_min,
                target_reps_max: set.target_reps_max,
                actual_kg: set.actual_kg,
                actual_reps: set.actual_reps,
                completed: set.completed,
                differs: set.differs,
            });
    }
    let exercises = sqlx::query!(
        r#"SELECT we.id, e.name, e.measure AS "measure: Measure", we.per_side_label
         FROM workout_exercises we
         JOIN exercises e ON e.id = we.exercise_id
         WHERE we.workout_id = $1
         ORDER BY we.position"#,
        id,
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|row| ResultExercise {
        sets: sets_by_row.remove(&row.id).unwrap_or_default(),
        id: row.id,
        name: row.name,
        measure: row.measure,
        per_side_label: row.per_side_label,
    })
    .collect();

    let report = workout
        .effort
        .zip(workout.finished_at)
        .map(|(effort, finished_at)| CoachReport {
            effort,
            comment: workout.comment.unwrap_or_default(),
            duration_min: workout.duration_min,
            finished_at,
            seen: workout.seen_at.is_some(),
        });
    Ok(Json(WorkoutResults {
        id: workout.id,
        client_id: workout.client_id,
        title: workout.title,
        date: workout.date,
        status: workout.status,
        report,
        exercises,
    }))
}

/// Marks the workout's report as seen, so it stops showing as new.
#[utoipa::path(
    post,
    operation_id = "mark_report_seen",
    path = "/coach/workouts/{id}/report/seen",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Workout id")),
    responses(
        (status = 204),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody, description = "No such workout, or no report yet"),
    )
)]
pub async fn mark_report_seen(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let marked = sqlx::query_scalar!(
        "UPDATE workout_reports r SET seen_at = COALESCE(r.seen_at, now())
         FROM workouts w
         WHERE r.workout_id = $1 AND w.id = r.workout_id AND w.coach_id = $2
         RETURNING r.workout_id",
        id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?;
    match marked {
        Some(_) => Ok(StatusCode::NO_CONTENT),
        None => Err(AppError::NotFound),
    }
}

async fn fetch(state: &AppState, coach_id: Uuid, id: Uuid) -> AppResult<Workout> {
    let workout = sqlx::query!(
        r#"SELECT id, client_id, title, date, status AS "status: WorkoutStatus", version,
                  published_at
           FROM workouts WHERE id = $1 AND coach_id = $2"#,
        id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let sets = sqlx::query!(
        r#"SELECT s.id, s.workout_exercise_id, s.target_kg::float8 AS kg,
                  s.target_reps_min, s.target_reps_max
           FROM workout_sets s
           JOIN workout_exercises we ON we.id = s.workout_exercise_id
           WHERE we.workout_id = $1
           ORDER BY s.position"#,
        id,
    )
    .fetch_all(&state.db)
    .await?;
    let mut sets_by_row: HashMap<Uuid, Vec<WorkoutSet>> = HashMap::new();
    for set in sets {
        sets_by_row
            .entry(set.workout_exercise_id)
            .or_default()
            .push(WorkoutSet {
                id: set.id,
                kg: set.kg,
                reps_min: set.target_reps_min,
                reps_max: set.target_reps_max,
            });
    }

    let rows = sqlx::query!(
        r#"SELECT we.id, we.exercise_id, e.name, e.measure AS "measure: Measure", e.video_uid,
                  we.per_side_label, we.note
           FROM workout_exercises we
           JOIN exercises e ON e.id = we.exercise_id
           WHERE we.workout_id = $1
           ORDER BY we.position"#,
        id,
    )
    .fetch_all(&state.db)
    .await?;
    let mut last_time = match workout.client_id {
        Some(client_id) => {
            let exercise_ids: Vec<Uuid> = rows.iter().map(|row| row.exercise_id).collect();
            history::last_time(&state.db, client_id, id, workout.date, &exercise_ids).await?
        }
        None => HashMap::new(),
    };
    let exercises = rows
        .into_iter()
        .map(|row| WorkoutExercise {
            last_time: last_time.remove(&row.exercise_id).unwrap_or_default(),
            thumbnail_url: row
                .video_uid
                .zip(state.video.as_ref())
                .map(|(uid, stream)| stream.thumbnail_url(&uid)),
            sets: sets_by_row.remove(&row.id).unwrap_or_default(),
            id: row.id,
            exercise_id: row.exercise_id,
            name: row.name,
            measure: row.measure,
            per_side_label: row.per_side_label,
            note: row.note,
        })
        .collect();

    Ok(Workout {
        id: workout.id,
        client_id: workout.client_id,
        title: workout.title,
        date: workout.date,
        status: workout.status,
        version: workout.version,
        published_at: workout.published_at,
        exercises,
    })
}

/// Accepts `3`, `"3"` and `W/"3"`.
fn if_match(headers: &HeaderMap) -> AppResult<i32> {
    headers
        .get(IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().trim_start_matches("W/").trim_matches('"'))
        .and_then(|value| value.parse().ok())
        .ok_or(AppError::BadRequest("version_required"))
}

/// A checked workout, flattened into the columns the save query binds.
struct Cleaned {
    title: String,
    date: Option<NaiveDate>,
    row_ids: Vec<Uuid>,
    exercise_ids: Vec<Uuid>,
    row_positions: Vec<i32>,
    labels: Vec<Option<String>>,
    notes: Vec<Option<String>>,
    set_ids: Vec<Uuid>,
    set_rows: Vec<Uuid>,
    set_positions: Vec<i32>,
    set_kg: Vec<Option<f64>>,
    reps_min: Vec<i32>,
    reps_max: Vec<i32>,
}

impl Cleaned {
    fn from(changes: WorkoutChanges) -> AppResult<Self> {
        let title = changes
            .title
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if title.chars().count() > MAX_TITLE_CHARS {
            return Err(AppError::BadRequest("invalid_title"));
        }
        if changes.exercises.len() > MAX_EXERCISES {
            return Err(AppError::BadRequest("too_many_exercises"));
        }

        let mut doc = Self {
            title,
            date: changes.date,
            row_ids: Vec::new(),
            exercise_ids: Vec::new(),
            row_positions: Vec::new(),
            labels: Vec::new(),
            notes: Vec::new(),
            set_ids: Vec::new(),
            set_rows: Vec::new(),
            set_positions: Vec::new(),
            set_kg: Vec::new(),
            reps_min: Vec::new(),
            reps_max: Vec::new(),
        };
        let mut seen = HashSet::new();
        for (position, exercise) in changes.exercises.into_iter().enumerate() {
            if !seen.insert(exercise.id) {
                return Err(AppError::BadRequest("invalid_ids"));
            }
            if exercise.sets.len() > MAX_SETS_PER_EXERCISE {
                return Err(AppError::BadRequest("too_many_sets"));
            }
            doc.row_ids.push(exercise.id);
            doc.exercise_ids.push(exercise.exercise_id);
            doc.row_positions.push(position as i32);
            doc.labels
                .push(clean_text(exercise.per_side_label, MAX_LABEL_CHARS)?);
            doc.notes.push(clean_text(exercise.note, MAX_NOTE_CHARS)?);

            for (set_position, set) in exercise.sets.into_iter().enumerate() {
                if !seen.insert(set.id) {
                    return Err(AppError::BadRequest("invalid_ids"));
                }
                let kg_ok = set
                    .kg
                    .is_none_or(|kg| kg.is_finite() && kg > 0.0 && kg <= MAX_KG);
                if !kg_ok
                    || set.reps_min < 1
                    || set.reps_max < set.reps_min
                    || set.reps_max > MAX_REPS
                {
                    return Err(AppError::BadRequest("invalid_set"));
                }
                doc.set_ids.push(set.id);
                doc.set_rows.push(exercise.id);
                doc.set_positions.push(set_position as i32);
                doc.set_kg
                    .push(set.kg.map(|kg| (kg * 100.0).round() / 100.0));
                doc.reps_min.push(set.reps_min);
                doc.reps_max.push(set.reps_max);
            }
        }
        Ok(doc)
    }
}

/// Trims; empty means none.
fn clean_text(text: Option<String>, max_chars: usize) -> AppResult<Option<String>> {
    let text = text
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty());
    if text
        .as_ref()
        .is_some_and(|text| text.chars().count() > max_chars)
    {
        return Err(AppError::BadRequest("invalid_text"));
    }
    Ok(text)
}
