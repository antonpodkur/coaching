//! The client's weight log, and Dasha's view of it.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDate;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{CurrentClient, CurrentCoach},
    error::{AppError, AppResult, ErrorBody},
    state::AppState,
    weight::{self, WeightEntry},
};

#[derive(Deserialize, ToSchema)]
pub struct LogWeight {
    pub kg: f64,
}

/// The client's weigh-ins, oldest first.
#[utoipa::path(
    get,
    operation_id = "get_my_weight",
    path = "/me/weight",
    tag = "client",
    security(("bearer" = [])),
    responses(
        (status = 200, body = Vec<WeightEntry>),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn list_mine(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
) -> AppResult<Json<Vec<WeightEntry>>> {
    Ok(Json(weight::entries(&state, client_id).await?))
}

/// Records the client's weight on a day, replacing that day's entry.
#[utoipa::path(
    put,
    operation_id = "log_my_weight",
    path = "/me/weight/{date}",
    tag = "client",
    security(("bearer" = [])),
    params(("date" = NaiveDate, Path, description = "The client's date, e.g. 2026-10-05")),
    request_body = LogWeight,
    responses(
        (status = 200, body = WeightEntry),
        (status = 400, body = ErrorBody, description = "`invalid_weight` (20–400 kg) or `invalid_date` (in the future)"),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn log(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
    Path(date): Path<NaiveDate>,
    Json(body): Json<LogWeight>,
) -> AppResult<Json<WeightEntry>> {
    Ok(Json(weight::log(&state, client_id, date, body.kg).await?))
}

/// Deletes the client's entry for a day.
#[utoipa::path(
    delete,
    operation_id = "delete_my_weight",
    path = "/me/weight/{date}",
    tag = "client",
    security(("bearer" = [])),
    params(("date" = NaiveDate, Path, description = "The entry's date")),
    responses(
        (status = 204),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
    Path(date): Path<NaiveDate>,
) -> AppResult<StatusCode> {
    weight::delete(&state, client_id, date).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// One client's weigh-ins, oldest first, for Dasha.
#[utoipa::path(
    get,
    operation_id = "get_client_weight",
    path = "/coach/clients/{id}/weight",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Client id")),
    responses(
        (status = 200, body = Vec<WeightEntry>),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn list_for_coach(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(client_id): Path<Uuid>,
) -> AppResult<Json<Vec<WeightEntry>>> {
    let theirs = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM clients WHERE id = $1 AND coach_id = $2) AS "theirs!""#,
        client_id,
        coach_id,
    )
    .fetch_one(&state.db)
    .await?;
    if !theirs {
        return Err(AppError::NotFound);
    }
    Ok(Json(weight::entries(&state, client_id).await?))
}
