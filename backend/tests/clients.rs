//! Dasha editing her clients: names, paid-until dates, the archive, and her own
//! timezone.

mod common;

use axum::http::StatusCode;
use coaching_backend::auth::{Role, jwt::MINI_APP_TOKEN_TTL};
use common::{
    BOT_TOKEN, call, coach_token, last_message_to, seed_client, seed_coach, signed_init_data,
    test_state, text_update, webhook,
};
use serde_json::{Value, json};
use sqlx::PgPool;

const COACH_TG: i64 = 555_000_222;
const CLIENT_TG: i64 = 777_000_111;

async fn sign_in(app: &axum::Router, telegram_id: i64) -> (StatusCode, Value) {
    call(
        app,
        "POST",
        "/auth/telegram-webapp",
        None,
        Some(json!({ "init_data": signed_init_data(telegram_id, BOT_TOKEN) })),
    )
    .await
}

fn names(clients: &Value) -> Vec<&str> {
    clients
        .as_array()
        .unwrap()
        .iter()
        .map(|client| client["name"].as_str().unwrap())
        .collect()
}

#[sqlx::test]
async fn dasha_renames_a_client_and_records_payment(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let other_coach = seed_coach(&db, 555_000_999).await;
    let state = test_state(db);
    let token = coach_token(&state, coach_id);
    let app = coaching_backend::router(state.clone());
    let uri = format!("/coach/clients/{client_id}");

    let (status, client) = call(
        &app,
        "PATCH",
        &uri,
        Some(&token),
        Some(json!({ "name": "  Максим Коваль ", "paid_until": "2026-11-01" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(client["name"], "Максим Коваль");
    assert_eq!(client["paid_until"], "2026-11-01");
    assert_eq!(client["archived"], false);

    // Left out means unchanged; null clears.
    let (_, client) = call(&app, "PATCH", &uri, Some(&token), Some(json!({}))).await;
    assert_eq!(client["paid_until"], "2026-11-01");
    let (_, client) = call(
        &app,
        "PATCH",
        &uri,
        Some(&token),
        Some(json!({ "paid_until": null })),
    )
    .await;
    assert!(client["paid_until"].is_null());
    assert_eq!(client["name"], "Максим Коваль");

    let (status, body) = call(
        &app,
        "PATCH",
        &uri,
        Some(&token),
        Some(json!({ "name": "   " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_name");

    // Another coach cannot touch her clients.
    let theirs = coach_token(&state, other_coach);
    let (status, _) = call(
        &app,
        "PATCH",
        &uri,
        Some(&theirs),
        Some(json!({ "name": "Хтось" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn archiving_hides_the_client_and_closes_their_app(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let state = test_state(db);
    let token = coach_token(&state, coach_id);
    let app = coaching_backend::router(state.clone());
    let uri = format!("/coach/clients/{client_id}");

    let (_, session) = sign_in(&app, CLIENT_TG).await;
    let client_token = session["token"].as_str().unwrap().to_owned();
    let (status, _) = call(&app, "GET", "/me/workouts", Some(&client_token), None).await;
    assert_eq!(status, StatusCode::OK);

    let (status, client) = call(
        &app,
        "PATCH",
        &uri,
        Some(&token),
        Some(json!({ "archived": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(client["archived"], true);

    let (_, active) = call(&app, "GET", "/coach/clients", Some(&token), None).await;
    assert!(names(&active).is_empty());
    let (_, archive) = call(
        &app,
        "GET",
        "/coach/clients?archived=true",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(names(&archive), ["Максим К."]);
    // Dasha can still open the profile and its history.
    let (status, _) = call(&app, "GET", &uri, Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);

    // The open app stops working at once, and signing in again is refused.
    let (status, _) = call(&app, "GET", "/me/workouts", Some(&client_token), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = sign_in(&app, CLIENT_TG).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "not_invited");
    let (status, _) = call(&app, "POST", &format!("{uri}/invite"), Some(&token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "no invites while archived");

    // Restored: back in the list, and the same Telegram account signs in.
    let (_, client) = call(
        &app,
        "PATCH",
        &uri,
        Some(&token),
        Some(json!({ "archived": false })),
    )
    .await;
    assert_eq!(client["archived"], false);
    assert_eq!(client["joined"], true);
    let (_, active) = call(&app, "GET", "/coach/clients", Some(&token), None).await;
    assert_eq!(names(&active), ["Максим К."]);
    let (status, _) = sign_in(&app, CLIENT_TG).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn a_returning_client_can_join_a_new_profile(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let old_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let state = test_state(db);
    let token = coach_token(&state, coach_id);
    let app = coaching_backend::router(state.clone());

    call(
        &app,
        "PATCH",
        &format!("/coach/clients/{old_id}"),
        Some(&token),
        Some(json!({ "archived": true })),
    )
    .await;
    // Months later Dasha adds them again instead of restoring the old profile.
    let (_, created) = call(
        &app,
        "POST",
        "/coach/clients",
        Some(&token),
        Some(json!({ "name": "Максим К. (знову)" })),
    )
    .await;
    let url = created["invite"]["url"].as_str().unwrap();
    let start = format!("/start {}", url.split_once("?start=").unwrap().1);

    webhook(&app, text_update(CLIENT_TG, &start)).await;
    assert!(last_message_to(&state, CLIENT_TG).text.starts_with("Вітаю"));

    let (_, session) = sign_in(&app, CLIENT_TG).await;
    assert_eq!(session["client"]["id"], created["client"]["id"]);
    let (_, old) = call(
        &app,
        "GET",
        &format!("/coach/clients/{old_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(
        old["joined"], false,
        "the archived profile let the account go"
    );
}

#[sqlx::test]
async fn dashas_phone_sets_her_timezone(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let state = test_state(db);
    let app = coaching_backend::router(state.clone());

    let (_, session) = sign_in(&app, COACH_TG).await;
    assert_eq!(session["coach"]["timezone"], "Europe/Kyiv");
    let token = session["token"].as_str().unwrap().to_owned();

    let (status, _) = call(
        &app,
        "PUT",
        "/coach/me/timezone",
        Some(&token),
        Some(json!({ "timezone": "Europe/Warsaw" })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, session) = sign_in(&app, COACH_TG).await;
    assert_eq!(session["coach"]["timezone"], "Europe/Warsaw");

    let (status, body) = call(
        &app,
        "PUT",
        "/coach/me/timezone",
        Some(&token),
        Some(json!({ "timezone": "Mars/Olympus" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "unknown_timezone");

    // Only the coach sets the coach's timezone.
    let client_token = state
        .jwt
        .issue(Role::Client, client_id, MINI_APP_TOKEN_TTL)
        .unwrap();
    let (status, _) = call(
        &app,
        "PUT",
        "/coach/me/timezone",
        Some(&client_token),
        Some(json!({ "timezone": "Europe/Kyiv" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
