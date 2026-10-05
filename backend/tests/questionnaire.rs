//! The questionnaire a client fills in for Dasha: answers, and gym photos and
//! videos kept privately on Bunny and shown only through signed links.

mod common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::Utc;
use coaching_backend::{
    auth::{Role, jwt::MINI_APP_TOKEN_TTL},
    jobs,
    state::AppState,
    storage::{StorageClient, StorageSettings},
    video::{StreamClient, StreamSettings},
};
use common::{call, coach_token, seed_client, seed_coach, test_state};
use hmac::{Hmac, KeyInit, Mac};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::Sha256;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const LIBRARY_ID: &str = "54321";
const READ_ONLY_KEY: &str = "client-read-only-key";
// Bunny's API statuses.
const TRANSCODING: i32 = 3;
const FINISHED: i32 = 4;
/// The start of a JPEG file, which is all the backend checks.
const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F'];

struct Setup {
    app: Router,
    state: AppState,
    client_id: Uuid,
    client: String,
    other_client: String,
    coach: String,
    other_coach: String,
}

fn photo_storage() -> StorageClient {
    StorageClient::fake(StorageSettings {
        zone: "photos-test".to_owned(),
        hostname: "storage.bunnycdn.com".to_owned(),
        password: "storage-password".to_owned(),
        cdn_hostname: "photos-test.b-cdn.net".to_owned(),
        token_key: "photos-token-key".to_owned(),
    })
}

fn private_library() -> StreamClient {
    StreamClient::fake(StreamSettings {
        library_id: LIBRARY_ID.to_owned(),
        api_key: "client-api-key".to_owned(),
        read_only_api_key: Some(READ_ONLY_KEY.to_owned()),
        cdn_hostname: "vz-private.b-cdn.net".to_owned(),
        token_key: Some("client-token-key".to_owned()),
    })
}

/// Максим, another client, Dasha, and another coach; Bunny is faked unless
/// `configured` is false.
async fn setup(db: PgPool, configured: bool) -> Setup {
    let coach_id = seed_coach(&db, 555_000_222).await;
    let other_coach_id = seed_coach(&db, 555_000_333).await;
    let client_id = seed_client(&db, coach_id, 777_000_111).await;
    let other_id = seed_client(&db, coach_id, 777_000_222).await;
    let mut state = test_state(db);
    if configured {
        state = state
            .with_storage(Some(photo_storage()))
            .with_client_videos(Some(private_library()));
    }
    let client_token = |id| {
        state
            .jwt
            .issue(Role::Client, id, MINI_APP_TOKEN_TTL)
            .unwrap()
    };
    Setup {
        app: coaching_backend::router(state.clone()),
        client: client_token(client_id),
        other_client: client_token(other_id),
        coach: coach_token(&state, coach_id),
        other_coach: coach_token(&state, other_coach_id),
        state,
        client_id,
    }
}

