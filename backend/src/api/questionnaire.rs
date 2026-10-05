//! The questionnaire a client fills in for Dasha, and her view of it.

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::{coach::exercises::UploadTicket, form_videos::FormVideoUpload},
    auth::{CurrentClient, CurrentCoach},
    error::{AppError, AppResult, ErrorBody},
    form_videos::{self, Viewer},
    questionnaire::{self, Answers, GymPhoto, Questionnaire},
    state::AppState,
    video::TUS_ENDPOINT,
};

/// A photo file's bytes as the request body. Only describes the body in the
/// API, so the field is never read.
#[derive(ToSchema)]
#[schema(value_type = String, format = Binary)]
#[allow(dead_code)]
pub struct PhotoFile(Vec<u8>);

/// The client's own questionnaire.
#[utoipa::path(
    get,
    operation_id = "get_my_questionnaire",
    path = "/me/questionnaire",
    tag = "client",
    security(("bearer" = [])),
    responses(
        (status = 200, body = Questionnaire),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn get_mine(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
) -> AppResult<Json<Questionnaire>> {
    Ok(Json(
        questionnaire::get(&state, client_id, Viewer::Client).await?,
    ))
}

/// Saves the client's answers: birth year, sex and height.
#[utoipa::path(
    put,
    operation_id = "save_my_answers",
    path = "/me/questionnaire",
    tag = "client",
    security(("bearer" = [])),
    request_body = Answers,
    responses(
        (status = 200, body = Questionnaire),
        (status = 400, body = ErrorBody, description = "`invalid_birth_year` or `invalid_height`"),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn save_answers(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
    Json(answers): Json<Answers>,
) -> AppResult<Json<Questionnaire>> {
    questionnaire::save_answers(&state, client_id, &answers).await?;
    Ok(Json(
        questionnaire::get(&state, client_id, Viewer::Client).await?,
    ))
}

/// Adds a photo of the client's gym: a JPEG shrunk on the phone, as the body.
/// Up to ten.
#[utoipa::path(
    post,
    operation_id = "add_gym_photo",
    path = "/me/gym/photos",
    tag = "client",
    security(("bearer" = [])),
    request_body(content = PhotoFile, content_type = "image/jpeg"),
    responses(
        (status = 201, body = GymPhoto),
        (status = 400, body = ErrorBody, description = "`invalid_photo`: not a JPEG, or too big"),
        (status = 401, body = ErrorBody),
        (status = 409, body = ErrorBody, description = "`too_many_photos`"),
        (status = 503, body = ErrorBody, description = "`photos_not_configured`"),
    )
)]
pub async fn add_photo(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
    jpeg: Bytes,
) -> AppResult<(StatusCode, Json<GymPhoto>)> {
    let photo = questionnaire::add_photo(&state, client_id, jpeg.to_vec()).await?;
    Ok((StatusCode::CREATED, Json(photo)))
}

/// Starts uploading a video of the client's gym: creates it on Bunny and signs
/// a tus upload straight from the phone. Up to three.
#[utoipa::path(
    post,
    operation_id = "start_gym_video",
    path = "/me/gym/videos",
    tag = "client",
    security(("bearer" = [])),
    responses(
        (status = 200, body = FormVideoUpload),
        (status = 401, body = ErrorBody),
        (status = 409, body = ErrorBody, description = "`too_many_videos`"),
        (status = 503, body = ErrorBody, description = "`client_videos_not_configured`"),
    )
)]
pub async fn start_video(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
) -> AppResult<Json<FormVideoUpload>> {
    let (id, title, grant) = questionnaire::start_video(&state, client_id).await?;
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

/// The phone finished sending a gym video; Bunny encodes it next.
#[utoipa::path(
    post,
    operation_id = "finish_gym_video",
    path = "/me/gym/videos/{id}/uploaded",
    tag = "client",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Gym video id")),
    responses(
        (status = 204),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn finish_video(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    questionnaire::finish_video(&state, client_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The client deletes a photo or video of their gym, on Bunny too.
#[utoipa::path(
    delete,
    operation_id = "delete_gym_media",
    path = "/me/gym/{id}",
    tag = "client",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Gym photo or video id")),
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
    questionnaire::delete(&state, client_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// One client's questionnaire, for Dasha.
#[utoipa::path(
    get,
    operation_id = "get_client_questionnaire",
    path = "/coach/clients/{id}/questionnaire",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Client id")),
    responses(
        (status = 200, body = Questionnaire),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn get_for_coach(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(client_id): Path<Uuid>,
) -> AppResult<Json<Questionnaire>> {
    let theirs = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM clients WHERE id = $1 AND coach_id = $2) AS "theirs!""#,
        client_id,
        coach_id,
    )
    .fetch_one(&state.db)
    .await?;
    if !theirs {
        return Err(AppError::NotFound);
    }
    Ok(Json(
        questionnaire::get(&state, client_id, Viewer::Coach).await?,
    ))
}
