use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::CurrentCoach,
    error::{AppError, AppResult, ErrorBody},
    state::AppState,
    video::{self, UploadStatus},
};

const MAX_NAME_CHARS: usize = 120;
const MAX_GROUP_CHARS: usize = 40;

/// How an exercise's sets are counted. Seconds share the reps fields, so for
/// `time` a set's `reps_*` and `actual_reps` are seconds.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type,
)]
#[sqlx(type_name = "exercise_measure", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Measure {
    /// Kilograms × reps.
    #[default]
    Weight,
    /// Reps; kilograms only as optional extra weight, e.g. a belt on pull-ups.
    Bodyweight,
    /// Seconds, e.g. a plank; kilograms only as optional extra weight.
    Time,
}

#[derive(Serialize, ToSchema)]
pub struct Exercise {
    pub id: Uuid,
    pub name: String,
    pub measure: Measure,
    /// Free text; the app offers a fixed set (Спина, Ноги, …).
    pub muscle_group: Option<String>,
    /// Other spellings, used to match imported Telegram plans.
    pub aliases: Vec<String>,
    /// The video clients see; `null` until the first upload is ready.
    pub video: Option<ExerciseVideo>,
    /// The newest upload while it is in flight or being encoded, or after it failed.
    pub upload: Option<VideoUpload>,
}

#[derive(Serialize, ToSchema)]
pub struct ExerciseVideo {
    pub length_secs: Option<i32>,
    /// HLS playlist. iOS plays it natively; elsewhere the app uses hls.js.
    pub hls_url: String,
    pub thumbnail_url: String,
}

#[derive(Serialize, ToSchema)]
pub struct VideoUpload {
    pub status: UploadStatus,
    pub started_at: DateTime<Utc>,
}

/// An `exercises` row as the queries below select it.
pub(crate) struct ExerciseRow {
    pub id: Uuid,
    pub name: String,
    pub measure: Measure,
    pub muscle_group: Option<String>,
    pub aliases: Vec<String>,
    pub video_uid: Option<String>,
    pub video_length_secs: Option<i32>,
    pub upload_status: Option<UploadStatus>,
    pub upload_started_at: Option<DateTime<Utc>>,
}

