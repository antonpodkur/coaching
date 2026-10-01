use axum::{Json, extract::State, http::StatusCode};

use crate::{
    api::client::SetTimezone,
    auth::CurrentCoach,
    error::{AppResult, ErrorBody},
    state::AppState,
};

/// Stores the timezone of Dasha's phone, so her evening summary arrives at 20:00
/// her time wherever she is.
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
    let timezone = body.parse()?;
    sqlx::query!(
        "UPDATE coaches SET timezone = $2 WHERE id = $1",
        coach_id,
        timezone.name(),
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
