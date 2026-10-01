//! Dasha's own account. There is no sign-up for coaches: `COACH_TELEGRAM_ID`
//! names her Telegram account, and the coach is created on startup.

use sqlx::PgPool;

/// What the bot calls her in clients' messages.
pub const DEFAULT_NAME: &str = "Даша";

/// Creates the coach for this Telegram account unless there already is one.
/// Returns whether it was created.
pub async fn ensure(db: &PgPool, telegram_id: i64) -> sqlx::Result<bool> {
    let inserted = sqlx::query!(
        "INSERT INTO coaches (telegram_id, name) VALUES ($1, $2)
         ON CONFLICT (telegram_id) DO NOTHING",
        telegram_id,
        DEFAULT_NAME,
    )
    .execute(db)
    .await?;
    Ok(inserted.rows_affected() == 1)
}
