//! The exercise library and its videos, with a fake Bunny Stream library.

mod common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use coaching_backend::{
    auth::{Role, jwt::MINI_APP_TOKEN_TTL},
    state::AppState,
    video::{StreamClient, StreamSettings},
};
use common::{call, coach_token, seed_coach, test_state};
use hmac::{Hmac, KeyInit, Mac};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const LIBRARY_ID: &str = "12345";
const API_KEY: &str = "stream-api-key";
const READ_ONLY_KEY: &str = "read-only-key";

// Bunny's API statuses (not the webhook's).
const TRANSCODING: i32 = 3;
const FINISHED: i32 = 4;
const ERROR: i32 = 5;

fn fake_stream() -> StreamClient {
    StreamClient::fake(StreamSettings {
        library_id: LIBRARY_ID.to_owned(),
        api_key: API_KEY.to_owned(),
        read_only_api_key: Some(READ_ONLY_KEY.to_owned()),
        cdn_hostname: "vz-test.b-cdn.net".to_owned(),
    })
}

/// A coach, her token, and the app with a fake Bunny library.
async fn setup(db: PgPool) -> (Router, AppState, String) {
    let coach_id = seed_coach(&db, 555_000_222).await;
    let state = test_state(db).with_video(Some(fake_stream()));
    let token = coach_token(&state, coach_id);
    (coaching_backend::router(state.clone()), state, token)
}

