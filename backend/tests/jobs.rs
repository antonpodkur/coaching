//! Background jobs, run one round at a time with a chosen clock: reminders,
//! Dasha's evening summary, and retries of messages Telegram refused.

mod common;

use chrono::{DateTime, Duration, Utc};
use coaching_backend::{
    auth::{Role, jwt::MINI_APP_TOKEN_TTL},
    jobs, notify,
    state::AppState,
    telegram::Button,
};
use common::{FRONTEND_URL, call, messages_to, seed_client, seed_coach, test_state};
use sqlx::PgPool;
use uuid::Uuid;

const COACH_TG: i64 = 555_000_222;
const KYIV_TG: i64 = 777_000_111;
const NEW_YORK_TG: i64 = 888_000_222;
const UNKNOWN_TZ_TG: i64 = 999_000_333;
/// The advisory lock key in `jobs.rs`.
const JOBS_LOCK: i64 = 0x636f_6163_685f_6a6f;

fn at(utc: &str) -> DateTime<Utc> {
    utc.parse().unwrap()
}

async fn workout(
    db: &PgPool,
    client_id: Uuid,
    title: &str,
    date: &str,
    status: &str,
    published_at: DateTime<Utc>,
) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO workouts (coach_id, client_id, title, date, status, published_at)
         SELECT coach_id, id, $2, $3::date, $4::workout_status, $5 FROM clients WHERE id = $1
         RETURNING id",
    )
    .bind(client_id)
    .bind(title)
    .bind(date)
    .bind(status)
    .bind(published_at)
    .fetch_one(db)
    .await
    .unwrap()
}

async fn client_in(
    db: &PgPool,
    coach_id: Uuid,
    telegram_id: i64,
    name: &str,
    timezone: &str,
) -> Uuid {
    let id = seed_client(db, coach_id, telegram_id).await;
    sqlx::query("UPDATE clients SET name = $2, timezone = $3 WHERE id = $1")
        .bind(id)
        .bind(name)
        .bind(timezone)
        .execute(db)
        .await
        .unwrap();
    id
}

fn texts(state: &AppState, chat_id: i64) -> Vec<String> {
    messages_to(state, chat_id)
        .into_iter()
        .map(|m| m.text)
        .collect()
}

#[sqlx::test]
async fn reminders_go_out_in_each_clients_morning(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let kyiv = client_in(&db, coach_id, KYIV_TG, "Максим К.", "Europe/Kyiv").await;
    let new_york = client_in(&db, coach_id, NEW_YORK_TG, "Олена К.", "America/New_York").await;
    let earlier = at("2026-10-04T12:00:00Z");
    let back = workout(&db, kyiv, "Спина", "2026-10-06", "published", earlier).await;
    workout(&db, new_york, "Ноги", "2026-10-06", "published", earlier).await;
    // Already done, and published but not for today: no reminders.
    workout(&db, kyiv, "Груди", "2026-10-06", "done", earlier).await;
    workout(&db, kyiv, "Плечі", "2026-10-07", "published", earlier).await;
    let state = test_state(db);

    // 08:00 in Kyiv: too early.
    jobs::tick(&state, at("2026-10-06T05:00:00Z"))
        .await
        .unwrap();
    assert!(texts(&state, KYIV_TG).is_empty());

    // 09:05 in Kyiv, 02:05 in New York.
    jobs::tick(&state, at("2026-10-06T06:05:00Z"))
        .await
        .unwrap();
    let reminders = messages_to(&state, KYIV_TG);
    assert_eq!(reminders.len(), 1);
    assert_eq!(
        reminders[0].text,
        "Нагадування: сьогодні тренування «Спина»."
    );
    assert!(matches!(
        &reminders[0].keyboard[0][0],
        Button::WebApp { url, .. } if *url == format!("{FRONTEND_URL}/app/workouts/{back}")
    ));
    assert!(texts(&state, NEW_YORK_TG).is_empty());

    // Later rounds do not repeat it; New York's morning comes hours later.
    jobs::tick(&state, at("2026-10-06T06:10:00Z"))
        .await
        .unwrap();
    jobs::tick(&state, at("2026-10-06T13:05:00Z"))
        .await
        .unwrap();
    assert_eq!(texts(&state, KYIV_TG).len(), 1);
    assert_eq!(
        texts(&state, NEW_YORK_TG),
        ["Нагадування: сьогодні тренування «Ноги»."]
    );
}

#[sqlx::test]
async fn a_workout_published_this_morning_gets_no_reminder(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let kyiv = client_in(&db, coach_id, KYIV_TG, "Максим К.", "Europe/Kyiv").await;
    workout(
        &db,
        kyiv,
        "Спина",
        "2026-10-06",
        "published",
        at("2026-10-06T05:30:00Z"),
    )
    .await;
    let state = test_state(db);

    jobs::tick(&state, at("2026-10-06T06:05:00Z"))
        .await
        .unwrap();
    assert!(
        texts(&state, KYIV_TG).is_empty(),
        "the publish message just said it"
    );
}

#[sqlx::test]
async fn messages_telegram_refused_are_retried(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let kyiv = client_in(&db, coach_id, KYIV_TG, "Максим К.", "Europe/Kyiv").await;
    let id = workout(&db, kyiv, "Спина", "2026-10-06", "published", Utc::now()).await;
    let state = test_state(db.clone());

    state.telegram.set_failing(true);
    assert!(
        notify::workout_published(&state, id).await.unwrap(),
        "queued counts as told"
    );
    state.telegram.set_failing(false);
    assert!(texts(&state, KYIV_TG).is_empty());

    // Too soon for a retry.
    jobs::tick(&state, Utc::now() + Duration::minutes(5))
        .await
        .unwrap();
    assert!(texts(&state, KYIV_TG).is_empty());

    jobs::tick(&state, Utc::now() + Duration::minutes(11))
        .await
        .unwrap();
    assert_eq!(
        texts(&state, KYIV_TG),
        ["Нове тренування від Даші: «Спина», вт, 6 жовтня."]
    );
    jobs::tick(&state, Utc::now() + Duration::minutes(25))
        .await
        .unwrap();
    assert_eq!(texts(&state, KYIV_TG).len(), 1);

    let attempts: i32 = sqlx::query_scalar("SELECT attempts FROM notifications")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(attempts, 2);
}

