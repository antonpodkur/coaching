use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    api::auth::ClientProfile,
    auth::CurrentClient,
    bot,
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
        r#"SELECT id, name, timezone, bot_allowed_at IS NOT NULL AS "bot_allowed!",
                  (birth_year IS NOT NULL OR sex IS NOT NULL OR height_cm IS NOT NULL
                   OR EXISTS (SELECT 1 FROM gym_media m WHERE m.client_id = clients.id))
                  AS "questionnaire_started!"
           FROM clients WHERE id = $1 AND archived_at IS NULL"#,
        client_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(profile))
}

/// The client allowed the bot to message them, in Telegram's popup in the app.
/// The first time, the bot sends its pinned welcome.
#[utoipa::path(
    post,
    operation_id = "allow_bot_messages",
    path = "/me/bot-allowed",
    tag = "client",
    security(("bearer" = [])),
    responses(
        (status = 204),
        (status = 401, body = ErrorBody),
        (status = 503, body = ErrorBody, description = "`bot_cannot_write`: Telegram refused, so messages are not allowed after all"),
    )
)]
pub async fn allow_bot(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
) -> AppResult<StatusCode> {
    bot::welcome_client(&state, client_id, None)
        .await
        .map_err(|err| {
            tracing::warn!(error = ?err, "could not welcome a client in the bot");
            AppError::Unavailable("bot_cannot_write")
        })?;
    Ok(StatusCode::NO_CONTENT)
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
