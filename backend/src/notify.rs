//! Messages the bot sends on its own, not in reply to someone. Each one first
//! claims a row in `notifications`, so repeating a step never sends twice.

use chrono::{Datelike, NaiveDate};
use uuid::Uuid;

use crate::{
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

/// `вт, 6 жовтня`.
pub fn short_date(date: NaiveDate) -> String {
    format!(
        "{}, {} {}",
        WEEKDAYS[date.weekday().num_days_from_monday() as usize],
        date.day(),
        MONTHS[date.month0() as usize]
    )
}

/// Tells the client a workout was published, once per workout and date. Returns
/// whether the client has been told (now or before); `false` if they have not
/// joined the app yet.
pub async fn workout_published(state: &AppState, workout_id: Uuid) -> anyhow::Result<bool> {
    let workout = sqlx::query!(
        r#"SELECT w.title, w.date AS "date!", c.telegram_id
           FROM workouts w JOIN clients c ON c.id = w.client_id
           WHERE w.id = $1 AND w.date IS NOT NULL"#,
        workout_id,
    )
    .fetch_one(&state.db)
    .await?;
    let Some(chat_id) = workout.telegram_id else {
        return Ok(false);
    };

    let claimed = sqlx::query_scalar!(
        "INSERT INTO notifications (kind, entity_id, local_date)
         VALUES ('workout_published', $1, $2)
         ON CONFLICT (kind, entity_id, local_date) DO NOTHING
         RETURNING id",
        workout_id,
        workout.date,
    )
    .fetch_optional(&state.db)
    .await?;
    let Some(notification_id) = claimed else {
        return Ok(true);
    };

    let when = short_date(workout.date);
    let text = if workout.title.is_empty() {
        format!("Нове тренування від Даші на {when}.")
    } else {
        format!("Нове тренування від Даші: «{}», {when}.", workout.title)
    };
    let message = OutgoingMessage::text(chat_id, text).with_row(vec![Button::WebApp {
        text: "Відкрити тренування".to_owned(),
        url: state.config.mini_app_url(),
    }]);
    if let Err(err) = state.telegram.send_message(message).await {
        // Free the claim so publishing again can retry.
        sqlx::query!("DELETE FROM notifications WHERE id = $1", notification_id)
            .execute(&state.db)
            .await?;
        return Err(err);
    }
    sqlx::query!(
        "UPDATE notifications SET sent_at = now() WHERE id = $1",
        notification_id,
    )
    .execute(&state.db)
    .await?;
    Ok(true)
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
}
