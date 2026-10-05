//! A client's weight over time: they log it, and both they and Dasha see it
//! charted. One entry a day; weighing again the same day replaces it.

use chrono::{Duration, NaiveDate, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

const MIN_KG: f64 = 20.0;
const MAX_KG: f64 = 400.0;

#[derive(Serialize, ToSchema)]
pub struct WeightEntry {
    /// The client's own calendar date.
    pub date: NaiveDate,
    pub kg: f64,
}

/// All of a client's weigh-ins, oldest first.
pub async fn entries(state: &AppState, client_id: Uuid) -> sqlx::Result<Vec<WeightEntry>> {
    sqlx::query_as!(
        WeightEntry,
        r#"SELECT measured_on AS date, value::float8 AS "kg!"
           FROM body_measurements
           WHERE client_id = $1 AND kind = 'weight'
           ORDER BY measured_on"#,
        client_id,
    )
    .fetch_all(&state.db)
    .await
}

/// Records the client's weight on `date`, replacing that day's entry.
pub async fn log(
    state: &AppState,
    client_id: Uuid,
    date: NaiveDate,
    kg: f64,
) -> AppResult<WeightEntry> {
    if !kg.is_finite() || !(MIN_KG..=MAX_KG).contains(&kg) {
        return Err(AppError::BadRequest("invalid_weight"));
    }
    // The client's today may be a day ahead of the server's.
    let latest = Utc::now().date_naive() + Duration::days(1);
    let earliest = NaiveDate::from_ymd_opt(2000, 1, 1).expect("a valid date");
    if !(earliest..=latest).contains(&date) {
        return Err(AppError::BadRequest("invalid_date"));
    }
    let entry = sqlx::query_as!(
        WeightEntry,
        r#"INSERT INTO body_measurements (client_id, kind, value, measured_on)
           VALUES ($1, 'weight', $2::float8::numeric(6, 2), $3)
           ON CONFLICT (client_id, kind, measured_on)
           DO UPDATE SET value = EXCLUDED.value, created_at = now()
           RETURNING measured_on AS date, value::float8 AS "kg!""#,
        client_id,
        kg,
        date,
    )
    .fetch_one(&state.db)
    .await?;
    Ok(entry)
}

/// Deletes the client's entry for `date`.
pub async fn delete(state: &AppState, client_id: Uuid, date: NaiveDate) -> AppResult<()> {
    let deleted = sqlx::query!(
        "DELETE FROM body_measurements
         WHERE client_id = $1 AND kind = 'weight' AND measured_on = $2",
        client_id,
        date,
    )
    .execute(&state.db)
    .await?;
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}