impl ExerciseRow {
    fn into_exercise(self, state: &AppState) -> Exercise {
        // Without Bunny settings there are no URLs to hand out, so no video.
        let video = self
            .video_uid
            .zip(state.video.as_ref())
            .map(|(uid, stream)| ExerciseVideo {
                length_secs: self.video_length_secs,
                hls_url: stream.hls_url(&uid),
                thumbnail_url: stream.thumbnail_url(&uid),
            });
        let upload = self
            .upload_status
            .zip(self.upload_started_at)
            .map(|(status, started_at)| VideoUpload { status, started_at });
        Exercise {
            id: self.id,
            name: self.name,
            measure: self.measure,
            muscle_group: self.muscle_group,
            aliases: self.aliases,
            video,
            upload,
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct NewExercise {
    pub name: String,
    pub muscle_group: Option<String>,
    /// `weight` when left out.
    #[serde(default)]
    pub measure: Measure,
}

/// Changes to an exercise; fields left out stay as they are.
#[derive(Deserialize, ToSchema)]
pub struct ExerciseChanges {
    pub name: Option<String>,
    /// An empty string clears the group.
    pub muscle_group: Option<String>,
    /// Switching to or from `time` is refused once the exercise has sets,
    /// since their numbers would change meaning (reps vs seconds).
    pub measure: Option<Measure>,
    /// Hides it from the library. Old workouts keep showing it.
    pub archived: Option<bool>,
}

/// The coach's library, alphabetical. Archived exercises are left out.
#[utoipa::path(
    get,
    operation_id = "list_exercises",
    path = "/coach/exercises",
    tag = "coach",
    security(("bearer" = [])),
    responses(
        (status = 200, body = Vec<Exercise>),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
) -> AppResult<Json<Vec<Exercise>>> {
    video::refresh_processing(&state, coach_id).await?;
    let rows = sqlx::query_as!(
        ExerciseRow,
        r#"SELECT id, name, measure AS "measure: Measure", muscle_group, aliases, video_uid,
                  video_length_secs, upload_status AS "upload_status: UploadStatus",
                  upload_started_at
           FROM exercises
           WHERE coach_id = $1 AND archived_at IS NULL
           ORDER BY lower(name)"#,
        coach_id,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| row.into_exercise(&state))
            .collect(),
    ))
}

/// Adds an exercise to the library. A video can follow later.
#[utoipa::path(
    post,
    operation_id = "create_exercise",
    path = "/coach/exercises",
    tag = "coach",
    security(("bearer" = [])),
    request_body = NewExercise,
    responses(
        (status = 201, body = Exercise),
        (status = 400, body = ErrorBody, description = "`invalid_name` or `invalid_group`"),
        (status = 401, body = ErrorBody),
        (status = 409, body = ErrorBody, description = "`name_taken`: the library already has it"),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Json(body): Json<NewExercise>,
) -> AppResult<(StatusCode, Json<Exercise>)> {
    let name = clean_name(&body.name)?;
    let group = clean_group(body.muscle_group.as_deref())?;
    let id = sqlx::query_scalar!(
        "INSERT INTO exercises (coach_id, name, muscle_group, measure)
         VALUES ($1, $2, $3, $4) RETURNING id",
        coach_id,
        name,
        group,
        body.measure as Measure,
    )
    .fetch_one(&state.db)
    .await
    .map_err(name_taken)?;
    let exercise = fetch(&state, coach_id, id).await?;
    Ok((StatusCode::CREATED, Json(exercise)))
}

/// One exercise. While its video is being encoded, this also asks Bunny how far
/// it got, so the app sees the result even if Bunny's webhook is late.
#[utoipa::path(
    get,
    operation_id = "get_exercise",
    path = "/coach/exercises/{id}",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Exercise id")),
    responses(
        (status = 200, body = Exercise),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn get(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Exercise>> {
    let exercise = fetch(&state, coach_id, id).await?;
    if exercise
        .upload
        .as_ref()
        .is_some_and(|upload| upload.status == UploadStatus::Processing)
    {
        video::refresh_exercise(&state, id).await?;
        return Ok(Json(fetch(&state, coach_id, id).await?));
    }
    Ok(Json(exercise))
}

/// Renames, regroups or archives an exercise.
#[utoipa::path(
    patch,
    operation_id = "update_exercise",
    path = "/coach/exercises/{id}",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Exercise id")),
    request_body = ExerciseChanges,
    responses(
        (status = 200, body = Exercise),
        (status = 400, body = ErrorBody, description = "`invalid_name` or `invalid_group`"),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody, description = "`name_taken`, or `measure_in_use`: it has sets in reps and cannot switch to seconds, or back"),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(id): Path<Uuid>,
    Json(changes): Json<ExerciseChanges>,
) -> AppResult<Json<Exercise>> {
    let name = changes.name.as_deref().map(clean_name).transpose()?;
    let set_group = changes.muscle_group.is_some();
    let group = clean_group(changes.muscle_group.as_deref())?;
    if let Some(measure) = changes.measure {
        check_measure_change(&state, coach_id, id, measure).await?;
    }
    let updated = sqlx::query_scalar!(
        r#"UPDATE exercises SET
             name = COALESCE($3, name),
             muscle_group = CASE WHEN $4 THEN $5 ELSE muscle_group END,
             measure = COALESCE($7, measure),
             archived_at = CASE
                 WHEN $6::boolean IS NULL THEN archived_at
                 WHEN $6 THEN COALESCE(archived_at, now())
                 ELSE NULL
             END
         WHERE id = $1 AND coach_id = $2
         RETURNING id"#,
        id,
        coach_id,
        name,
        set_group,
        group,
        changes.archived,
        changes.measure as Option<Measure>,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(name_taken)?;
    if updated.is_none() {
        return Err(AppError::NotFound);
    }
    Ok(Json(fetch(&state, coach_id, id).await?))
}

/// Between weight and bodyweight the numbers mean the same (kg, reps), so that
/// switch is always fine. To or from time, reps would turn into seconds.
async fn check_measure_change(
    state: &AppState,
    coach_id: Uuid,
    id: Uuid,
    measure: Measure,
) -> AppResult<()> {
    let current = sqlx::query!(
        r#"SELECT e.measure AS "measure: Measure",
                  EXISTS (SELECT 1 FROM workout_exercises we
                          JOIN workout_sets s ON s.workout_exercise_id = we.id
                          WHERE we.exercise_id = e.id) AS "used!"
           FROM exercises e WHERE e.id = $1 AND e.coach_id = $2"#,
        id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let changes_unit = current.measure != measure
        && (current.measure == Measure::Time || measure == Measure::Time);
    if changes_unit && current.used {
        return Err(AppError::Conflict("measure_in_use"));
    }
    Ok(())
}

pub(crate) async fn fetch(state: &AppState, coach_id: Uuid, id: Uuid) -> AppResult<Exercise> {
    let row = sqlx::query_as!(
        ExerciseRow,
        r#"SELECT id, name, measure AS "measure: Measure", muscle_group, aliases, video_uid,
                  video_length_secs, upload_status AS "upload_status: UploadStatus",
                  upload_started_at
           FROM exercises
           WHERE id = $1 AND coach_id = $2"#,
        id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(row.into_exercise(state))
}

fn clean_name(name: &str) -> AppResult<String> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(AppError::BadRequest("invalid_name"));
    }
    Ok(name)
}

/// `None` and `""` both mean "no group".
fn clean_group(group: Option<&str>) -> AppResult<Option<String>> {
    let group = group.map(str::trim).filter(|group| !group.is_empty());
    if group.is_some_and(|group| group.chars().count() > MAX_GROUP_CHARS) {
        return Err(AppError::BadRequest("invalid_group"));
    }
    Ok(group.map(str::to_owned))
}

/// The library's unique index on the lowercased name (among unarchived exercises).
fn name_taken(err: sqlx::Error) -> AppError {
    match &err {
        sqlx::Error::Database(db) if db.constraint() == Some("exercises_coach_name_key") => {
            AppError::Conflict("name_taken")
        }
        _ => err.into(),
    }
}

/// What the phone needs to upload one video straight to Bunny with tus.
#[derive(Serialize, ToSchema)]
pub struct UploadTicket {
    /// Bunny's tus endpoint.
    pub endpoint: String,
    /// Send as the `LibraryId` header.
    pub library_id: String,
    /// Send as the `VideoId` header.
    pub video_id: String,
    /// Unix seconds; send as the `AuthorizationExpire` header.
    pub expires_at: i64,
    /// Send as the `AuthorizationSignature` header.
    pub signature: String,
}

/// Starts a video upload for the exercise: creates the video on Bunny and signs
/// an upload into it. The current video stays until the new one is encoded.
#[utoipa::path(
    post,
    path = "/coach/exercises/{id}/video-upload",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Exercise id")),
    responses(
        (status = 200, body = UploadTicket),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 503, body = ErrorBody, description = "`video_not_configured`: no Bunny settings"),
    )
)]
pub async fn start_video_upload(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(id): Path<Uuid>,
) -> AppResult<Json<UploadTicket>> {
    let grant = video::start_upload(&state, coach_id, id).await?;
    let library_id = video::stream(&state)?.settings().library_id.clone();
    Ok(Json(UploadTicket {
        endpoint: video::TUS_ENDPOINT.to_owned(),
        library_id,
        video_id: grant.video_id,
        expires_at: grant.expires_at,
        signature: grant.signature,
    }))
}

/// The phone finished uploading; Bunny encodes the video next.
#[utoipa::path(
    post,
    path = "/coach/exercises/{id}/video-uploaded",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Exercise id")),
    responses(
        (status = 200, body = Exercise),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn finish_video_upload(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Exercise>> {
    video::finish_upload(&state, coach_id, id).await?;
    Ok(Json(fetch(&state, coach_id, id).await?))
}
