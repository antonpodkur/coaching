//! Nutrition targets: Dasha sets them, the client reads them.

use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{CurrentClient, CurrentCoach},
    error::{AppError, AppResult, ErrorBody},
    notify,
    nutrition::{self, NewTarget, NutritionTarget},
    state::AppState,
};

#[derive(Serialize, ToSchema)]
pub struct MyNutrition {
    /// `null` until Dasha sets a target.
    pub current: Option<NutritionTarget>,
}

#[derive(Serialize, ToSchema)]
pub struct NutritionHistory {
    /// Newest first; the first is the current target.
    pub targets: Vec<NutritionTarget>,
}

#[derive(Serialize, ToSchema)]
pub struct SavedTarget {
    pub target: NutritionTarget,
    /// The bot is telling the client. `false` if it cannot write to them;
    /// they still see the target in the app.
    pub client_notified: bool,
}

/// The client's daily nutrition target from Dasha.
#[utoipa::path(
    get,
    operation_id = "get_my_nutrition",
    path = "/me/nutrition",
    tag = "client",
    security(("bearer" = [])),
    responses(
        (status = 200, body = MyNutrition),
        (status = 401, body = ErrorBody),
    )
)]
pub async fn get_mine(
    State(state): State<AppState>,
    CurrentClient(client_id): CurrentClient,
) -> AppResult<Json<MyNutrition>> {
    Ok(Json(MyNutrition {
        current: nutrition::current(&state, client_id).await?,
    }))
}

/// A client's nutrition targets, the current one first.
#[utoipa::path(
    get,
    operation_id = "get_client_nutrition",
    path = "/coach/clients/{id}/nutrition",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Client id")),
    responses(
        (status = 200, body = NutritionHistory),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn history(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(client_id): Path<Uuid>,
) -> AppResult<Json<NutritionHistory>> {
    check_client(&state, coach_id, client_id).await?;
    Ok(Json(NutritionHistory {
        targets: nutrition::history(&state, client_id).await?,
    }))
}

/// Sets a client's daily target, replacing the current one, and the bot tells
/// the client the new numbers.
#[utoipa::path(
    post,
    operation_id = "set_client_nutrition",
    path = "/coach/clients/{id}/nutrition",
    tag = "coach",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Client id")),
    request_body = NewTarget,
    responses(
        (status = 200, body = SavedTarget),
        (status = 400, body = ErrorBody, description = "`invalid_grams` (0–1000 each, not all 0) or `note_too_long`"),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
pub async fn set(
    State(state): State<AppState>,
    CurrentCoach(coach_id): CurrentCoach,
    Path(client_id): Path<Uuid>,
    Json(new): Json<NewTarget>,
) -> AppResult<Json<SavedTarget>> {
    check_client(&state, coach_id, client_id).await?;
    let target = nutrition::set(&state, client_id, &new).await?;
    let client_notified = notify::nutrition_changed(&state, target.id).await?;
    Ok(Json(SavedTarget {
        target,
        client_notified,
    }))
}

async fn check_client(state: &AppState, coach_id: Uuid, client_id: Uuid) -> AppResult<()> {
    let theirs = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM clients WHERE id = $1 AND coach_id = $2) AS "theirs!""#,
        client_id,
        coach_id,
    )
    .fetch_one(&state.db)
    .await?;
    if theirs {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}
