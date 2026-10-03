//! A client's own technique videos under an exercise of their workout.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::coach::exercises::UploadTicket,
    auth::CurrentClient,
    error::{AppResult, ErrorBody},
    form_videos,
    state::AppState,
    video::TUS_ENDPOINT,
};

#[derive(Serialize, ToSchema)]
pub struct FormVideoUpload {
    /// The new video; tell `/form-videos/{id}/uploaded` when the file is sent.
    pub id: Uuid,
    /// Send as the tus `title` metadata, so Bunny keeps this name.
    pub title: String,
    pub ticket: UploadTicket,
}

/// Starts uploading a video of how the client did this exercise: creates it on
/// Bunny and signs a tus upload straight from the phone. Up to three per
/// exercise in a workout.
#[utoipa::path(
    post,
    operation_id = "start_form_video",
    path = "/workout-exercises/{id}/videos",
    tag = "client",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The exercise's ID in the workout")),
    responses(
        (status = 200, body = FormVideoUpload),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody, description = "`too_many_videos`"),
        (status = 503, body = ErrorBody, description = "`client_videos_not_configured`"),
    )
)]
pub async fn start(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
    Path(workout_exercise_id): Path<Uuid>,
) -> AppResult<Json<FormVideoUpload>> {
    let (id, title, grant) = form_videos::start(&state, client_id, workout_exercise_id).await?;
    let library_id = form_videos::stream(&state)?.settings().library_id.clone();
    Ok(Json(FormVideoUpload {
        id,
        title,
        ticket: UploadTicket {
            endpoint: TUS_ENDPOINT.to_owned(),
            library_id,
            video_id: grant.video_id,
            expires_at: grant.expires_at,
            signature: grant.signature,
        },
    }))
}

/// The phone finished sending the file; Bunny encodes it next, then Dasha is told.
#[utoipa::path(
    post,
    operation_id = "finish_form_video",
    path = "/form-videos/{id}/uploaded",
    tag = "client",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Form video id")),
    responses(
        (status = 204),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn finish(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    form_videos::finish(&state, client_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The client deletes one of their videos, on Bunny too.
#[utoipa::path(
    delete,
    operation_id = "delete_form_video",
    path = "/form-videos/{id}",
    tag = "client",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Form video id")),
    responses(
        (status = 204),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    form_videos::delete(&state, client_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
