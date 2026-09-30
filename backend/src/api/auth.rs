use axum::{Json, extract::State};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{
        Role,
        jwt::{CLIENT_TOKEN_TTL, COACH_TOKEN_TTL},
        telegram::{LoginWidgetPayload, verify_login_widget, verify_web_app_init_data},
    },
    error::{AppError, AppResult, ErrorBody},
    state::AppState,
};

#[derive(Deserialize, ToSchema)]
pub struct WebAppAuthRequest {
    /// `Telegram.WebApp.initData`, exactly as the Mini App received it.
    pub init_data: String,
}

#[derive(Serialize, ToSchema)]
pub struct ClientProfile {
    pub id: Uuid,
    pub name: String,
    /// IANA timezone reported by the client's phone, e.g. `Europe/Kyiv`.
    pub timezone: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ClientSession {
    pub token: String,
    pub client: ClientProfile,
}

#[derive(Serialize, ToSchema)]
pub struct CoachProfile {
    pub id: Uuid,
    pub name: String,
}

#[derive(Serialize, ToSchema)]
pub struct CoachSession {
    pub token: String,
    pub coach: CoachProfile,
}

/// Mini App sign-in: trades Telegram-signed `initData` for a session token.
#[utoipa::path(
    post,
    path = "/auth/telegram-webapp",
    tag = "auth",
    request_body = WebAppAuthRequest,
    responses(
        (status = 200, body = ClientSession),
        (status = 401, body = ErrorBody, description = "`initData` is invalid or older than a day"),
        (status = 403, body = ErrorBody, description = "`not_a_client`: this Telegram user has no invite"),
    )
)]
pub async fn telegram_webapp(
    State(state): State<AppState>,
    Json(request): Json<WebAppAuthRequest>,
) -> AppResult<Json<ClientSession>> {
    let init_data = verify_web_app_init_data(
        &request.init_data,
        &state.config.bot_token,
        Utc::now().timestamp(),
    )
    .map_err(|err| {
        tracing::debug!(%err, "rejected Mini App initData");
        AppError::Unauthorized
    })?;

    let client = sqlx::query_as!(
        ClientProfile,
        "SELECT id, name, timezone FROM clients
         WHERE telegram_id = $1 AND archived_at IS NULL",
        init_data.user.id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        // The ID is what the coach needs to add this person (see README, "Local Telegram").
        tracing::info!(
            telegram_id = init_data.user.id,
            "Mini App sign-in by a non-client"
        );
        AppError::Forbidden("not_a_client")
    })?;

    let token = state.jwt.issue(Role::Client, client.id, CLIENT_TOKEN_TTL)?;
    Ok(Json(ClientSession { token, client }))
}

/// Coach sign-in with the Telegram Login Widget.
#[utoipa::path(
    post,
    path = "/auth/telegram-login",
    tag = "auth",
    request_body = LoginWidgetPayload,
    responses(
        (status = 200, body = CoachSession),
        (status = 401, body = ErrorBody, description = "The payload is invalid or older than a day"),
        (status = 403, body = ErrorBody, description = "`not_a_coach`: this Telegram user is not a coach"),
    )
)]
pub async fn telegram_login(
    State(state): State<AppState>,
    Json(payload): Json<LoginWidgetPayload>,
) -> AppResult<Json<CoachSession>> {
    verify_login_widget(&payload, &state.config.bot_token, Utc::now().timestamp()).map_err(
        |err| {
            tracing::debug!(%err, "rejected Login Widget payload");
            AppError::Unauthorized
        },
    )?;

    let coach = sqlx::query_as!(
        CoachProfile,
        "SELECT id, name FROM coaches WHERE telegram_id = $1",
        payload.id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        tracing::info!(telegram_id = payload.id, "coach sign-in by a non-coach");
        AppError::Forbidden("not_a_coach")
    })?;

    let token = state.jwt.issue(Role::Coach, coach.id, COACH_TOKEN_TTL)?;
    Ok(Json(CoachSession { token, coach }))
}
