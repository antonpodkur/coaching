use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    auth::{CurrentSession, Role},
    error::{AppError, AppResult, ErrorBody},
    push,
    state::AppState,
};

#[derive(Serialize, ToSchema)]
pub struct PushKey {
    /// For `pushManager.subscribe({ applicationServerKey })`, URL-safe base64.
    pub public_key: String,
}

/// The key a browser needs to subscribe to web push.
#[utoipa::path(
    get,
    path = "/push/key",
    tag = "push",
    responses(
        (status = 200, body = PushKey),
        (status = 503, body = ErrorBody, description = "`push_not_configured`"),
    )
)]
pub async fn key(State(state): State<AppState>) -> AppResult<Json<PushKey>> {
    let push = state
        .push
        .as_ref()
        .ok_or(AppError::Unavailable("push_not_configured"))?;
    Ok(Json(PushKey {
        public_key: push.public_key().to_owned(),
    }))
}

/// A browser's push subscription, as `PushSubscription.toJSON()` gives it.
#[derive(Deserialize, ToSchema)]
pub struct NewSubscription {
    pub endpoint: String,
    pub keys: SubscriptionKeys,
}

#[derive(Deserialize, ToSchema)]
pub struct SubscriptionKeys {
    pub p256dh: String,
    pub auth: String,
}

/// This phone gets the signed-in person's notifications from now on, also if
/// it got someone else's before.
#[utoipa::path(
    put,
    path = "/push/subscription",
    tag = "push",
    security(("bearer" = [])),
    request_body = NewSubscription,
    responses(
        (status = 204),
        (status = 400, body = ErrorBody, description = "`invalid_subscription`: not a browser's push service, or not its keys"),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn subscribe(
    State(state): State<AppState>,
    CurrentSession(claims): CurrentSession,
    Json(subscription): Json<NewSubscription>,
) -> AppResult<StatusCode> {
    let keys = &subscription.keys;
    if !push::known_push_service(&subscription.endpoint)
        || !push::valid_keys(&keys.p256dh, &keys.auth)
    {
        return Err(AppError::BadRequest("invalid_subscription"));
    }
    let (client_id, coach_id) = match claims.role {
        Role::Client => (Some(claims.sub), None),
        Role::Coach => (None, Some(claims.sub)),
    };
    sqlx::query!(
        "INSERT INTO push_subscriptions (client_id, coach_id, endpoint, p256dh, auth)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (endpoint) DO UPDATE
         SET client_id = EXCLUDED.client_id, coach_id = EXCLUDED.coach_id,
             p256dh = EXCLUDED.p256dh, auth = EXCLUDED.auth",
        client_id,
        coach_id,
        subscription.endpoint,
        keys.p256dh,
        keys.auth,
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, ToSchema)]
pub struct OldSubscription {
    pub endpoint: String,
}

/// This phone stops getting the signed-in person's notifications: they
/// signed out here, or turned notifications off.
#[utoipa::path(
    delete,
    path = "/push/subscription",
    tag = "push",
    security(("bearer" = [])),
    request_body = OldSubscription,
    responses(
        (status = 204),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn unsubscribe(
    State(state): State<AppState>,
    CurrentSession(claims): CurrentSession,
    Json(subscription): Json<OldSubscription>,
) -> AppResult<StatusCode> {
    sqlx::query!(
        "DELETE FROM push_subscriptions
         WHERE endpoint = $1
           AND CASE WHEN $2 THEN coach_id = $3 ELSE client_id = $3 END",
        subscription.endpoint,
        claims.role == Role::Coach,
        claims.sub,
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
