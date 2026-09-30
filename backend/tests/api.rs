//! Route-level tests against a real Postgres. `#[sqlx::test]` gives every test
//! its own fresh, migrated database (needs `DATABASE_URL`).

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::Utc;
use coaching_backend::{
    auth::{Role, jwt::COACH_TOKEN_TTL},
    config::Config,
    state::AppState,
};
use hmac::{Hmac, KeyInit, Mac};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::Sha256;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const BOT_TOKEN: &str = "123456:TEST-bot-token";

fn test_state(db: PgPool) -> AppState {
    AppState::new(
        db,
        Config {
            database_url: String::new(),
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            bot_token: BOT_TOKEN.to_owned(),
            jwt_secret: "test-secret-that-is-long-enough-for-hs256".to_owned(),
            frontend_origin: header::HeaderValue::from_static("http://localhost:5173"),
        },
    )
}

/// Builds Mini App `initData` signed the way Telegram signs it.
fn signed_init_data(telegram_id: i64, bot_token: &str) -> String {
    let user = json!({ "id": telegram_id, "first_name": "Максим" }).to_string();
    let auth_date = Utc::now().timestamp().to_string();
    let check = format!("auth_date={auth_date}\nuser={user}");

    let mut secret = Hmac::<Sha256>::new_from_slice(b"WebAppData").unwrap();
    secret.update(bot_token.as_bytes());
    let secret = secret.finalize().into_bytes();
    let mut mac = Hmac::<Sha256>::new_from_slice(&secret).unwrap();
    mac.update(check.as_bytes());
    let hash = hex::encode(mac.finalize().into_bytes());

    form_urlencoded::Serializer::new(String::new())
        .append_pair("auth_date", &auth_date)
        .append_pair("user", &user)
        .append_pair("hash", &hash)
        .finish()
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = match body {
        Some(body) => request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    }
    .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn seed_coach(db: &PgPool) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO coaches (telegram_id, name) VALUES (555000222, 'Даша') RETURNING id"
    )
    .fetch_one(db)
    .await
    .unwrap()
}

async fn seed_client(db: &PgPool, coach_id: Uuid, telegram_id: i64) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO clients (coach_id, name, telegram_id) VALUES ($1, 'Максим К.', $2) RETURNING id",
        coach_id,
        telegram_id,
    )
    .fetch_one(db)
    .await
    .unwrap()
}

#[sqlx::test]
async fn invited_client_signs_in_and_reads_profile(db: PgPool) {
    let coach_id = seed_coach(&db).await;
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
    seed_coach(&db).await;
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
    assert_eq!(body["error"], "not_a_client");

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
async fn import_preview_matches_the_library_and_is_coach_only(db: PgPool) {
    let coach_id = seed_coach(&db).await;
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
    let coach_token = state
        .jwt
        .issue(Role::Coach, coach_id, COACH_TOKEN_TTL)
        .unwrap();
    let client_token = state
        .jwt
        .issue(Role::Client, Uuid::new_v4(), COACH_TOKEN_TTL)
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
