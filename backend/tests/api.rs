//! Route-level tests against a real Postgres. `#[sqlx::test]` gives every test
//! its own fresh, migrated database (needs `DATABASE_URL`).

mod common;

use axum::http::StatusCode;
use coaching_backend::auth::{Role, jwt::BROWSER_TOKEN_TTL};
use common::{BOT_TOKEN, call, coach_token, seed_client, seed_coach, signed_init_data, test_state};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn invited_client_signs_in_and_reads_profile(db: PgPool) {
    let coach_id = seed_coach(&db, 555_000_222).await;
    let client_id = seed_client(&db, coach_id, 777_000_111).await;
    let app = coaching_backend::router(test_state(db));

    let init_data = signed_init_data(777_000_111, BOT_TOKEN);
    let (status, session) = call(
        &app,
        "POST",
        "/auth/telegram-webapp",
        None,
        Some(json!({ "init_data": init_data })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(session["role"], "client");
    assert_eq!(session["client"]["id"], client_id.to_string());

    let token = session["token"].as_str().unwrap();
    let (status, me) = call(&app, "GET", "/me", Some(token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["name"], "Максим К.");

    let (status, _) = call(
        &app,
        "PUT",
        "/me/timezone",
        Some(token),
        Some(json!({ "timezone": "Europe/Warsaw" })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, me) = call(&app, "GET", "/me", Some(token), None).await;
    assert_eq!(me["timezone"], "Europe/Warsaw");

    let (status, body) = call(
        &app,
        "PUT",
        "/me/timezone",
        Some(token),
        Some(json!({ "timezone": "Mars/Olympus" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "unknown_timezone");
}

#[sqlx::test]
async fn uninvited_or_forged_logins_are_refused(db: PgPool) {
    seed_coach(&db, 555_000_222).await;
    let app = coaching_backend::router(test_state(db));

    let stranger = signed_init_data(999, BOT_TOKEN);
    let (status, body) = call(
        &app,
        "POST",
        "/auth/telegram-webapp",
        None,
        Some(json!({ "init_data": stranger })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "not_invited");

    let forged = signed_init_data(777_000_111, "654321:not-our-bot");
    let (status, _) = call(
        &app,
        "POST",
        "/auth/telegram-webapp",
        None,
        Some(json!({ "init_data": forged })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = call(&app, "GET", "/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn the_coach_gets_the_coach_area_in_the_mini_app(db: PgPool) {
    let coach_id = seed_coach(&db, 555_000_222).await;
    // She is also her own test client; the coach area still wins.
    seed_client(&db, coach_id, 555_000_222).await;
    let app = coaching_backend::router(test_state(db));

    let init_data = signed_init_data(555_000_222, BOT_TOKEN);
    let (status, session) = call(
        &app,
        "POST",
        "/auth/telegram-webapp",
        None,
        Some(json!({ "init_data": init_data })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(session["role"], "coach");
    assert_eq!(session["coach"]["id"], coach_id.to_string());
    assert!(session.get("client").is_none());

    let token = session["token"].as_str().unwrap();
    let (status, clients) = call(&app, "GET", "/coach/clients", Some(token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(clients.as_array().unwrap().len(), 1);
    let (status, body) = call(&app, "GET", "/me", Some(token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "wrong_role");
}

#[sqlx::test]
async fn import_preview_matches_the_library_and_is_coach_only(db: PgPool) {
    let coach_id = seed_coach(&db, 555_000_222).await;
    let row_id = sqlx::query_scalar!(
        "INSERT INTO exercises (coach_id, name, aliases) VALUES ($1, 'Тяга гантелі в нахилі', '{}')
         RETURNING id",
        coach_id,
    )
    .fetch_one(&db)
    .await
    .unwrap();
    let pull_up_id = sqlx::query_scalar!(
        "INSERT INTO exercises (coach_id, name, aliases) VALUES ($1, 'Підтягування на турніку', '{Підтягування}')
         RETURNING id",
        coach_id,
    )
    .fetch_one(&db)
    .await
    .unwrap();
    let state = test_state(db);
    let coach_token = coach_token(&state, coach_id);
    let client_token = state
        .jwt
        .issue(Role::Client, Uuid::new_v4(), BROWSER_TOKEN_TTL)
        .unwrap();
    let app = coaching_backend::router(state);

    let text = include_str!("fixtures/example-back.txt");
    let (status, preview) = call(
        &app,
        "POST",
        "/coach/import/parse",
        Some(&coach_token),
        Some(json!({ "text": text })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let exercises = preview["exercises"].as_array().unwrap();
    assert_eq!(exercises.len(), 7);
    assert_eq!(
        exercises[0]["exercise_id"],
        pull_up_id.to_string(),
        "matched by alias"
    );
    assert_eq!(exercises[3]["exercise_id"], row_id.to_string());
    assert_eq!(exercises[3]["per_side_label"], "на кожну руку");
    assert_eq!(
        exercises[2]["sets"][0],
        json!({ "kg": 79.0, "reps_min": 8, "reps_max": 10 })
    );
    assert!(exercises[1]["exercise_id"].is_null(), "new exercise");

    let (status, body) = call(
        &app,
        "POST",
        "/coach/import/parse",
        Some(&client_token),
        Some(json!({ "text": text })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "wrong_role");
}
