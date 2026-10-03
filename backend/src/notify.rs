//! Messages the bot sends on its own, not in reply to someone.
//!
//! Every message is first a row in `notifications`, unique per kind, subject
//! and date, so repeating a step never sends twice. The row is sent right away;
//! if Telegram fails, the background jobs retry it (see `jobs`). The text is
//! built when sending, from the data as it is then: a retried reminder for a
//! workout that has since been finished is skipped instead of sent.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use uuid::Uuid;

use crate::{
    api::workouts::Effort,
    state::AppState,
    telegram::{Button, OutgoingMessage},
};

const WEEKDAYS: [&str; 7] = ["пн", "вт", "ср", "чт", "пт", "сб", "нд"];
const MONTHS: [&str; 12] = [
    "січня",
    "лютого",
    "березня",
    "квітня",
    "травня",
    "червня",
    "липня",
    "серпня",
    "вересня",
    "жовтня",
    "листопада",
    "грудня",
];

/// A delivery holds its row this long, so the jobs and a request never send
/// the same message at once. Longer than Telegram's request timeout.
const LEASE_SECS: i64 = 120;

/// `вт, 6 жовтня`.
pub fn short_date(date: NaiveDate) -> String {
    format!(
        "{}, {} {}",
        WEEKDAYS[date.weekday().num_days_from_monday() as usize],
        date.day(),
        MONTHS[date.month0() as usize]
    )
}

/// `6 жовтня`.
fn day_month(date: NaiveDate) -> String {
    format!("{} {}", date.day(), MONTHS[date.month0() as usize])
}

