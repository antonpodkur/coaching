use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{
        CurrentSession, Role,
        bot_login::{self, Polled},
        jwt::{BROWSER_TOKEN_TTL, MINI_APP_TOKEN_TTL},
        telegram::verify_web_app_init_data,
    },
    avatars::{self, Avatar},
    bot,
    error::{AppError, AppResult, ErrorBody},
    invites::{self, Accepted},
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
    /// The bot may message them. Joining through an app link skips the bot's
    /// Start, so until they allow it the app asks (`requestWriteAccess`).
    pub bot_allowed: bool,
    /// They answered something in the questionnaire or added a gym photo or
    /// video; until then the app offers it.
    pub questionnaire_started: bool,
    /// They use the app installed on a phone's home screen, so the Telegram
    /// version stops offering it.
    pub app_installed: bool,
    /// The photo they added in their profile, or else their Telegram photo.
    pub avatar: Option<Avatar>,
}

#[derive(Serialize, ToSchema)]
pub struct CoachProfile {
    pub id: Uuid,
    pub name: String,
    /// IANA timezone of Dasha's phone; her evening summary follows it.
    pub timezone: String,
    /// She uses the app installed on a phone's home screen.
    pub app_installed: bool,
}

/// Who signed in decides which screens they get.
#[derive(Serialize, ToSchema)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum AppSession {
    /// Dasha's workspace. Checked first, so an account that is both gets this.
    Coach { token: String, coach: CoachProfile },
    Client {
        token: String,
        client: ClientProfile,
    },
}

async fn coach_by_telegram(db: &PgPool, telegram_id: i64) -> sqlx::Result<Option<CoachProfile>> {
    sqlx::query_as!(
        CoachProfile,
        r#"SELECT id, name, timezone, app_installed_at IS NOT NULL AS "app_installed!"
           FROM coaches WHERE telegram_id = $1"#,
        telegram_id,
    )
    .fetch_optional(db)
    .await
}

async fn coach_by_id(db: &PgPool, coach_id: Uuid) -> sqlx::Result<Option<CoachProfile>> {
    sqlx::query_as!(
        CoachProfile,
        r#"SELECT id, name, timezone, app_installed_at IS NOT NULL AS "app_installed!"
           FROM coaches WHERE id = $1"#,
        coach_id,
    )
    .fetch_optional(db)
    .await
}

/// The client who joined with this Telegram account, unless archived.
async fn client_by_telegram(db: &PgPool, telegram_id: i64) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar!(
        "SELECT id FROM clients WHERE telegram_id = $1 AND archived_at IS NULL",
        telegram_id,
    )
    .fetch_optional(db)
    .await
}

/// A client's profile, unless they were archived.
pub async fn client_profile(
    state: &AppState,
    client_id: Uuid,
) -> sqlx::Result<Option<ClientProfile>> {
    let row = sqlx::query!(
        r#"SELECT id, name, timezone, bot_allowed_at IS NOT NULL AS "bot_allowed!",
                  (birth_year IS NOT NULL OR sex IS NOT NULL OR height_cm IS NOT NULL
                   OR EXISTS (SELECT 1 FROM gym_media m WHERE m.client_id = clients.id))
                  AS "questionnaire_started!",
                  app_installed_at IS NOT NULL AS "app_installed!",
                  avatar_path, avatar_from_telegram
           FROM clients WHERE id = $1 AND archived_at IS NULL"#,
        client_id,
    )
    .fetch_optional(&state.db)
    .await?;
    Ok(row.map(|row| ClientProfile {
        avatar: avatars::avatar(
            state,
            row.id,
            row.avatar_path.as_deref(),
            row.avatar_from_telegram,
        ),
        id: row.id,
        name: row.name,
        timezone: row.timezone,
        bot_allowed: row.bot_allowed,
        questionnaire_started: row.questionnaire_started,
        app_installed: row.app_installed,
    }))
}

