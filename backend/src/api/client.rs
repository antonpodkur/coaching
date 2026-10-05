use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::{
    api::auth::{self, ClientProfile},
    auth::CurrentClient,
    bot,
    error::{AppError, AppResult, ErrorBody},
    state::AppState,
    timezone,
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
    let profile = auth::client_profile(&state.db, client_id).await?;
    Ok(Json(profile.ok_or(AppError::NotFound)?))
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
/// An old name such as `Europe/Kiev` is stored under its current one.
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
    let timezone = body.storable(&state.db).await?;
    sqlx::query!(
        "UPDATE clients SET timezone = $2 WHERE id = $1",
        client_id,
        timezone,
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

impl SetTimezone {
    /// The name to store, or `unknown_timezone`; see [`timezone::storable`].
    pub(crate) async fn storable(&self, db: &PgPool) -> AppResult<&'static str> {
        timezone::storable(db, &self.timezone)
            .await?
            .ok_or(AppError::BadRequest("unknown_timezone"))
    }
}