#[sqlx::test]
async fn dasha_gets_an_evening_summary_only_when_there_is_something_to_say(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let maksym = client_in(&db, coach_id, KYIV_TG, "Максим К.", "Europe/Kyiv").await;
    let alina = client_in(&db, coach_id, NEW_YORK_TG, "Аліна Р.", "Europe/Kyiv").await;
    let earlier = at("2026-10-04T12:00:00Z");
    let opened = workout(&db, maksym, "Спина", "2026-10-06", "published", earlier).await;
    workout(&db, alina, "Ноги", "2026-10-06", "published", earlier).await;
    sqlx::query("UPDATE clients SET paid_until = '2026-10-08' WHERE id = $1")
        .bind(maksym)
        .execute(&db)
        .await
        .unwrap();
    let state = test_state(db.clone());
    let app = coaching_backend::router(state.clone());

    // Максим opens his workout in the app; Аліна does not.
    let token = state
        .jwt
        .issue(Role::Client, maksym, MINI_APP_TOKEN_TTL)
        .unwrap();
    call(
        &app,
        "GET",
        &format!("/workouts/{opened}"),
        Some(&token),
        None,
    )
    .await;

    // 19:30 in Kyiv: not yet.
    jobs::tick(&state, at("2026-10-06T16:30:00Z"))
        .await
        .unwrap();
    assert!(texts(&state, COACH_TG).is_empty());

    jobs::tick(&state, at("2026-10-06T17:05:00Z"))
        .await
        .unwrap();
    jobs::tick(&state, at("2026-10-06T17:10:00Z"))
        .await
        .unwrap();
    assert_eq!(
        texts(&state, COACH_TG),
        [
            "Підсумок дня, вт, 6 жовтня.\n\nНе відкрили сьогоднішнє тренування:\n• Аліна Р. — «Ноги»\n\nЗакінчується оплата:\n• Максим К. — до 8 жовтня"
        ]
    );

    // A quiet day: no message, and the day is closed rather than checked again.
    sqlx::query("UPDATE clients SET paid_until = NULL")
        .execute(&db)
        .await
        .unwrap();
    jobs::tick(&state, at("2026-10-07T17:05:00Z"))
        .await
        .unwrap();
    assert_eq!(texts(&state, COACH_TG).len(), 1);
    let skipped: bool = sqlx::query_scalar(
        "SELECT skipped FROM notifications WHERE kind = 'coach_daily' AND local_date = '2026-10-07'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(skipped);
}

#[sqlx::test]
async fn a_timezone_postgres_does_not_know_stops_nobody(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let kyiv = client_in(&db, coach_id, KYIV_TG, "Максим К.", "Europe/Kyiv").await;
    // Stored before the API checked names with Postgres, like Europe/Kiev was.
    let unknown = client_in(&db, coach_id, UNKNOWN_TZ_TG, "Олена К.", "Mars/Olympus").await;
    sqlx::query("UPDATE coaches SET timezone = 'Mars/Olympus' WHERE id = $1")
        .bind(coach_id)
        .execute(&db)
        .await
        .unwrap();
    let earlier = at("2026-10-04T12:00:00Z");
    workout(&db, kyiv, "Спина", "2026-10-06", "published", earlier).await;
    workout(&db, unknown, "Ноги", "2026-10-06", "published", earlier).await;
    let state = test_state(db);

    // 09:05 in Kyiv; the unknown timezone counts as Kyiv.
    jobs::tick(&state, at("2026-10-06T06:05:00Z"))
        .await
        .unwrap();
    assert_eq!(
        texts(&state, KYIV_TG),
        ["Нагадування: сьогодні тренування «Спина»."]
    );
    assert_eq!(
        texts(&state, UNKNOWN_TZ_TG),
        ["Нагадування: сьогодні тренування «Ноги»."]
    );

    // Dasha's summary at 20:05 Kyiv time.
    jobs::tick(&state, at("2026-10-06T17:05:00Z"))
        .await
        .unwrap();
    let summary = texts(&state, COACH_TG);
    assert_eq!(summary.len(), 1);
    assert!(summary[0].starts_with("Підсумок дня, вт, 6 жовтня."));
}

#[sqlx::test]
async fn only_one_instance_runs_a_round(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let kyiv = client_in(&db, coach_id, KYIV_TG, "Максим К.", "Europe/Kyiv").await;
    workout(
        &db,
        kyiv,
        "Спина",
        "2026-10-06",
        "published",
        at("2026-10-04T12:00:00Z"),
    )
    .await;
    let state = test_state(db.clone());

    // Another instance holds the lock.
    let mut other = db.acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(JOBS_LOCK)
        .execute(&mut *other)
        .await
        .unwrap();
    jobs::tick(&state, at("2026-10-06T06:05:00Z"))
        .await
        .unwrap();
    assert!(texts(&state, KYIV_TG).is_empty());

    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(JOBS_LOCK)
        .execute(&mut *other)
        .await
        .unwrap();
    jobs::tick(&state, at("2026-10-06T06:05:00Z"))
        .await
        .unwrap();
    assert_eq!(texts(&state, KYIV_TG).len(), 1);
}