async fn post_photo(s: &Setup, token: &str, bytes: &[u8]) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("POST")
        .uri("/me/gym/photos")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "image/jpeg")
        .body(Body::from(bytes.to_vec()))
        .unwrap();
    let response = s.app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn coach_view(s: &Setup) -> Value {
    let path = format!("/coach/clients/{}/questionnaire", s.client_id);
    let (status, body) = call(&s.app, "GET", &path, Some(&s.coach), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn coach_client(s: &Setup) -> Value {
    let path = format!("/coach/clients/{}", s.client_id);
    call(&s.app, "GET", &path, Some(&s.coach), None).await.1
}

#[sqlx::test]
async fn answers_are_checked_saved_and_shown_to_dasha(db: PgPool) {
    let s = setup(db, true).await;
    let (status, empty) = call(&s.app, "GET", "/me/questionnaire", Some(&s.client), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        empty,
        json!({ "birth_year": null, "sex": null, "height_cm": null, "photos": [], "videos": [] })
    );
    let (_, me) = call(&s.app, "GET", "/me", Some(&s.client), None).await;
    assert_eq!(me["questionnaire_started"], false);

    let save = |answers: Value| {
        call(
            &s.app,
            "PUT",
            "/me/questionnaire",
            Some(&s.client),
            Some(answers),
        )
    };
    let (status, body) = save(json!({ "birth_year": 1900 })).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_birth_year"))
    );
    let (status, body) = save(json!({ "height_cm": 300 })).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_height"))
    );
    let (status, saved) =
        save(json!({ "birth_year": 1991, "sex": "male", "height_cm": 182 })).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["birth_year"], 1991);
    assert_eq!(saved["sex"], "male");

    let (_, me) = call(&s.app, "GET", "/me", Some(&s.client), None).await;
    assert_eq!(me["questionnaire_started"], true);
    let view = coach_view(&s).await;
    assert_eq!(
        (&view["sex"], &view["height_cm"]),
        (&json!("male"), &json!(182))
    );
    let client = coach_client(&s).await;
    assert_eq!(client["birth_year"], 1991);
    assert_eq!(
        (&client["gym_photos"], &client["gym_videos"]),
        (&json!(0), &json!(0))
    );

    // Only Dasha, not another coach or a client.
    let path = format!("/coach/clients/{}/questionnaire", s.client_id);
    let (status, _) = call(&s.app, "GET", &path, Some(&s.other_coach), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(&s.app, "GET", &path, Some(&s.client), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Leaving an answer out clears it.
    let (_, cleared) = save(json!({ "sex": "male" })).await;
    assert!(cleared["birth_year"].is_null() && cleared["height_cm"].is_null());
}

#[sqlx::test]
async fn gym_photos_are_private_limited_and_deletable(db: PgPool) {
    let s = setup(db, true).await;
    let storage = s.state.storage.clone().unwrap();

    let (status, photo) = post_photo(&s, &s.client, JPEG).await;
    assert_eq!(status, StatusCode::CREATED, "{photo}");
    let id = photo["id"].as_str().unwrap();
    let path = format!("clients/{}/gym/{id}.jpg", s.client_id);
    assert_eq!(storage.fake_files(), std::slice::from_ref(&path));
    // Served only through a link signed for the client's gym folder.
    let url = photo["url"].as_str().unwrap();
    assert!(
        url.starts_with("https://photos-test.b-cdn.net/bcdn_token=HS256-"),
        "{url}"
    );
    assert!(url.contains(&format!("token_path=%2Fclients%2F{}%2Fgym%2F", s.client_id)));
    assert!(url.ends_with(&format!("/{path}")));

    let (status, body) = post_photo(&s, &s.client, b"not a photo").await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_photo"))
    );
    for _ in 1..10 {
        assert_eq!(post_photo(&s, &s.client, JPEG).await.0, StatusCode::CREATED);
    }
    let (status, body) = post_photo(&s, &s.client, JPEG).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::CONFLICT, &json!("too_many_photos"))
    );
    assert_eq!(coach_view(&s).await["photos"].as_array().unwrap().len(), 10);
    assert_eq!(coach_client(&s).await["gym_photos"], 10);

    // Only their own, and deleting removes the file too.
    let mine = format!("/me/gym/{id}");
    let (status, _) = call(&s.app, "DELETE", &mine, Some(&s.other_client), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(&s.app, "DELETE", &mine, Some(&s.client), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(storage.fake_files().len(), 9);
    assert!(!storage.fake_files().contains(&path));
}

#[sqlx::test]
async fn without_storage_photos_and_videos_are_off(db: PgPool) {
    let s = setup(db, false).await;
    let (status, body) = post_photo(&s, &s.client, JPEG).await;
    assert_eq!(
        (status, &body["error"]),
        (
            StatusCode::SERVICE_UNAVAILABLE,
            &json!("photos_not_configured")
        )
    );
    let (status, body) = call(&s.app, "POST", "/me/gym/videos", Some(&s.client), None).await;
    assert_eq!(
        (status, &body["error"]),
        (
            StatusCode::SERVICE_UNAVAILABLE,
            &json!("client_videos_not_configured")
        )
    );
}

#[sqlx::test]
async fn gym_videos_go_through_the_private_library(db: PgPool) {
    let s = setup(db, true).await;
    let library = s.state.client_videos.clone().unwrap();
    let start = || call(&s.app, "POST", "/me/gym/videos", Some(&s.client), None);

    let (status, upload) = start().await;
    assert_eq!(status, StatusCode::OK, "{upload}");
    assert_eq!(upload["ticket"]["library_id"], LIBRARY_ID);
    assert_eq!(upload["title"], "Максим К. · зал");
    let id = upload["id"].as_str().unwrap();
    let guid = upload["ticket"]["video_id"].as_str().unwrap().to_owned();

    // Still uploading: the client sees it, Dasha does not yet.
    let (_, mine) = call(&s.app, "GET", "/me/questionnaire", Some(&s.client), None).await;
    assert_eq!(mine["videos"][0]["status"], "uploading");
    assert!(
        coach_view(&s).await["videos"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    // Sent; Bunny is still encoding, then finishes without a webhook.
    library.fake_set_status(&guid, TRANSCODING, 0);
    let uploaded = format!("/me/gym/videos/{id}/uploaded");
    let (status, _) = call(&s.app, "POST", &uploaded, Some(&s.client), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(coach_view(&s).await["videos"][0]["status"], "processing");
    library.fake_set_status(&guid, FINISHED, 42);
    jobs::tick(&s.state, Utc::now()).await.unwrap();

    let video = &coach_view(&s).await["videos"][0];
    assert_eq!(video["status"], "ready");
    assert_eq!(video["length_secs"], 42);
    let hls = video["hls_url"].as_str().unwrap();
    assert!(
        hls.starts_with("https://vz-private.b-cdn.net/bcdn_token=HS256-"),
        "{hls}"
    );
    assert_eq!(coach_client(&s).await["gym_videos"], 1);

    // The library's webhook moves gym videos along too.
    let (_, second) = start().await;
    let second_guid = second["ticket"]["video_id"].as_str().unwrap().to_owned();
    let second_uploaded = format!("/me/gym/videos/{}/uploaded", second["id"].as_str().unwrap());
    library.fake_set_status(&second_guid, TRANSCODING, 0);
    call(&s.app, "POST", &second_uploaded, Some(&s.client), None).await;
    library.fake_set_status(&second_guid, FINISHED, 30);
    let event = json!({ "VideoLibraryId": 54321, "VideoGuid": second_guid, "Status": 3 });
    assert_eq!(webhook(&s.app, &event).await, StatusCode::OK);
    assert_eq!(coach_view(&s).await["videos"][1]["status"], "ready");

    // Three at most; deleting one removes it from Bunny and frees the slot.
    assert_eq!(start().await.0, StatusCode::OK);
    let (status, body) = start().await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::CONFLICT, &json!("too_many_videos"))
    );
    let (status, _) = call(
        &s.app,
        "DELETE",
        &format!("/me/gym/{id}"),
        Some(&s.client),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(library.fake_deleted(), [guid]);
    assert_eq!(start().await.0, StatusCode::OK);
}

/// Posts a webhook to the client video library, signed like Bunny signs it.
async fn webhook(app: &Router, body: &Value) -> StatusCode {
    let body = body.to_string();
    let mut mac = Hmac::<Sha256>::new_from_slice(READ_ONLY_KEY.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let request = Request::builder()
        .method("POST")
        .uri("/webhooks/client-videos")
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-BunnyStream-Signature-Version", "v1")
        .header("X-BunnyStream-Signature-Algorithm", "hmac-sha256")
        .header(
            "X-BunnyStream-Signature",
            hex::encode(mac.finalize().into_bytes()),
        )
        .body(Body::from(body))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    response.into_body().collect().await.unwrap();
    status
}
