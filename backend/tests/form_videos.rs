//! Clients' own technique videos under an exercise: uploaded from the phone to a
//! private Bunny library, played only through signed links, and flagged for
//! Dasha until she opens the report.

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
    telegram::Button,
    video::{StreamClient, StreamSettings},
};
use common::{
    FRONTEND_URL, call, coach_token, last_message_to, messages_to, seed_client, seed_coach,
    test_state,
};
use hmac::{Hmac, KeyInit, Mac};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::Sha256;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const COACH_TG: i64 = 555_000_222;
const CLIENT_TG: i64 = 777_000_111;
const LIBRARY_ID: &str = "54321";
const READ_ONLY_KEY: &str = "client-read-only-key";

struct Setup {
    app: Router,
    state: AppState,
    coach: String,
    client: String,
    other_client: String,
    workout_id: Uuid,
    squat: Uuid,
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

/// A published workout for Максим with squats, and a second client.
async fn setup(db: PgPool, configured: bool) -> Setup {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let other_id: Uuid = sqlx::query_scalar(
        "INSERT INTO clients (coach_id, name, telegram_id, bot_allowed_at)
         VALUES ($1, 'Олена К.', 777000222, now()) RETURNING id",
    )
    .bind(coach_id)
    .fetch_one(&db)
    .await
    .unwrap();
    let exercise_id: Uuid = sqlx::query_scalar(
        "INSERT INTO exercises (coach_id, name) VALUES ($1, 'Присідання') RETURNING id",
    )
    .bind(coach_id)
    .fetch_one(&db)
    .await
    .unwrap();
    let workout_id: Uuid = sqlx::query_scalar(
        "INSERT INTO workouts (coach_id, client_id, title, date, status, published_at)
         VALUES ($1, $2, 'Ноги', '2026-10-06', 'published', now()) RETURNING id",
    )
    .bind(coach_id)
    .bind(client_id)
    .fetch_one(&db)
    .await
    .unwrap();
    let squat: Uuid = sqlx::query_scalar(
        "INSERT INTO workout_exercises (workout_id, exercise_id, position)
         VALUES ($1, $2, 0) RETURNING id",
    )
    .bind(workout_id)
    .bind(exercise_id)
    .fetch_one(&db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workout_sets (workout_exercise_id, position, target_kg, target_reps_min, target_reps_max)
         VALUES ($1, 0, 60, 8, 8)",
    )
    .bind(squat)
    .execute(&db)
    .await
    .unwrap();

    let mut state = test_state(db);
    if configured {
        state = state.with_client_videos(Some(private_library()));
    }
    let token = |id| {
        state
            .jwt
            .issue(Role::Client, id, MINI_APP_TOKEN_TTL)
            .unwrap()
    };
    Setup {
        app: coaching_backend::router(state.clone()),
        coach: coach_token(&state, coach_id),
        client: token(client_id),
        other_client: token(other_id),
        state,
        workout_id,
        squat,
    }
}

async fn start(s: &Setup, token: &str) -> (StatusCode, Value) {
    call(
        &s.app,
        "POST",
        &format!("/workout-exercises/{}/videos", s.squat),
        Some(token),
        None,
    )
    .await
}

async fn client_videos(s: &Setup) -> Value {
    let (_, workout) = call(
        &s.app,
        "GET",
        &format!("/workouts/{}", s.workout_id),
        Some(&s.client),
        None,
    )
    .await;
    assert_eq!(workout["videos_enabled"], true);
    workout["exercises"][0]["videos"].clone()
}

async fn report_videos(s: &Setup) -> Value {
    let (_, results) = call(
        &s.app,
        "GET",
        &format!("/coach/workouts/{}/results", s.workout_id),
        Some(&s.coach),
        None,
    )
    .await;
    results["exercises"][0]["videos"].clone()
}

fn library(s: &Setup) -> &StreamClient {
    s.state.client_videos.as_ref().unwrap()
}

#[sqlx::test]
async fn a_video_goes_from_the_phone_to_dasha(db: PgPool) {
    let s = setup(db, true).await;

    let (status, upload) = start(&s, &s.client).await;
    assert_eq!(status, StatusCode::OK, "{upload}");
    assert_eq!(upload["ticket"]["library_id"], LIBRARY_ID);
    let guid = upload["ticket"]["video_id"].as_str().unwrap().to_owned();
    let id = upload["id"].as_str().unwrap().to_owned();
    assert_eq!(client_videos(&s).await[0]["status"], "uploading");
    assert!(
        report_videos(&s).await.as_array().unwrap().is_empty(),
        "Dasha sees it once the file is in"
    );

    let (status, _) = call(
        &s.app,
        "POST",
        &format!("/form-videos/{id}/uploaded"),
        Some(&s.client),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(report_videos(&s).await[0]["status"], "processing");
    assert!(messages_to(&s.state, COACH_TG).is_empty());

    // Bunny finishes encoding; the jobs notice without a webhook.
    library(&s).fake_set_status(&guid, 4, 42);
    jobs::tick(&s.state, Utc::now()).await.unwrap();
    let message = last_message_to(&s.state, COACH_TG);
    assert_eq!(
        message.text,
        "Максим К.: нове відео техніки — «Присідання»."
    );
    assert!(matches!(
        &message.keyboard[0][0],
        Button::WebApp { url, .. }
            if *url == format!("{FRONTEND_URL}/app/workouts/{}/report", s.workout_id)
    ));

    // Played only through signed links covering the video's directory.
    let video = &client_videos(&s).await[0];
    assert_eq!(video["status"], "ready");
    assert_eq!(video["length_secs"], 42);
    let hls = video["hls_url"].as_str().unwrap();
    assert!(
        hls.starts_with("https://vz-private.b-cdn.net/bcdn_token=HS256-"),
        "{hls}"
    );
    assert!(hls.contains(&format!("&token_path=%2F{guid}%2F&expires=")));
    assert!(hls.ends_with(&format!("/{guid}/playlist.m3u8")));
    assert!(
        video["thumbnail_url"]
            .as_str()
            .unwrap()
            .ends_with("/thumbnail.jpg")
    );

    // Flagged as new for Dasha until she opens the report.
    let (_, clients) = call(&s.app, "GET", "/coach/clients", Some(&s.coach), None).await;
    let maksym = clients
        .as_array()
        .unwrap()
        .iter()
        .find(|client| client["name"] == "Максим К.")
        .unwrap();
    assert_eq!(maksym["new_videos"], 1);
    let client_id = maksym["id"].as_str().unwrap();
    let (_, workouts) = call(
        &s.app,
        "GET",
        &format!("/coach/clients/{client_id}/workouts"),
        Some(&s.coach),
        None,
    )
    .await;
    assert_eq!(workouts[0]["new_videos"], 1);

    let (status, _) = call(
        &s.app,
        "POST",
        &format!("/coach/workouts/{}/report/seen", s.workout_id),
        Some(&s.coach),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "no report needed");
    assert_eq!(report_videos(&s).await[0]["seen"], true);
    let (_, clients) = call(&s.app, "GET", "/coach/clients", Some(&s.coach), None).await;
    assert!(
        clients
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["new_videos"] == 0)
    );

    // Told once.
    jobs::tick(&s.state, Utc::now()).await.unwrap();
    assert_eq!(messages_to(&s.state, COACH_TG).len(), 1);
}

#[sqlx::test]
async fn three_per_exercise_only_their_own_and_deletable(db: PgPool) {
    let s = setup(db, true).await;

    let mut started = Vec::new();
    for _ in 0..3 {
        let (status, upload) = start(&s, &s.client).await;
        assert_eq!(status, StatusCode::OK);
        started.push(upload);
    }
    let (status, body) = start(&s, &s.client).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "too_many_videos");