fn coach_session(state: &AppState, coach: CoachProfile, ttl: Duration) -> AppResult<AppSession> {
    let token = state.jwt.issue(Role::Coach, coach.id, ttl)?;
    Ok(AppSession::Coach { token, coach })
}

fn client_session(state: &AppState, client: ClientProfile, ttl: Duration) -> AppResult<AppSession> {
    let token = state.jwt.issue(Role::Client, client.id, ttl)?;
    Ok(AppSession::Client { token, client })
}

/// Mini App sign-in: trades Telegram-signed `initData` for a session token.
///
/// When the app was opened by an invite link (`?startapp=inv_<code>`, which
/// arrives as `start_param`), the client joins here, without the bot.
#[utoipa::path(
    post,
    path = "/auth/telegram-webapp",
    tag = "auth",
    request_body = WebAppAuthRequest,
    responses(
        (status = 200, body = AppSession),
        (status = 401, body = ErrorBody, description = "`initData` is invalid or older than a day"),
        (status = 403, body = ErrorBody, description = "`not_invited`: neither the coach nor an invited client; `invite_invalid`: the invite link is used or expired; `linked_elsewhere`: this account already belongs to another client"),
    )
)]
pub async fn telegram_webapp(
    State(state): State<AppState>,
    Json(request): Json<WebAppAuthRequest>,
) -> AppResult<Json<AppSession>> {
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

    let coach = coach_by_telegram(&state.db, telegram_id).await?;
    // Dasha opening an invite link herself, to check it, must not use it up.
    if let Some(coach) = coach {
        // Kept for the clients' "Написати Даші" button.
        sqlx::query!(
            "UPDATE coaches SET username = $2 WHERE id = $1 AND username IS DISTINCT FROM $2",
            coach.id,
            init_data.user.username,
        )
        .execute(&state.db)
        .await?;
        return Ok(Json(coach_session(&state, coach, MINI_APP_TOKEN_TTL)?));
    }

    let invite = init_data
        .start_param
        .as_deref()
        .and_then(|param| param.strip_prefix(invites::START_PREFIX));
    let mut invite_unusable = false;
    if let Some(code) = invite {
        match invites::accept(&state.db, code, telegram_id).await? {
            Accepted::Joined {
                client_id,
                client_name,
                coach_telegram_id,
            } => bot::announce_join(&state, client_id, &client_name, coach_telegram_id).await,
            Accepted::LinkedElsewhere => return Err(AppError::Forbidden("linked_elsewhere")),
            // Fine for someone who already joined and tapped the link again.
            Accepted::Invalid => invite_unusable = true,
        }
    }

    let client_id = client_by_telegram(&state.db, telegram_id).await?;
    let client = match client_id {
        Some(client_id) => client_profile(&state, client_id).await?,
        None => None,
    };
    let mut client = client.ok_or_else(|| {
        if invite_unusable {
            return AppError::Forbidden("invite_invalid");
        }
        // The ID is what the coach needs to add this person (see README, "Local Telegram").
        tracing::info!(telegram_id, "Mini App sign-in by someone without an invite");
        AppError::Forbidden("not_invited")
    })?;

    // Allowed in Telegram's own dialog, e.g. when the app link opened: welcome now.
    if !client.bot_allowed && init_data.user.allows_write_to_pm {
        match bot::welcome_client(&state, client.id, Some(&init_data.user.first_name)).await {
            Ok(_) => client.bot_allowed = true,
            Err(err) => tracing::warn!(error = ?err, "could not welcome a client in the bot"),
        }
    }

    Ok(Json(client_session(&state, client, MINI_APP_TOKEN_TTL)?))
}

#[derive(Serialize, ToSchema)]
pub struct BotLoginStart {
    /// Keep in the app; send it to `/auth/bot-login/poll` to collect the session.
    pub poll_secret: String,
    /// Opens the bot with the login code: `t.me`, for a computer.
    pub bot_url: String,
    /// The same, straight in the Telegram app (`tg://`), for a phone.
    pub bot_app_url: String,
    /// Shown in the app; the bot shows the same digits before they confirm.
    pub display_code: String,
    pub expires_at: DateTime<Utc>,
}