async fn create(app: &Router, token: &str, name: &str) -> Value {
    let (status, exercise) = call(
        app,
        "POST",
        "/coach/exercises",
        Some(token),
        Some(json!({ "name": name, "muscle_group": "Ноги" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{exercise}");
    exercise
}

/// Posts a Bunny webhook, signed like Bunny signs it unless `signed` is false.
async fn stream_webhook(app: &Router, body: Value, signed: bool) -> StatusCode {
    let body = body.to_string();
    let mut request = Request::builder()
        .method("POST")
        .uri("/webhooks/stream")
        .header(header::CONTENT_TYPE, "application/json");
    if signed {
        let mut mac = Hmac::<Sha256>::new_from_slice(READ_ONLY_KEY.as_bytes()).unwrap();
        mac.update(body.as_bytes());
        request = request
            .header("X-BunnyStream-Signature-Version", "v1")
            .header("X-BunnyStream-Signature-Algorithm", "hmac-sha256")
            .header(
                "X-BunnyStream-Signature",
                hex::encode(mac.finalize().into_bytes()),
            );
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    response.into_body().collect().await.unwrap();
    status
}

fn finished(video_id: &str) -> Value {
    // Webhook status 3 means "finished"; the backend asks the API either way.
    json!({ "VideoLibraryId": 12345, "VideoGuid": video_id, "Status": 3 })
}

#[sqlx::test]
async fn the_library_lists_adds_renames_and_archives(db: PgPool) {
    let other_coach = seed_coach(&db, 111).await;
    let (app, state, token) = setup(db).await;

    let rdl = create(&app, &token, "  Румунська   тяга ").await;
    assert_eq!(rdl["name"], "Румунська тяга");
    assert!(rdl["video"].is_null() && rdl["upload"].is_null());
    create(&app, &token, "Випади").await;

    let (status, body) = call(
        &app,
        "POST",
        "/coach/exercises",
        Some(&token),
        Some(json!({ "name": "румунська тяга" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "name_taken");
    let (status, body) = call(
        &app,
        "POST",
        "/coach/exercises",
        Some(&token),
        Some(json!({ "name": "   " })),
    )
    .await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_name"))
    );

    let (_, list) = call(&app, "GET", "/coach/exercises", Some(&token), None).await;
    let names: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].clone())
        .collect();
    assert_eq!(names, [json!("Випади"), json!("Румунська тяга")]);

    let path = format!("/coach/exercises/{}", rdl["id"].as_str().unwrap());
    let (status, renamed) = call(
        &app,
        "PATCH",
        &path,
        Some(&token),
        Some(json!({ "name": "Румунська тяга з гантелями", "muscle_group": "" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(renamed["name"], "Румунська тяга з гантелями");
    assert!(renamed["muscle_group"].is_null());

    // Another coach and clients cannot see it.
    let theirs = coach_token(&state, other_coach);
    let (status, _) = call(&app, "GET", &path, Some(&theirs), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &app,
        "PATCH",
        &path,
        Some(&theirs),
        Some(json!({ "archived": true })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let client = state
        .jwt
        .issue(Role::Client, Uuid::new_v4(), MINI_APP_TOKEN_TTL)
        .unwrap();
    let (status, _) = call(&app, "GET", "/coach/exercises", Some(&client), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = call(
        &app,
        "PATCH",
        &path,
        Some(&token),
        Some(json!({ "archived": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, list) = call(&app, "GET", "/coach/exercises", Some(&token), None).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    // An archived name is free again.
    create(&app, &token, "Румунська тяга з гантелями").await;
}

#[sqlx::test]
async fn a_video_goes_from_upload_to_ready_and_replaces_the_old_one(db: PgPool) {
    let (app, state, token) = setup(db).await;
    let stream = state.video.clone().unwrap();
    let exercise = create(&app, &token, "Тяга гантелі в нахилі").await;
    let path = format!("/coach/exercises/{}", exercise["id"].as_str().unwrap());

    // The phone gets a signed ticket for one new Bunny video.
    let (status, ticket) = call(
        &app,
        "POST",
        &format!("{path}/video-upload"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ticket["endpoint"], "https://video.bunnycdn.com/tusupload");
    assert_eq!(ticket["library_id"], LIBRARY_ID);
    let first = ticket["video_id"].as_str().unwrap().to_owned();
    let expires = ticket["expires_at"].as_i64().unwrap();
    let expected = hex::encode(Sha256::digest(format!(
        "{LIBRARY_ID}{API_KEY}{expires}{first}"
    )));
    assert_eq!(ticket["signature"], expected);
    let (_, uploading) = call(&app, "GET", &path, Some(&token), None).await;
    assert_eq!(uploading["upload"]["status"], "uploading");

    // Upload done; Bunny encodes.
    let (status, processing) = call(
        &app,
        "POST",
        &format!("{path}/video-uploaded"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(processing["upload"]["status"], "processing");
    stream.fake_set_status(&first, TRANSCODING, 0);
    let (_, still) = call(&app, "GET", &path, Some(&token), None).await;
    assert_eq!(still["upload"]["status"], "processing");

    // Looking at the exercise asks Bunny, even without a webhook.
    stream.fake_set_status(&first, FINISHED, 48);
    let (_, ready) = call(&app, "GET", &path, Some(&token), None).await;
    assert!(ready["upload"].is_null());
    assert_eq!(ready["video"]["length_secs"], 48);
    assert_eq!(
        ready["video"]["hls_url"],
        format!("https://vz-test.b-cdn.net/{first}/playlist.m3u8")
    );

    // A replacement keeps the old video playable until it is ready.
    let (_, ticket) = call(
        &app,
        "POST",
        &format!("{path}/video-upload"),
        Some(&token),
        None,
    )
    .await;
    let second = ticket["video_id"].as_str().unwrap().to_owned();
    let (_, meanwhile) = call(&app, "GET", &path, Some(&token), None).await;
    assert_eq!(meanwhile["upload"]["status"], "uploading");
    assert_eq!(meanwhile["video"]["length_secs"], 48);

    stream.fake_set_status(&second, FINISHED, 30);
    assert_eq!(
        stream_webhook(&app, finished(&second), true).await,
        StatusCode::OK
    );
    let (_, replaced) = call(&app, "GET", &path, Some(&token), None).await;
    assert_eq!(replaced["video"]["length_secs"], 30);
    assert!(replaced["upload"].is_null());
    assert_eq!(
        stream.fake_deleted(),
        [first],
        "the old video is deleted on Bunny"
    );
}

#[sqlx::test]
async fn abandoned_and_failed_uploads(db: PgPool) {
    let (app, state, token) = setup(db).await;
    let stream = state.video.clone().unwrap();
    let exercise = create(&app, &token, "Присідання").await;
    let path = format!("/coach/exercises/{}", exercise["id"].as_str().unwrap());

    // Starting again drops the unfinished upload, on Bunny too.
    let (_, ticket) = call(
        &app,
        "POST",
        &format!("{path}/video-upload"),
        Some(&token),
        None,
    )
    .await;
    let abandoned = ticket["video_id"].as_str().unwrap().to_owned();
    let (_, ticket) = call(
        &app,
        "POST",
        &format!("{path}/video-upload"),
        Some(&token),
        None,
    )
    .await;
    let current = ticket["video_id"].as_str().unwrap().to_owned();
    assert_eq!(stream.fake_deleted(), [abandoned.as_str()]);

    // A webhook about the dropped video changes nothing.
    assert_eq!(
        stream_webhook(&app, finished(&abandoned), true).await,
        StatusCode::OK
    );
    let (_, exercise) = call(&app, "GET", &path, Some(&token), None).await;
    assert_eq!(exercise["upload"]["status"], "uploading");

    stream.fake_set_status(&current, ERROR, 0);
    assert_eq!(
        stream_webhook(&app, finished(&current), true).await,
        StatusCode::OK
    );
    let (_, failed) = call(&app, "GET", &path, Some(&token), None).await;
    assert_eq!(failed["upload"]["status"], "failed");
    assert!(failed["video"].is_null());
}

#[sqlx::test]
async fn webhooks_must_be_signed_and_about_our_library(db: PgPool) {
    let (app, state, token) = setup(db).await;
    let stream = state.video.clone().unwrap();
    let exercise = create(&app, &token, "Жим лежачи").await;
    let path = format!("/coach/exercises/{}", exercise["id"].as_str().unwrap());
    let (_, ticket) = call(
        &app,
        "POST",
        &format!("{path}/video-upload"),
        Some(&token),
        None,
    )
    .await;
    let video_id = ticket["video_id"].as_str().unwrap().to_owned();
    stream.fake_set_status(&video_id, FINISHED, 20);

    assert_eq!(
        stream_webhook(&app, finished(&video_id), false).await,
        StatusCode::UNAUTHORIZED
    );
    let other_library = json!({ "VideoLibraryId": 999, "VideoGuid": video_id, "Status": 3 });
    assert_eq!(
        stream_webhook(&app, other_library, true).await,
        StatusCode::OK
    );
    assert_eq!(
        stream_webhook(&app, json!({ "unexpected": true }), true).await,
        StatusCode::OK
    );
    let (_, untouched) = call(&app, "GET", "/coach/exercises", Some(&token), None).await;
    // The list itself checks encodings; this one never got to "processing".
    assert_eq!(untouched[0]["upload"]["status"], "uploading");

    assert_eq!(
        stream_webhook(&app, finished(&video_id), true).await,
        StatusCode::OK
    );
    let (_, ready) = call(&app, "GET", &path, Some(&token), None).await;
    assert_eq!(ready["video"]["length_secs"], 20);
}

#[sqlx::test]
async fn without_bunny_the_library_works_but_uploads_are_off(db: PgPool) {
    let coach_id = seed_coach(&db, 555_000_222).await;
    let state = test_state(db);
    let token = coach_token(&state, coach_id);
    let app = coaching_backend::router(state);

    let exercise = create(&app, &token, "Планка").await;
    let path = format!(
        "/coach/exercises/{}/video-upload",
        exercise["id"].as_str().unwrap()
    );
    let (status, body) = call(&app, "POST", &path, Some(&token), None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], "video_not_configured");
    assert_eq!(
        stream_webhook(&app, finished("anything"), true).await,
        StatusCode::NOT_FOUND
    );
}
