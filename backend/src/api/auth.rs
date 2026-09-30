use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{
        Role,
        bot_login::{self, Polled},
        jwt::{CLIENT_TOKEN_TTL, COACH_TOKEN_TTL},
        telegram::verify_web_app_init_data,
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

#[derive(Serialize, ToSchema)]
pub struct BotLoginStart {
    /// Keep in the page; send it to `/auth/bot-login/poll` to collect the token.
    pub poll_secret: String,
    /// Opens the bot in Telegram with the login code.
    pub bot_url: String,
    /// Shown on the page; the bot shows the same digits before the coach confirms.
    pub display_code: String,
    pub expires_at: DateTime<Utc>,
}

/// Starts a coach sign-in that the coach confirms in the bot.
#[utoipa::path(
    post,
    path = "/auth/bot-login",
    tag = "auth",
    responses((status = 200, body = BotLoginStart))
)]
pub async fn bot_login_start(State(state): State<AppState>) -> AppResult<Json<BotLoginStart>> {
    let login = bot_login::start(&state.db).await?;
    Ok(Json(BotLoginStart {
        poll_secret: login.poll_secret,
        bot_url: state
            .config
            .bot_start_url(&format!("{}{}", bot_login::START_PREFIX, login.code)),
        display_code: login.display_code,
        expires_at: login.expires_at,
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct BotLoginPollRequest {
    pub poll_secret: String,
}

#[derive(Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BotLoginPoll {
    /// Not confirmed in the bot yet; ask again in a couple of seconds.
    Pending,
    /// The coach pressed Cancel in the bot.
    Cancelled,
    /// Timed out, unknown, or the token was already collected.
    Expired,
    /// Confirmed. The token is handed out only once.
    Approved { token: String, coach: CoachProfile },
}

/// Collects the result of a coach sign-in started with `/auth/bot-login`.
#[utoipa::path(
    post,
    path = "/auth/bot-login/poll",
    tag = "auth",
    request_body = BotLoginPollRequest,
    responses((status = 200, body = BotLoginPoll))
)]
pub async fn bot_login_poll(
    State(state): State<AppState>,
    Json(request): Json<BotLoginPollRequest>,
) -> AppResult<Json<BotLoginPoll>> {
    let polled = bot_login::poll(&state.db, &request.poll_secret).await?;
    Ok(Json(match polled {
        Polled::Pending => BotLoginPoll::Pending,
        Polled::Cancelled => BotLoginPoll::Cancelled,
        Polled::Expired => BotLoginPoll::Expired,
        Polled::Approved { coach_id } => {
            let coach = sqlx::query_as!(
                CoachProfile,
                "SELECT id, name FROM coaches WHERE id = $1",
                coach_id,
            )
            .fetch_one(&state.db)
            .await?;
            let token = state.jwt.issue(Role::Coach, coach.id, COACH_TOKEN_TTL)?;
            BotLoginPoll::Approved { token, coach }
        }
    }))
}
