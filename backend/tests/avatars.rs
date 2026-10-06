//! Clients' avatars: their own photo, or else a copy of their Telegram photo.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{Duration, Utc};
use coaching_backend::{
    auth::{Role, jwt::BROWSER_TOKEN_TTL},
    avatars,
    state::AppState,
    storage::{StorageClient, StorageSettings},
    telegram::Sent,
};
use common::{call, coach_token, seed_client, seed_coach, test_state};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const COACH_TG: i64 = 555_000_222;
const CLIENT_TG: i64 = 777_000_111;

fn with_storage(db: PgPool) -> AppState {
    test_state(db).with_storage(Some(StorageClient::fake(StorageSettings {
        zone: "photos-test".to_owned(),
        hostname: "storage.bunnycdn.com".to_owned(),
        password: "storage-password".to_owned(),
        cdn_hostname: "photos-test.b-cdn.net".to_owned(),
        token_key: "photos-token-key".to_owned(),
    })))
}

fn jpeg(tag: u8) -> Vec<u8> {
    vec![0xFF, 0xD8, 0xFF, 0xE0, tag, tag]
}

async fn put_avatar(app: &axum::Router, token: &str, body: Vec<u8>) -> (StatusCode, Value) {
    put_photo(app, "/me/avatar", token, body).await
}

async fn put_photo(
    app: &axum::Router,
    uri: &str,
    token: &str,
    body: Vec<u8>,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("PUT")
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "image/jpeg")
        .body(Body::from(body))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn files(state: &AppState) -> Vec<String> {
    state.storage.as_ref().unwrap().fake_files()
}

fn downloads(state: &AppState) -> usize {
    state
        .telegram
        .sent()
        .iter()
        .filter(|sent| matches!(sent, Sent::Downloaded { .. }))
        .count()
}

async fn setup(db: &PgPool) -> (Uuid, Uuid) {
    let coach_id = seed_coach(db, COACH_TG).await;
    let client_id = seed_client(db, coach_id, CLIENT_TG).await;
    (coach_id, client_id)
}

