//! The client's weight log: one entry a day, checked, and visible to Dasha.

mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use coaching_backend::auth::{Role, jwt::MINI_APP_TOKEN_TTL};
use common::{call, coach_token, seed_client, seed_coach, test_state};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test]
async fn weigh_ins_are_checked_kept_one_a_day_and_shown_to_dasha(db: PgPool) {
    let coach_id = seed_coach(&db, 555_000_222).await;
    let other_coach_id = seed_coach(&db, 555_000_333).await;
    let client_id = seed_client(&db, coach_id, 777_000_111).await;
    let state = test_state(db);
    let app = coaching_backend::router(state.clone());
    let client = state
        .jwt
        .issue(Role::Client, client_id, MINI_APP_TOKEN_TTL)
        .unwrap();
    let coach = coach_token(&state, coach_id);
    let log = async |date: &str, kg: f64| {
        let path = format!("/me/weight/{date}");
        call(&app, "PUT", &path, Some(&client), Some(json!({ "kg": kg }))).await
    };

    let (status, entry) = log("2026-09-01", 84.35).await;
    assert_eq!(status, StatusCode::OK, "{entry}");
    assert_eq!(entry, json!({ "date": "2026-09-01", "kg": 84.35 }));
    log("2026-09-29", 83.0).await;
    // Weighing again the same day replaces the entry.
    log("2026-09-29", 82.4).await;

    let (_, entries) = call(&app, "GET", "/me/weight", Some(&client), None).await;
    assert_eq!(
        entries,
        json!([
            { "date": "2026-09-01", "kg": 84.35 },
            { "date": "2026-09-29", "kg": 82.4 },
        ])
    );

    let (status, body) = log("2026-09-30", 5.0).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_weight"))
    );
    let next_week = (Utc::now() + Duration::days(7)).date_naive().to_string();
    let (status, body) = log(&next_week, 80.0).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_date"))
    );

    // Dasha sees the same; another coach and a client cannot use her route.
    let path = format!("/coach/clients/{client_id}/weight");
    let (status, seen) = call(&app, "GET", &path, Some(&coach), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(seen, entries);
    let other = coach_token(&state, other_coach_id);
    let (status, _) = call(&app, "GET", &path, Some(&other), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(&app, "GET", &path, Some(&client), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = call(&app, "DELETE", "/me/weight/2026-09-01", Some(&client), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, "DELETE", "/me/weight/2026-09-01", Some(&client), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, entries) = call(&app, "GET", "/me/weight", Some(&client), None).await;
    assert_eq!(entries.as_array().unwrap().len(), 1);
}