    // A failed upload does not count.
    let failed = &started[0];
    let failed_guid = failed["ticket"]["video_id"].as_str().unwrap();
    call(
        &s.app,
        "POST",
        &format!("/form-videos/{}/uploaded", failed["id"].as_str().unwrap()),
        Some(&s.client),
        None,
    )
    .await;
    library(&s).fake_set_status(failed_guid, 5, 0);
    jobs::tick(&s.state, Utc::now()).await.unwrap();
    assert_eq!(client_videos(&s).await[0]["status"], "failed");
    let (status, _) = start(&s, &s.client).await;
    assert_eq!(status, StatusCode::OK);

    // Someone else's workout, and someone else's video.
    let (status, _) = start(&s, &s.other_client).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let second = started[1]["id"].as_str().unwrap();
    let (status, _) = call(
        &s.app,
        "DELETE",
        &format!("/form-videos/{second}"),
        Some(&s.other_client),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Deleting removes it on Bunny too.
    let (status, _) = call(
        &s.app,
        "DELETE",
        &format!("/form-videos/{second}"),
        Some(&s.client),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(
        library(&s).fake_deleted().contains(
            &started[1]["ticket"]["video_id"]
                .as_str()
                .unwrap()
                .to_owned()
        )
    );
    assert_eq!(client_videos(&s).await.as_array().unwrap().len(), 3);

    // Not on a draft.
    sqlx::query("UPDATE workouts SET status = 'draft'")
        .execute(&s.state.db)
        .await
        .unwrap();
    let (status, _) = start(&s, &s.client).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn without_the_private_library_videos_are_off(db: PgPool) {
    let s = setup(db, false).await;
    let (status, body) = start(&s, &s.client).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], "client_videos_not_configured");
}

#[sqlx::test]
async fn the_private_librarys_webhook_is_signed(db: PgPool) {
    let s = setup(db, true).await;
    let (_, upload) = start(&s, &s.client).await;
    let guid = upload["ticket"]["video_id"].as_str().unwrap().to_owned();
    call(
        &s.app,
        "POST",
        &format!("/form-videos/{}/uploaded", upload["id"].as_str().unwrap()),
        Some(&s.client),
        None,
    )
    .await;
    library(&s).fake_set_status(&guid, 4, 30);

    let event = json!({ "VideoLibraryId": 54321, "VideoGuid": guid, "Status": 3 });
    assert_eq!(
        webhook(&s.app, &event, false).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(client_videos(&s).await[0]["status"], "processing");
    assert_eq!(webhook(&s.app, &event, true).await, StatusCode::OK);
    assert_eq!(client_videos(&s).await[0]["status"], "ready");

    // Another library's events are ignored.
    let other = json!({ "VideoLibraryId": 1, "VideoGuid": "x", "Status": 3 });
    assert_eq!(webhook(&s.app, &other, true).await, StatusCode::OK);
}

async fn webhook(app: &Router, body: &Value, signed: bool) -> StatusCode {
    let body = body.to_string();
    let mut request = Request::builder()
        .method("POST")
        .uri("/webhooks/client-videos")
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
