use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{
        Role,
        bot_login::{self, Polled},
        jwt::{BROWSER_TOKEN_TTL, MINI_APP_TOKEN_TTL},
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
pub struct CoachProfile {
    pub id: Uuid,
    pub name: String,
    /// IANA timezone of Dasha's phone; her evening summary follows it.
    pub timezone: String,
}

/// Who opened the Mini App decides which screens they get.
#[derive(Serialize, ToSchema)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum MiniAppSession {
    /// Dasha's workspace. Checked first, so an account that is both gets this.
    Coach { token: String, coach: CoachProfile },
    Client {
        token: String,
        client: ClientProfile,
    },
}

/// Mini App sign-in: trades Telegram-signed `initData` for a session token.
#[utoipa::path(
    post,
    path = "/auth/telegram-webapp",
    tag = "auth",
    request_body = WebAppAuthRequest,
    responses(
        (status = 200, body = MiniAppSession),
        (status = 401, body = ErrorBody, description = "`initData` is invalid or older than a day"),
        (status = 403, body = ErrorBody, description = "`not_invited`: neither the coach nor an invited client"),
    )
)]
pub async fn telegram_webapp(
    State(state): State<AppState>,
    Json(request): Json<WebAppAuthRequest>,
) -> AppResult<Json<MiniAppSession>> {
    let init_data = verify_web_app_init_data(
        &request.init_data,
        &state.config.bot_token,
        Utc::now().timestamp(),
    )
    .map_err(|err| {
        tracing::debug!(%err, "rejected Mini App initData");
        AppError::Unauthorized
    })?;
    let telegram_id = init_data.user.id;

    let coach = sqlx::query_as!(
        CoachProfile,
        "SELECT id, name, timezone FROM coaches WHERE telegram_id = $1",
        telegram_id,
    )
    .fetch_optional(&state.db)
    .await?;
    if let Some(coach) = coach {
        let token = state.jwt.issue(Role::Coach, coach.id, MINI_APP_TOKEN_TTL)?;
        return Ok(Json(MiniAppSession::Coach { token, coach }));
    }

    let client = sqlx::query_as!(
        ClientProfile,
        "SELECT id, name, timezone FROM clients
         WHERE telegram_id = $1 AND archived_at IS NULL",
        telegram_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        // The ID is what the coach needs to add this person (see README, "Local Telegram").
        tracing::info!(telegram_id, "Mini App sign-in by someone without an invite");
        AppError::Forbidden("not_invited")
    })?;
    let token = state
        .jwt
        .issue(Role::Client, client.id, MINI_APP_TOKEN_TTL)?;
    Ok(Json(MiniAppSession::Client { token, client }))
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
                "SELECT id, name, timezone FROM coaches WHERE id = $1",
                coach_id,
            )
            .fetch_one(&state.db)
            .await?;
            let token = state.jwt.issue(Role::Coach, coach.id, BROWSER_TOKEN_TTL)?;
            BotLoginPoll::Approved { token, coach }
        }
    }))
}