#[sqlx::test]
async fn a_client_adds_changes_and_removes_their_photo(db: PgPool) {
    let (coach_id, client_id) = setup(&db).await;
    let state = with_storage(db);
    let app = coaching_backend::router(state.clone());
    let token = state
        .jwt
        .issue(Role::Client, client_id, BROWSER_TOKEN_TTL)
        .unwrap();

    let (_, me) = call(&app, "GET", "/me", Some(&token), None).await;
    assert!(me["avatar"].is_null(), "initials until there is a photo");

    let (status, avatar) = put_avatar(&app, &token, jpeg(1)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(avatar["from_telegram"], false);
    let url = avatar["url"].as_str().unwrap();
    assert!(
        url.starts_with("https://photos-test.b-cdn.net/bcdn_token=HS256-"),
        "only through a signed link"
    );
    let first = files(&state);
    assert_eq!(first.len(), 1);
    assert!(first[0].starts_with(&format!("clients/{client_id}/avatar/")));

    // Dasha sees it in her list and on the client's page.
    let coach = coach_token(&state, coach_id);
    let (_, clients) = call(&app, "GET", "/coach/clients", Some(&coach), None).await;
    assert!(
        clients[0]["avatar_url"]
            .as_str()
            .unwrap()
            .contains(&first[0])
    );
    let (_, client) = call(
        &app,
        "GET",
        &format!("/coach/clients/{client_id}"),
        Some(&coach),
        None,
    )
    .await;
    assert!(client["avatar_url"].as_str().unwrap().contains(&first[0]));

    // A new photo replaces the old file.
    put_avatar(&app, &token, jpeg(2)).await;
    let second = files(&state);
    assert_eq!(second.len(), 1);
    assert_ne!(second, first);

    // Not a JPEG: refused, and nothing changes.
    let (status, body) = put_avatar(&app, &token, b"<svg/>".to_vec()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_photo");
    assert_eq!(files(&state), second);

    let (status, _) = call(&app, "DELETE", "/me/avatar", Some(&token), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(files(&state).is_empty());
    let (_, me) = call(&app, "GET", "/me", Some(&token), None).await;
    assert!(me["avatar"].is_null());

    // Dasha cannot set it; that is the client's.
    let (status, _) = put_avatar(&app, &coach, jpeg(3)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn without_photo_storage_avatars_are_off(db: PgPool) {
    let (_, client_id) = setup(&db).await;
    let state = test_state(db);
    let app = coaching_backend::router(state.clone());
    let token = state
        .jwt
        .issue(Role::Client, client_id, BROWSER_TOKEN_TTL)
        .unwrap();
    let (status, body) = put_avatar(&app, &token, jpeg(1)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], "photos_not_configured");
}

#[sqlx::test]
async fn the_telegram_photo_fills_in_until_they_add_their_own(db: PgPool) {
    let (_, client_id) = setup(&db).await;
    let state = with_storage(db.clone());
    let app = coaching_backend::router(state.clone());
    let token = state
        .jwt
        .issue(Role::Client, client_id, BROWSER_TOKEN_TTL)
        .unwrap();
    let a_week_later = |days| Utc::now() + Duration::days(days);

    // Telegram hides their photo from bots: still initials, checked again in a week.
    avatars::refresh_from_telegram(&state, Utc::now())
        .await
        .unwrap();
    let (_, me) = call(&app, "GET", "/me", Some(&token), None).await;
    assert!(me["avatar"].is_null());

    // They show it later: the next weekly look copies it.
    state
        .telegram
        .set_profile_photo(CLIENT_TG, Some(("photo-a", jpeg(10))));
    avatars::refresh_from_telegram(&state, Utc::now())
        .await
        .unwrap();
    assert_eq!(downloads(&state), 0, "not due again yet");
    avatars::refresh_from_telegram(&state, a_week_later(8))
        .await
        .unwrap();
    assert_eq!(downloads(&state), 1);
    let (_, me) = call(&app, "GET", "/me", Some(&token), None).await;
    assert_eq!(me["avatar"]["from_telegram"], true);
    let copied = files(&state);
    assert_eq!(copied.len(), 1);

    // The same photo a week later: not fetched again.
    avatars::refresh_from_telegram(&state, a_week_later(16))
        .await
        .unwrap();
    assert_eq!(downloads(&state), 1);
    assert_eq!(files(&state), copied);

    // A new Telegram photo replaces the copy.
    state
        .telegram
        .set_profile_photo(CLIENT_TG, Some(("photo-b", jpeg(11))));
    avatars::refresh_from_telegram(&state, a_week_later(24))
        .await
        .unwrap();
    assert_eq!(downloads(&state), 2);
    assert_eq!(files(&state).len(), 1);
    assert_ne!(files(&state), copied);

    // Their own photo wins, and Telegram never replaces it.
    put_avatar(&app, &token, jpeg(1)).await;
    let own = files(&state);
    state
        .telegram
        .set_profile_photo(CLIENT_TG, Some(("photo-c", jpeg(12))));
    avatars::refresh_from_telegram(&state, a_week_later(40))
        .await
        .unwrap();
    assert_eq!(downloads(&state), 2);
    assert_eq!(files(&state), own);
    let (_, me) = call(&app, "GET", "/me", Some(&token), None).await;
    assert_eq!(me["avatar"]["from_telegram"], false);

    // Removing their own brings the Telegram photo back at the next round.
    call(&app, "DELETE", "/me/avatar", Some(&token), None).await;
    avatars::refresh_from_telegram(&state, a_week_later(40))
        .await
        .unwrap();
    let (_, me) = call(&app, "GET", "/me", Some(&token), None).await;
    assert_eq!(me["avatar"]["from_telegram"], true);

    // They remove or hide it in Telegram: the copy goes too.
    state.telegram.set_profile_photo(CLIENT_TG, None);
    avatars::refresh_from_telegram(&state, a_week_later(48))
        .await
        .unwrap();
    let (_, me) = call(&app, "GET", "/me", Some(&token), None).await;
    assert!(me["avatar"].is_null());
    assert!(files(&state).is_empty());
}

#[sqlx::test]
async fn telegram_failing_is_tried_again_a_day_later(db: PgPool) {
    let (_, client_id) = setup(&db).await;
    let state = with_storage(db.clone());
    state
        .telegram
        .set_profile_photo(CLIENT_TG, Some(("photo-a", jpeg(10))));
    state.telegram.set_failing(true);
    avatars::refresh_from_telegram(&state, Utc::now())
        .await
        .unwrap();
    state.telegram.set_failing(false);

    avatars::refresh_from_telegram(&state, Utc::now() + Duration::hours(1))
        .await
        .unwrap();
    assert_eq!(downloads(&state), 0, "not hammering Telegram");
    avatars::refresh_from_telegram(&state, Utc::now() + Duration::hours(25))
        .await
        .unwrap();
    assert_eq!(downloads(&state), 1);
    let from_telegram = sqlx::query_scalar!(
        "SELECT avatar_from_telegram FROM clients WHERE id = $1",
        client_id
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(from_telegram);
}

#[sqlx::test]
async fn the_coach_adds_her_photo_and_her_clients_see_it(db: PgPool) {
    let (coach_id, client_id) = setup(&db).await;
    let state = with_storage(db);
    let app = coaching_backend::router(state.clone());
    let coach = coach_token(&state, coach_id);
    let client = state
        .jwt
        .issue(Role::Client, client_id, BROWSER_TOKEN_TTL)
        .unwrap();

    let (_, me) = call(&app, "GET", "/me", Some(&client), None).await;
    assert_eq!(me["coach"]["name"], "Даша");
    assert!(
        me["coach"]["avatar_url"].is_null(),
        "initials until she adds one"
    );

    let (status, avatar) = put_photo(&app, "/coach/me/avatar", &coach, jpeg(1)).await;
    assert_eq!(status, StatusCode::OK);
    let stored = files(&state);
    assert_eq!(stored.len(), 1);
    assert!(stored[0].starts_with(&format!("coaches/{coach_id}/avatar/")));
    assert!(avatar["url"].as_str().unwrap().contains(&stored[0]));

    // Her own session carries it, and so does every client's.
    let (_, session) = call(&app, "POST", "/auth/refresh", Some(&coach), None).await;
    assert!(
        session["coach"]["avatar_url"]
            .as_str()
            .unwrap()
            .contains(&stored[0])
    );
    let (_, me) = call(&app, "GET", "/me", Some(&client), None).await;
    assert!(
        me["coach"]["avatar_url"]
            .as_str()
            .unwrap()
            .contains(&stored[0])
    );

    // A new photo replaces the old file.
    put_photo(&app, "/coach/me/avatar", &coach, jpeg(2)).await;
    assert_eq!(files(&state).len(), 1);
    assert_ne!(files(&state), stored);

    // Only the coach sets it.
    let (status, _) = put_photo(&app, "/coach/me/avatar", &client, jpeg(3)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = call(&app, "DELETE", "/coach/me/avatar", Some(&coach), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(files(&state).is_empty());
    let (_, me) = call(&app, "GET", "/me", Some(&client), None).await;
    assert!(me["coach"]["avatar_url"].is_null());
}