/// Starts a sign-in outside Telegram, which the person confirms in the bot.
#[utoipa::path(
    post,
    path = "/auth/bot-login",
    tag = "auth",
    responses((status = 200, body = BotLoginStart))
)]
pub async fn bot_login_start(State(state): State<AppState>) -> AppResult<Json<BotLoginStart>> {
    let login = bot_login::start(&state.db).await?;
    let payload = format!("{}{}", bot_login::START_PREFIX, login.code);
    Ok(Json(BotLoginStart {
        poll_secret: login.poll_secret,
        bot_url: state.config.bot_start_url(&payload),
        bot_app_url: state.config.bot_app_start_url(&payload),
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
    /// They pressed Cancel in the bot.
    Cancelled,
    /// The Telegram account that opened the link has no access: neither the
    /// coach nor a client who joined.
    Refused,
    /// Timed out, unknown, or the session was already collected.
    Expired,
    /// Confirmed. The session is handed out only once.
    Approved { session: AppSession },
}

/// Collects the result of a sign-in started with `/auth/bot-login`: the
/// coach's or the client's session, whoever confirmed it.
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
        Polled::Refused => BotLoginPoll::Refused,
        Polled::Expired => BotLoginPoll::Expired,
        Polled::Approved { telegram_id } => {
            let session = if let Some(coach) = coach_by_telegram(&state.db, telegram_id).await? {
                Some(coach_session(&state, coach, BROWSER_TOKEN_TTL)?)
            } else if let Some(client_id) = client_by_telegram(&state.db, telegram_id).await?
                && let Some(client) = client_profile(&state, client_id).await?
            {
                Some(client_session(&state, client, BROWSER_TOKEN_TTL)?)
            } else {
                None
            };
            // Archived between confirming and collecting.
            session.map_or(BotLoginPoll::Refused, |session| BotLoginPoll::Approved {
                session,
            })
        }
    }))
}

/// Renews a session, which an app outside Telegram does each time it opens,
/// for as long again as the session it replaces. Refused (401) once the coach
/// or client is gone or archived, which signs them out.
#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "auth",
    security(("bearer" = [])),
    responses(
        (status = 200, body = AppSession),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    CurrentSession(claims): CurrentSession,
) -> AppResult<Json<AppSession>> {
    let ttl = Duration::seconds(claims.exp - claims.iat);
    let session = match claims.role {
        Role::Coach => {
            let coach = coach_by_id(&state.db, claims.sub).await?;
            coach_session(&state, coach.ok_or(AppError::Unauthorized)?, ttl)?
        }
        Role::Client => {
            let client = client_profile(&state, claims.sub).await?;
            client_session(&state, client.ok_or(AppError::Unauthorized)?, ttl)?
        }
    };
    Ok(Json(session))
}

/// The app runs installed on a phone's home screen, for whoever is signed in.
/// The Telegram version then stops offering to install it.
#[utoipa::path(
    post,
    path = "/auth/installed",
    tag = "auth",
    security(("bearer" = [])),
    responses(
        (status = 204),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn installed(
    State(state): State<AppState>,
    CurrentSession(claims): CurrentSession,
) -> AppResult<StatusCode> {
    match claims.role {
        Role::Coach => {
            sqlx::query!(
                "UPDATE coaches SET app_installed_at = now()
                 WHERE id = $1 AND app_installed_at IS NULL",
                claims.sub,
            )
            .execute(&state.db)
            .await?;
        }
        Role::Client => {
            sqlx::query!(
                "UPDATE clients SET app_installed_at = now()
                 WHERE id = $1 AND app_installed_at IS NULL",
                claims.sub,
            )
            .execute(&state.db)
            .await?;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}