/// What a `notifications` row is about; stored as its `kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// To the client; the subject is the workout.
    WorkoutPublished,
    /// To the client on the workout's day; the subject is the workout.
    WorkoutReminder,
    /// To Dasha; the subject is the workout.
    WorkoutFinished,
    /// To Dasha in the evening; the subject is the coach.
    CoachDaily,
    /// To Dasha when a client's technique video is ready; the subject is the video.
    FormVideo,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorkoutPublished => "workout_published",
            Self::WorkoutReminder => "workout_reminder",
            Self::WorkoutFinished => "workout_finished",
            Self::CoachDaily => "coach_daily",
            Self::FormVideo => "form_video",
        }
    }

    fn parse(kind: &str) -> Option<Self> {
        [
            Self::WorkoutPublished,
            Self::WorkoutReminder,
            Self::WorkoutFinished,
            Self::CoachDaily,
            Self::FormVideo,
        ]
        .into_iter()
        .find(|known| known.as_str() == kind)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Delivery {
    Sent,
    /// Nothing to send anymore; the row is closed.
    Skipped,
    /// Already sent, or another delivery holds it right now.
    NotDue,
}

/// Records a message to send, once per kind, subject and date. Returns the row
/// while it is still unsent.
pub async fn queue(
    state: &AppState,
    kind: Kind,
    entity_id: Uuid,
    local_date: NaiveDate,
) -> sqlx::Result<Option<Uuid>> {
    let row = sqlx::query!(
        "INSERT INTO notifications (kind, entity_id, local_date)
         VALUES ($1, $2, $3)
         ON CONFLICT (kind, entity_id, local_date) DO UPDATE SET kind = EXCLUDED.kind
         RETURNING id, sent_at",
        kind.as_str(),
        entity_id,
        local_date,
    )
    .fetch_one(&state.db)
    .await?;
    Ok(row.sent_at.is_none().then_some(row.id))
}

/// Sends one queued message, built from the data as it is now. A failed send
/// leaves the row for the jobs to retry and returns the error.
pub async fn deliver(state: &AppState, id: Uuid, now: DateTime<Utc>) -> anyhow::Result<Delivery> {
    let claimed = sqlx::query!(
        "UPDATE notifications SET attempts = attempts + 1, last_attempt_at = $2
         WHERE id = $1 AND sent_at IS NULL
           AND (last_attempt_at IS NULL
                OR last_attempt_at < $2::timestamptz - make_interval(secs => $3))
         RETURNING kind, entity_id, local_date",
        id,
        now,
        LEASE_SECS as f64,
    )
    .fetch_optional(&state.db)
    .await?;
    let Some(row) = claimed else {
        return Ok(Delivery::NotDue);
    };

    let message = match Kind::parse(&row.kind) {
        Some(Kind::WorkoutPublished) => published(state, row.entity_id).await?,
        Some(Kind::WorkoutReminder) => reminder(state, row.entity_id).await?,
        Some(Kind::WorkoutFinished) => finished(state, row.entity_id).await?,
        Some(Kind::CoachDaily) => daily_summary(state, row.entity_id, row.local_date).await?,
        Some(Kind::FormVideo) => form_video(state, row.entity_id).await?,
        None => None,
    };
    let Some(message) = message else {
        sqlx::query!(
            "UPDATE notifications SET sent_at = $2, skipped = true WHERE id = $1",
            id,
            now,
        )
        .execute(&state.db)
        .await?;
        return Ok(Delivery::Skipped);
    };

    state.telegram.send_message(message).await?;
    sqlx::query!(
        "UPDATE notifications SET sent_at = $2 WHERE id = $1",
        id,
        now,
    )
    .execute(&state.db)
    .await?;
    Ok(Delivery::Sent)
}

/// Queues the "new workout" message and sends it. Returns `false` if the client
/// has not joined yet, or has not let the bot write to them, so there is no one
/// to tell. A failed send is retried by the jobs, so it still counts as told.
pub async fn workout_published(state: &AppState, workout_id: Uuid) -> anyhow::Result<bool> {
    let workout = sqlx::query!(
        r#"SELECT w.date AS "date!", c.telegram_id IS NOT NULL AND c.bot_allowed_at IS NOT NULL
                  AS "reachable!"
           FROM workouts w JOIN clients c ON c.id = w.client_id
           WHERE w.id = $1 AND w.date IS NOT NULL"#,
        workout_id,
    )
    .fetch_one(&state.db)
    .await?;
    if !workout.reachable {
        return Ok(false);
    }
    if let Some(id) = queue(state, Kind::WorkoutPublished, workout_id, workout.date).await? {
        send_or_leave_for_retry(state, id).await;
    }
    Ok(true)
}

/// Queues Dasha's message about a finished workout and sends it.
pub async fn workout_finished(state: &AppState, workout_id: Uuid) -> anyhow::Result<()> {
    let date = sqlx::query_scalar!(
        r#"SELECT date AS "date!" FROM workouts WHERE id = $1 AND date IS NOT NULL"#,
        workout_id,
    )
    .fetch_one(&state.db)
    .await?;
    if let Some(id) = queue(state, Kind::WorkoutFinished, workout_id, date).await? {
        send_or_leave_for_retry(state, id).await;
    }
    Ok(())
}

/// Sends a queued message now; if that fails, the jobs try again later.
async fn send_or_leave_for_retry(state: &AppState, id: Uuid) {
    if let Err(err) = deliver(state, id, Utc::now()).await {
        tracing::warn!(error = ?err, %id, "could not send a bot message; the jobs will retry");
    }
}

/// "Нове тренування від Даші", while the workout is visible and the client linked.
async fn published(state: &AppState, workout_id: Uuid) -> anyhow::Result<Option<OutgoingMessage>> {
    let Some(workout) = sqlx::query!(
        r#"SELECT w.title, w.date AS "date!", c.telegram_id AS "telegram_id!"
           FROM workouts w JOIN clients c ON c.id = w.client_id
           WHERE w.id = $1 AND w.date IS NOT NULL AND c.telegram_id IS NOT NULL
             AND c.bot_allowed_at IS NOT NULL AND w.status IN ('published', 'done')"#,
        workout_id,
    )
    .fetch_optional(&state.db)
    .await?
    else {
        return Ok(None);
    };
    let when = short_date(workout.date);
    let text = if workout.title.is_empty() {
        format!("Нове тренування від Даші на {when}.")
    } else {
        format!("Нове тренування від Даші: «{}», {when}.", workout.title)
    };
    Ok(Some(open_workout(
        state,
        workout.telegram_id,
        text,
        workout_id,
    )))
}

/// The workout-day reminder, unless the workout is finished or gone by now.
async fn reminder(state: &AppState, workout_id: Uuid) -> anyhow::Result<Option<OutgoingMessage>> {
    let Some(workout) = sqlx::query!(
        r#"SELECT w.title, c.telegram_id AS "telegram_id!"
           FROM workouts w JOIN clients c ON c.id = w.client_id
           WHERE w.id = $1 AND w.status = 'published' AND c.telegram_id IS NOT NULL
             AND c.bot_allowed_at IS NOT NULL AND c.archived_at IS NULL"#,
        workout_id,
    )
    .fetch_optional(&state.db)
    .await?
    else {
        return Ok(None);
    };
    let text = if workout.title.is_empty() {
        "Нагадування: сьогодні тренування від Даші.".to_owned()
    } else {
        format!("Нагадування: сьогодні тренування «{}».", workout.title)
    };
    Ok(Some(open_workout(
        state,
        workout.telegram_id,
        text,
        workout_id,
    )))
}

/// Tells Dasha a client finished a workout: how much was done, how it felt,
/// and the comment.
async fn finished(state: &AppState, workout_id: Uuid) -> anyhow::Result<Option<OutgoingMessage>> {
    let Some(workout) = sqlx::query!(
        r#"SELECT w.title, w.date AS "date!", c.id AS client_id, c.name AS client_name,
                  co.telegram_id AS coach_telegram_id,
                  r.effort AS "effort: Effort", r.comment,
                  (SELECT count(*) FROM workout_sets s
                   JOIN workout_exercises we ON we.id = s.workout_exercise_id
                   WHERE we.workout_id = w.id) AS "sets!",
                  (SELECT count(*) FROM workout_sets s
                   JOIN workout_exercises we ON we.id = s.workout_exercise_id
                   WHERE we.workout_id = w.id AND s.completed_at IS NOT NULL) AS "done!",
                  (SELECT count(*) FROM workout_sets s
                   JOIN workout_exercises we ON we.id = s.workout_exercise_id
                   WHERE we.workout_id = w.id AND set_differs(s)) AS "different!"
           FROM workouts w
           JOIN clients c ON c.id = w.client_id
           JOIN coaches co ON co.id = w.coach_id
           JOIN workout_reports r ON r.workout_id = w.id
           WHERE w.id = $1 AND w.date IS NOT NULL"#,
        workout_id,
    )
    .fetch_optional(&state.db)
    .await?
    else {
        return Ok(None);
    };

    let title = if workout.title.is_empty() {
        "тренування".to_owned()
    } else {
        format!("«{}»", workout.title)
    };
    let mut summary = format!("{} з {} підходів", workout.done, workout.sets);
    if workout.different > 0 {
        summary.push_str(&format!(" · {} інакше, ніж у плані", workout.different));
    }
    summary.push_str(match workout.effort {
        Effort::Easy => " · легко",
        Effort::Ok => " · нормально",
        Effort::Hard => " · важко",
    });
    let mut text = format!(
        "{}: звіт про {title}, {}.\n{summary}",
        workout.client_name,
        short_date(workout.date)
    );
    if !workout.comment.is_empty() {
        text.push_str(&format!("\n\n«{}»", workout.comment));
    }
    Ok(Some(
        OutgoingMessage::text(workout.coach_telegram_id, text).with_row(vec![Button::WebApp {
            text: "Відкрити клієнта".to_owned(),
            url: format!(
                "{}/clients/{}",
                state.config.mini_app_url(),
                workout.client_id
            ),
        }]),
    ))
}

/// Dasha's evening summary for `date`: today's workouts nobody opened, and
/// payments ending within three days. Nothing to say means no message.
async fn daily_summary(
    state: &AppState,
    coach_id: Uuid,
    date: NaiveDate,
) -> anyhow::Result<Option<OutgoingMessage>> {
    let Some(coach_chat) =
        sqlx::query_scalar!("SELECT telegram_id FROM coaches WHERE id = $1", coach_id,)
            .fetch_optional(&state.db)
            .await?
    else {
        return Ok(None);
    };
    let unopened = sqlx::query!(
        "SELECT c.name, w.title
         FROM workouts w JOIN clients c ON c.id = w.client_id
         WHERE w.coach_id = $1 AND w.date = $2 AND w.status = 'published'
           AND w.opened_at IS NULL AND c.archived_at IS NULL
         ORDER BY c.name",
        coach_id,
        date,
    )
    .fetch_all(&state.db)
    .await?;
    let renewals = sqlx::query!(
        r#"SELECT name, paid_until AS "paid_until!" FROM clients
           WHERE coach_id = $1 AND archived_at IS NULL
             AND paid_until BETWEEN $2 AND $2 + 3
           ORDER BY paid_until, name"#,
        coach_id,
        date,
    )
    .fetch_all(&state.db)
    .await?;
    if unopened.is_empty() && renewals.is_empty() {
        return Ok(None);
    }

    let mut text = format!("Підсумок дня, {}.", short_date(date));
    if !unopened.is_empty() {
        text.push_str("\n\nНе відкрили сьогоднішнє тренування:");
        for row in &unopened {
            if row.title.is_empty() {
                text.push_str(&format!("\n• {}", row.name));
            } else {
                text.push_str(&format!("\n• {} — «{}»", row.name, row.title));
            }
        }
    }
    if !renewals.is_empty() {
        text.push_str("\n\nЗакінчується оплата:");
        for row in &renewals {
            text.push_str(&format!(
                "\n• {} — до {}",
                row.name,
                day_month(row.paid_until)
            ));
        }
    }
    Ok(Some(OutgoingMessage::text(coach_chat, text).with_row(
        vec![Button::WebApp {
            text: "Відкрити кабінет".to_owned(),
            url: state.config.mini_app_url(),
        }],
    )))
}

/// "Максим К.: нове відео техніки — «Присідання».", with a button to the report,
/// unless Dasha has already watched it.
async fn form_video(state: &AppState, id: Uuid) -> anyhow::Result<Option<OutgoingMessage>> {
    let Some(video) = sqlx::query!(
        "SELECT c.name AS client_name, e.name AS exercise_name, w.id AS workout_id,
                co.telegram_id AS coach_telegram_id
         FROM form_videos v
         JOIN workout_exercises we ON we.id = v.workout_exercise_id
         JOIN workouts w ON w.id = we.workout_id
         JOIN exercises e ON e.id = we.exercise_id
         JOIN clients c ON c.id = v.client_id
         JOIN coaches co ON co.id = c.coach_id
         WHERE v.id = $1 AND v.status = 'ready' AND v.seen_at IS NULL",
        id,
    )
    .fetch_optional(&state.db)
    .await?
    else {
        return Ok(None);
    };
    let text = format!(
        "{}: нове відео техніки — «{}».",
        video.client_name, video.exercise_name
    );
    Ok(Some(
        OutgoingMessage::text(video.coach_telegram_id, text).with_row(vec![Button::WebApp {
            text: "Переглянути".to_owned(),
            url: format!(
                "{}/workouts/{}/report",
                state.config.mini_app_url(),
                video.workout_id
            ),
        }]),
    ))
}

/// A message with a button that opens the Mini App on the workout.
fn open_workout(state: &AppState, chat_id: i64, text: String, workout_id: Uuid) -> OutgoingMessage {
    OutgoingMessage::text(chat_id, text).with_row(vec![Button::WebApp {
        text: "Відкрити тренування".to_owned(),
        url: format!("{}/workouts/{workout_id}", state.config.mini_app_url()),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_short_ukrainian_dates() {
        let date = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).unwrap();
        assert_eq!(short_date(date(2026, 10, 6)), "вт, 6 жовтня");
        assert_eq!(short_date(date(2026, 11, 1)), "нд, 1 листопада");
        assert_eq!(short_date(date(2027, 1, 18)), "пн, 18 січня");
    }

    #[test]
    fn kinds_round_trip() {
        for kind in [
            Kind::WorkoutPublished,
            Kind::WorkoutReminder,
            Kind::WorkoutFinished,
            Kind::CoachDaily,
            Kind::FormVideo,
        ] {
            assert_eq!(Kind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(Kind::parse("unknown"), None);
    }
}
