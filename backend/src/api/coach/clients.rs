use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    auth::CurrentCoach,
    error::{AppError, AppResult, ErrorBody},
    invites,
    state::AppState,
    telegram::{Button, ShareableMessage},
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
    /// Reports Dasha has not opened yet.
    pub unseen_reports: i64,
    /// Ready videos from the client that Dasha has not watched yet.
    pub new_videos: i64,
    /// Hidden from the list; the client cannot open the app until restored.
    pub archived: bool,
}

#[derive(Serialize, ToSchema)]
pub struct InviteLink {
    /// `https://t.me/<bot>?startapp=inv_<code>`: opens the app straight away and
    /// joins. Works once.
    pub url: String,
    pub expires_at: DateTime<Utc>,
    /// The invitation as a message card with an "Відкрити" button, for
    /// `Telegram.WebApp.shareMessage` in Dasha's Mini App. `null` if Telegram
    /// could not prepare it; then share `url`.
    pub prepared_message_id: Option<String>,
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

#[derive(Deserialize, IntoParams)]
pub struct ClientFilter {
    /// `true` lists the archive instead, most recently archived first.
    #[serde(default)]
    pub archived: bool,
}

/// Changes to one client. Fields left out stay as they are.
#[derive(Deserialize, ToSchema)]
pub struct ClientChanges {
    pub name: Option<String>,
    /// The last day the client has paid for. `null` clears it.
    #[serde(default, deserialize_with = "present")]
    #[schema(value_type = Option<NaiveDate>)]
    pub paid_until: Option<Option<NaiveDate>>,
    /// `true` archives the client and ends their sessions; `false` restores them.
    pub archived: Option<bool>,
}

/// Tells a field sent as `null` (clear it) from one left out (keep it).
fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// The coach's clients, newest first. Archived clients only on request.
#[utoipa::path(
    get,
    operation_id = "list_clients",
    path = "/coach/clients",
    tag = "coach",
    security(("bearer" = [])),
    params(ClientFilter),
    responses(
        (status = 200, body = Vec<CoachClient>),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Query(filter): Query<ClientFilter>,
) -> AppResult<Json<Vec<CoachClient>>> {
    let clients = sqlx::query_as!(
        CoachClient,
        r#"SELECT c.id, c.name, c.telegram_id IS NOT NULL AS "joined!", c.invite_expires_at,
                  c.paid_until, c.created_at,
                  (SELECT count(*) FROM workout_reports r JOIN workouts w ON w.id = r.workout_id
                   WHERE w.client_id = c.id AND r.seen_at IS NULL) AS "unseen_reports!",
                  (SELECT count(*) FROM form_videos v
                   WHERE v.client_id = c.id AND v.status = 'ready' AND v.seen_at IS NULL)
                   AS "new_videos!",
                  c.archived_at IS NOT NULL AS "archived!"
           FROM clients c
           WHERE c.coach_id = $1 AND (c.archived_at IS NOT NULL) = $2
           ORDER BY c.archived_at DESC NULLS LAST, c.created_at DESC"#,
        coach_id,
        filter.archived,
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
    let name = clean_name(&body.name)?;

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
    let invite = invite_link(&state, coach_id, invite).await?;

    Ok((StatusCode::CREATED, Json(CreatedClient { client, invite })))
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
    Ok(Json(invite_link(&state, coach_id, invite).await?))
}

async fn invite_link(
    state: &AppState,
    coach_id: Uuid,
    invite: invites::Invite,
) -> AppResult<InviteLink> {
    let url = state
        .config
        .app_start_url(&format!("{}{}", invites::START_PREFIX, invite.code));
    let coach_telegram_id =
        sqlx::query_scalar!("SELECT telegram_id FROM coaches WHERE id = $1", coach_id)
            .fetch_one(&state.db)
            .await?;
    let card = ShareableMessage {
        title: "Запрошення до тренувань".to_owned(),
        description: "Онлайн-тренування з Дарією Хижняк".to_owned(),
        text: format!(
            "Запрошення до онлайн-тренувань з Дарією Хижняк.\n\n\
             У застосунку — твої тренування, відео техніки до вправ і звіт після кожного \
             тренування. Натисни «Відкрити», і він відкриється просто в Telegram.\n\n\
             Посилання особисте й діє {} днів.",
            invites::INVITE_TTL.num_days()
        ),
        button: Button::Url {
            text: "Відкрити".to_owned(),
            url: url.clone(),
        },
    };
    // The plain link still works without the card, so a failure only logs.
    let prepared_message_id = match state
        .telegram
        .save_prepared_message(coach_telegram_id, card)
        .await
    {
        Ok(id) => Some(id),
        Err(err) => {
            tracing::warn!(error = ?err, "could not prepare the invite card");
            None
        }
    };
    Ok(InviteLink {
        url,
        expires_at: invite.expires_at,
        prepared_message_id,
    })
}

fn clean_name(name: &str) -> AppResult<&str> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(AppError::BadRequest("invalid_name"));
    }
    Ok(name)
}

/// One client, archived or not.
#[utoipa::path(
    get,
    operation_id = "get_client",
    path = "/coach/clients/{id}",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Client id")),
    responses(
        (status = 200, body = CoachClient),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn get(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(client_id): Path<Uuid>,
) -> AppResult<Json<CoachClient>> {
    Ok(Json(fetch_client(&state, coach_id, client_id).await?))
}

/// Renames a client, records how long they have paid for, or archives them.
///
/// Archiving keeps their workouts and reports for Dasha. The client is signed
/// out, gets no more bot messages, and their unused invite stops working until
/// they are restored.
#[utoipa::path(
    patch,
    operation_id = "update_client",
    path = "/coach/clients/{id}",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Client id")),
    request_body = ClientChanges,
    responses(
        (status = 200, body = CoachClient),
        (status = 400, body = ErrorBody, description = "`invalid_name`"),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(client_id): Path<Uuid>,
    Json(changes): Json<ClientChanges>,
) -> AppResult<Json<CoachClient>> {
    let name = changes.name.as_deref().map(clean_name).transpose()?;
    let updated = sqlx::query!(
        "UPDATE clients SET
             name = COALESCE($3, name),
             paid_until = CASE WHEN $4 THEN $5 ELSE paid_until END,
             archived_at = CASE
                 WHEN $6::boolean IS NULL THEN archived_at
                 WHEN $6 THEN COALESCE(archived_at, now())
                 ELSE NULL
             END
         WHERE id = $1 AND coach_id = $2",
        client_id,
        coach_id,
        name,
        changes.paid_until.is_some(),
        changes.paid_until.flatten(),
        changes.archived,
    )
    .execute(&state.db)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(fetch_client(&state, coach_id, client_id).await?))
}

async fn fetch_client(state: &AppState, coach_id: Uuid, client_id: Uuid) -> AppResult<CoachClient> {
    sqlx::query_as!(
        CoachClient,
        r#"SELECT c.id, c.name, c.telegram_id IS NOT NULL AS "joined!", c.invite_expires_at,
                  c.paid_until, c.created_at,
                  (SELECT count(*) FROM workout_reports r JOIN workouts w ON w.id = r.workout_id
                   WHERE w.client_id = c.id AND r.seen_at IS NULL) AS "unseen_reports!",
                  (SELECT count(*) FROM form_videos v
                   WHERE v.client_id = c.id AND v.status = 'ready' AND v.seen_at IS NULL)
                   AS "new_videos!",
                  c.archived_at IS NOT NULL AS "archived!"
           FROM clients c WHERE c.id = $1 AND c.coach_id = $2"#,
        client_id,
        coach_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)
}
