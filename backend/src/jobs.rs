//! Background work in the same process as the API, so it costs no extra
//! service. Every few minutes it queues workout-day reminders and Dasha's
//! evening summary, then sends whatever the bot still owes, including messages
//! that failed earlier. A Postgres advisory lock keeps it to one instance.

use std::time::Duration;

use chrono::{DateTime, NaiveTime, Utc};
use tokio::time::MissedTickBehavior;

use crate::{
    form_videos,
    notify::{self, Kind},
    questionnaire,
    state::AppState,
};

const EVERY: Duration = Duration::from_secs(5 * 60);
/// Any constant works; it only has to be the same for every instance.
const LOCK_KEY: i64 = 0x636f_6163_685f_6a6f;

/// Reminders go out from 09:00 in the client's timezone, and not after 21:00
/// if the server was down all day.
const REMINDER_FROM: NaiveTime = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
const REMINDER_UNTIL: NaiveTime = NaiveTime::from_hms_opt(21, 0, 0).unwrap();
/// A workout published this recently was just announced; no reminder on top.
const ANNOUNCED_HOURS: i32 = 12;
/// Dasha's summary goes out from 20:00 in her timezone.
const SUMMARY_FROM: NaiveTime = NaiveTime::from_hms_opt(20, 0, 0).unwrap();
const SUMMARY_UNTIL: NaiveTime = NaiveTime::from_hms_opt(22, 0, 0).unwrap();
/// Clients without a timezone yet are assumed to be in Ukraine. So is anyone
/// whose stored timezone Postgres does not know, rather than stopping the
/// whole round with an error.
const DEFAULT_TIMEZONE: &str = "Europe/Kyiv";

/// A failed message is tried again after this long, up to `MAX_ATTEMPTS`
/// times, and not once it is a day old.
const RETRY_AFTER_MINS: i32 = 10;
const MAX_ATTEMPTS: i32 = 8;
const BATCH: i64 = 50;

/// Runs forever; spawn it next to the server.
pub async fn run(state: AppState) {
    let mut ticker = tokio::time::interval(EVERY);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        ticker.tick().await;
        if let Err(err) = tick(&state, Utc::now()).await {
            tracing::warn!(error = ?err, "background jobs failed; trying again next round");
        }
    }
}

/// One round, as of `now`. Skipped if another instance is running one.
pub async fn tick(state: &AppState, now: DateTime<Utc>) -> anyhow::Result<()> {
    let mut lock = state.db.acquire().await?;
    let locked = sqlx::query_scalar!(r#"SELECT pg_try_advisory_lock($1) AS "locked!""#, LOCK_KEY)
        .fetch_one(&mut *lock)
        .await?;
    if !locked {
        return Ok(());
    }
    let result = round(state, now).await;
    sqlx::query_scalar!("SELECT pg_advisory_unlock($1)", LOCK_KEY)
        .fetch_one(&mut *lock)
        .await?;
    result
}

async fn round(state: &AppState, now: DateTime<Utc>) -> anyhow::Result<()> {
    warn_unknown_timezones(state).await?;
    queue_reminders(state, now).await?;
    queue_summaries(state, now).await?;
    // Clients' videos that finished encoding without a webhook; tells Dasha.
    form_videos::refresh_processing(state, now).await?;
    questionnaire::refresh_processing(state).await?;
    send_due(state, now).await
}

/// The API stores only names Postgres knows, but a Postgres upgrade can drop
/// old ones. Their owners get `DEFAULT_TIMEZONE` until the names are fixed.
async fn warn_unknown_timezones(state: &AppState) -> sqlx::Result<()> {
    let unknown = sqlx::query_scalar!(
        r#"SELECT timezone AS "timezone!" FROM clients WHERE timezone IS NOT NULL
           UNION SELECT timezone FROM coaches
           EXCEPT SELECT name FROM pg_timezone_names"#
    )
    .fetch_all(&state.db)
    .await?;
    if !unknown.is_empty() {
        tracing::warn!(
            ?unknown,
            instead = DEFAULT_TIMEZONE,
            "Postgres does not know these stored timezones"
        );
    }
    Ok(())
}

/// One reminder per published workout, on its date, in the client's morning.
async fn queue_reminders(state: &AppState, now: DateTime<Utc>) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO notifications (kind, entity_id, local_date, created_at)
         SELECT $1, w.id, w.date, $2
         FROM workouts w
         JOIN clients c ON c.id = w.client_id
         LEFT JOIN pg_timezone_names tz ON tz.name = c.timezone
         CROSS JOIN LATERAL (
             SELECT $2::timestamptz AT TIME ZONE COALESCE(tz.name, $3) AS local_now
         ) client_time
         WHERE w.status = 'published'
           AND c.telegram_id IS NOT NULL AND c.archived_at IS NULL
           AND (c.bot_allowed_at IS NOT NULL
                OR EXISTS (SELECT 1 FROM push_subscriptions p WHERE p.client_id = c.id))
           AND w.date = client_time.local_now::date
           AND client_time.local_now::time BETWEEN $4 AND $5
           AND w.published_at < $2 - make_interval(hours => $6)
         ON CONFLICT (kind, entity_id, local_date) DO NOTHING",
        Kind::WorkoutReminder.as_str(),
        now,
        DEFAULT_TIMEZONE,
        REMINDER_FROM,
        REMINDER_UNTIL,
        ANNOUNCED_HOURS,
    )
    .execute(&state.db)
    .await?;
    Ok(())
}

/// One evening summary per coach and day; it is skipped if there is nothing to say.
async fn queue_summaries(state: &AppState, now: DateTime<Utc>) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO notifications (kind, entity_id, local_date, created_at)
         SELECT $1, co.id, coach_time.local_now::date, $2
         FROM coaches co
         LEFT JOIN pg_timezone_names tz ON tz.name = co.timezone
         CROSS JOIN LATERAL (
             SELECT $2::timestamptz AT TIME ZONE COALESCE(tz.name, $3) AS local_now
         ) coach_time
         WHERE coach_time.local_now::time BETWEEN $4 AND $5
         ON CONFLICT (kind, entity_id, local_date) DO NOTHING",
        Kind::CoachDaily.as_str(),
        now,
        DEFAULT_TIMEZONE,
        SUMMARY_FROM,
        SUMMARY_UNTIL,
    )
    .execute(&state.db)
    .await?;
    Ok(())
}

/// Sends new rows, and retries failed ones after a pause.
async fn send_due(state: &AppState, now: DateTime<Utc>) -> anyhow::Result<()> {
    let due = sqlx::query_scalar!(
        "SELECT id FROM notifications
         WHERE sent_at IS NULL AND attempts < $2
           AND created_at > $1::timestamptz - interval '1 day'
           AND (last_attempt_at IS NULL
                OR last_attempt_at < $1::timestamptz - make_interval(mins => $3))
         ORDER BY created_at
         LIMIT $4",
        now,
        MAX_ATTEMPTS,
        RETRY_AFTER_MINS,
        BATCH,
    )
    .fetch_all(&state.db)
    .await?;
    let count = due.len();
    for id in due {
        // One failure (Telegram down, a blocked bot) must not hold up the rest.
        if let Err(err) = notify::deliver(state, id, now).await {
            tracing::warn!(error = ?err, %id, "could not send a bot message; will retry");
        }
    }
    tracing::debug!(due = count, "background round done");
    Ok(())
}
