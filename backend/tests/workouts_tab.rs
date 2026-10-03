//! The workouts tab: every client's workouts for a week, filterable by client.

mod common;

use axum::http::StatusCode;
use common::{call, coach_token, seed_client, seed_coach, test_state};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

async fn workout(db: &PgPool, client_id: Uuid, title: &str, date: Option<&str>, status: &str) {
    sqlx::query(
        "INSERT INTO workouts (coach_id, client_id, title, date, status)
         SELECT coach_id, id, $2, $3::date, $4::workout_status FROM clients WHERE id = $1",
    )
    .bind(client_id)
    .bind(title)
    .bind(date)
    .bind(status)
    .execute(db)
    .await
    .unwrap();
}

async fn client(db: &PgPool, coach_id: Uuid, telegram_id: i64, name: &str) -> Uuid {
    let id = seed_client(db, coach_id, telegram_id).await;
    sqlx::query("UPDATE clients SET name = $2 WHERE id = $1")
        .bind(id)
        .bind(name)
        .execute(db)
        .await
        .unwrap();
    id
}

fn titles(list: &Value) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|item| {
            format!(
                "{}: {}",
                item["client_name"].as_str().unwrap(),
                item["workout"]["title"].as_str().unwrap()
            )
        })
        .collect()
}

#[sqlx::test]
async fn the_week_across_clients_and_for_one(db: PgPool) {
    let coach_id = seed_coach(&db, 555_000_222).await;
    let maksym = client(&db, coach_id, 777_000_111, "Максим К.").await;
    let olena = client(&db, coach_id, 777_000_222, "Олена К.").await;
    let gone = client(&db, coach_id, 777_000_333, "Архів А.").await;
    sqlx::query("UPDATE clients SET archived_at = now() WHERE id = $1")
        .bind(gone)
        .execute(&db)
        .await
        .unwrap();
    let other_coach = seed_coach(&db, 555_000_999).await;
    let stranger = client(&db, other_coach, 777_000_444, "Чужий Ч.").await;

    workout(&db, maksym, "Спина", Some("2026-10-06"), "published").await;
    workout(&db, olena, "Ноги", Some("2026-10-05"), "done").await;
    workout(&db, maksym, "Минулий тиждень", Some("2026-09-29"), "done").await;
    workout(&db, olena, "Без дати", None, "draft").await;
    workout(&db, gone, "Архівний", Some("2026-10-07"), "published").await;
    workout(&db, stranger, "Чужий", Some("2026-10-06"), "published").await;

    let state = test_state(db);
    let token = coach_token(&state, coach_id);
    let app = coaching_backend::router(state);

    let (status, week) = call(
        &app,
        "GET",
        "/coach/workouts?from=2026-10-05&to=2026-10-11",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{week}");
    // Undated drafts first, then by date; archived and other coaches' clients left out.
    assert_eq!(
        titles(&week),
        ["Олена К.: Без дати", "Олена К.: Ноги", "Максим К.: Спина"]
    );
    assert_eq!(week[2]["client_id"], maksym.to_string());
    assert_eq!(week[2]["workout"]["status"], "published");

    let (_, only_maksym) = call(
        &app,
        "GET",
        &format!("/coach/workouts?from=2026-10-05&to=2026-10-11&client_id={maksym}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(titles(&only_maksym), ["Максим К.: Спина"]);

    for range in [
        "from=2026-10-11&to=2026-10-05",
        "from=2026-01-01&to=2026-12-31",
    ] {
        let (status, body) = call(
            &app,
            "GET",
            &format!("/coach/workouts?{range}"),
            Some(&token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "invalid_range");
    }
}
