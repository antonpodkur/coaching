use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::CurrentCoach,
    error::{AppError, AppResult, ErrorBody},
    invites,
    state::AppState,
};

const MAX_NAME_CHARS: usize = 80;

#[derive(Serialize, ToSchema)]
pub struct CoachClient {
    pub id: Uuid,
    pub name: String,
    /// The client opened their invite and is linked to a Telegram account.
    pub joined: bool,
    /// Set while an unused invite exists; it may already have expired.
    pub invite_expires_at: Option<DateTime<Utc>>,
    pub paid_until: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct InviteLink {
    /// `https://t.me/<bot>?start=inv_<code>`. Works once.
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct NewClient {
    pub name: String,
}

#[derive(Serialize, ToSchema)]
pub struct CreatedClient {
    pub client: CoachClient,
    pub invite: InviteLink,
}

/// All of the coach's clients, newest first.
#[utoipa::path(
    get,
    operation_id = "list_clients",
    path = "/coach/clients",
    tag = "coach",
    security(("bearer" = [])),
    responses(
        (status = 200, body = Vec<CoachClient>),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
) -> AppResult<Json<Vec<CoachClient>>> {
    let clients = sqlx::query_as!(
        CoachClient,
        r#"SELECT id, name, telegram_id IS NOT NULL AS "joined!", invite_expires_at, paid_until,
                  created_at
           FROM clients
           WHERE coach_id = $1 AND archived_at IS NULL
           ORDER BY created_at DESC"#,
        coach_id,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(clients))
}

/// Adds a client and returns their first invite link.
#[utoipa::path(
    post,
    operation_id = "create_client",
    path = "/coach/clients",
    tag = "coach",
    security(("bearer" = [])),
    request_body = NewClient,
    responses(
        (status = 201, body = CreatedClient),
        (status = 400, body = ErrorBody, description = "`invalid_name`"),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Json(body): Json<NewClient>,
) -> AppResult<(StatusCode, Json<CreatedClient>)> {
    let name = body.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(AppError::BadRequest("invalid_name"));
    }

    let client_id = sqlx::query_scalar!(
        "INSERT INTO clients (coach_id, name) VALUES ($1, $2) RETURNING id",
        coach_id,
        name,
    )
    .fetch_one(&state.db)
    .await?;
    let invite = invites::issue(&state.db, coach_id, client_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let client = fetch_client(&state, coach_id, client_id).await?;

    Ok((
        StatusCode::CREATED,
        Json(CreatedClient {
            client,
            invite: invite_link(&state, invite),
        }),
    ))
}

/// A fresh invite link. The previous unused link stops working.
///
/// For a client who already joined, the new link re-links the profile to whoever
/// opens it, e.g. after they switched Telegram accounts.
#[utoipa::path(
    post,
    operation_id = "reinvite_client",
    path = "/coach/clients/{id}/invite",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Client id")),
    responses(
        (status = 200, body = InviteLink),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn reinvite(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(client_id): Path<Uuid>,
) -> AppResult<Json<InviteLink>> {
    let invite = invites::issue(&state.db, coach_id, client_id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(invite_link(&state, invite)))
}

fn invite_link(state: &AppState, invite: invites::Invite) -> InviteLink {
    InviteLink {
        url: state
            .config
            .bot_start_url(&format!("{}{}", invites::START_PREFIX, invite.code)),
        expires_at: invite.expires_at,
    }
}

async fn fetch_client(state: &AppState, coach_id: Uuid, client_id: Uuid) -> AppResult<CoachClient> {
    sqlx::query_as!(
        CoachClient,
        r#"SELECT id, name, telegram_id IS NOT NULL AS "joined!", invite_expires_at, paid_until,
                  created_at
           FROM clients WHERE id = $1 AND coach_id = $2"#,
        client_id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)
}
