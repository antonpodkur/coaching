use axum::{Json, body::Bytes, extract::State, http::StatusCode};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    api::client::SetTimezone,
    auth::CurrentCoach,
    avatars,
    error::{AppResult, ErrorBody},
    photos::PhotoFile,
    state::AppState,
};

#[derive(Serialize, ToSchema)]
pub struct CoachAvatar {
    /// Signed for a few hours.
    pub url: String,
}

/// Stores the timezone of Dasha's phone, so her evening summary arrives at 20:00
/// her time wherever she is. An old name such as `Europe/Kiev` is stored under
/// its current one.
#[utoipa::path(
    put,
    operation_id = "set_coach_timezone",
    path = "/coach/me/timezone",
    tag = "coach",
    security(("bearer" = [])),
    request_body = SetTimezone,
    responses(
        (status = 204),
        (status = 400, body = ErrorBody, description = "`unknown_timezone`"),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn set_timezone(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Json(body): Json<SetTimezone>,
) -> AppResult<StatusCode> {
    let timezone = body.storable(&state.db).await?;
    sqlx::query!(
        "UPDATE coaches SET timezone = $2 WHERE id = $1",
        coach_id,
        timezone,
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The coach's photo, a square JPEG the phone already shrank. Her clients see
/// it next to her workouts and comments.
#[utoipa::path(
    put,
    operation_id = "set_coach_avatar",
    path = "/coach/me/avatar",
    tag = "coach",
    security(("bearer" = [])),
    request_body(content = PhotoFile, content_type = "image/jpeg"),
    responses(
        (status = 200, body = CoachAvatar),
        (status = 400, body = ErrorBody, description = "`invalid_photo`: not a JPEG, or too big"),
        (status = 401, body = ErrorBody),
        (status = 503, body = ErrorBody, description = "`photos_not_configured`"),
    )
)]
pub async fn set_avatar(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    jpeg: Bytes,
) -> AppResult<Json<CoachAvatar>> {
    let url = avatars::upload_coach(&state, coach_id, jpeg.to_vec()).await?;
    Ok(Json(CoachAvatar { url }))
}

/// Removes the coach's photo.
#[utoipa::path(
    delete,
    operation_id = "remove_coach_avatar",
    path = "/coach/me/avatar",
    tag = "coach",
    security(("bearer" = [])),
    responses(
        (status = 204),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn remove_avatar(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
) -> AppResult<StatusCode> {
    avatars::remove_coach(&state, coach_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
