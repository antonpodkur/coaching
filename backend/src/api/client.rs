use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    api::auth::ClientProfile,
    auth::CurrentClient,
    error::{AppError, AppResult, ErrorBody},
    state::AppState,
};

/// The signed-in client's profile.
#[utoipa::path(
    get,
    path = "/me",
    tag = "client",
    security(("bearer" = [])),
    responses(
        (status = 200, body = ClientProfile),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn me(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
) -> AppResult<Json<ClientProfile>> {
    let profile = sqlx::query_as!(
        ClientProfile,
        "SELECT id, name, timezone FROM clients WHERE id = $1 AND archived_at IS NULL",
        client_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(profile))
}

#[derive(Deserialize, ToSchema)]
pub struct SetTimezone {
    /// IANA name from `Intl.DateTimeFormat().resolvedOptions().timeZone`.
    pub timezone: String,
}

/// Stores the phone's timezone so reminders arrive at the client's local time.
#[utoipa::path(
    put,
    path = "/me/timezone",
    tag = "client",
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
    CurrentClient(client_id): CurrentClient,
    Json(body): Json<SetTimezone>,
) -> AppResult<StatusCode> {
    let timezone = body.parse()?;
    sqlx::query!(
        "UPDATE clients SET timezone = $2 WHERE id = $1",
        client_id,
        timezone.name(),
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

impl SetTimezone {
    /// Anything but an IANA name is refused, so `AT TIME ZONE` never fails on it.
    pub(crate) fn parse(&self) -> AppResult<chrono_tz::Tz> {
        self.timezone
            .parse()
            .map_err(|_| AppError::BadRequest("unknown_timezone"))
    }
}
