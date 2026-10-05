//! Dasha's description and photos of a library exercise, shown to clients in
//! their workout through signed links.

mod common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use coaching_backend::{
    auth::{Role, jwt::MINI_APP_TOKEN_TTL},
    storage::{StorageClient, StorageSettings},
};
use common::{call, coach_token, seed_client, seed_coach, test_state};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

/// The start of a JPEG file, which is all the backend checks.
const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F'];

async fn post_photo(
    app: &Router,
    token: &str,
    exercise: &str,
    bytes: &[u8],
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("POST")
        .uri(format!("/coach/exercises/{exercise}/photos"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "image/jpeg")
        .body(Body::from(bytes.to_vec()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[sqlx::test]
async fn an_exercise_gets_a_description_and_photos_clients_see(db: PgPool) {
    let coach_id = seed_coach(&db, 555_000_222).await;
    let other_coach_id = seed_coach(&db, 555_000_333).await;
    let client_id = seed_client(&db, coach_id, 777_000_111).await;
    let storage = StorageClient::fake(StorageSettings {
        zone: "photos-test".to_owned(),
        hostname: "storage.bunnycdn.com".to_owned(),
        password: "storage-password".to_owned(),
        cdn_hostname: "photos-test.b-cdn.net".to_owned(),
        token_key: "photos-token-key".to_owned(),
    });
    let state = test_state(db.clone()).with_storage(Some(storage.clone()));
    let app = coaching_backend::router(state.clone());
    let coach = coach_token(&state, coach_id);
    let other = coach_token(&state, other_coach_id);

    let (_, exercise) = call(
        &app,
        "POST",
        "/coach/exercises",
        Some(&coach),
        Some(json!({ "name": "Тяга верхнього блоку" })),
    )
    .await;
    let id = exercise["id"].as_str().unwrap().to_owned();
    assert!(exercise["description"].is_null());
    assert_eq!(exercise["photos"], json!([]));
    let path = format!("/coach/exercises/{id}");
    let describe = async |text: String| {
        call(
            &app,
            "PATCH",
            &path,
            Some(&coach),
            Some(json!({ "description": text })),
        )
        .await
    };

    let (status, described) = describe("  Широка рукоятка, хват зверху  ".to_owned()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(described["description"], "Широка рукоятка, хват зверху");
    let (status, body) = describe("а".repeat(1001)).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("description_too_long"))
    );

    // Photos: stored per exercise, signed for its folder, five at most.
    let (status, photo) = post_photo(&app, &coach, &id, JPEG).await;
    assert_eq!(status, StatusCode::CREATED, "{photo}");
    let photo_id = photo["id"].as_str().unwrap().to_owned();
    let file = format!("exercises/{id}/{photo_id}.jpg");
    assert_eq!(storage.fake_files(), std::slice::from_ref(&file));
    let url = photo["url"].as_str().unwrap();
    assert!(
        url.contains(&format!("token_path=%2Fexercises%2F{id}%2F")),
        "{url}"
    );
    assert!(url.ends_with(&format!("/{file}")));
    let (status, body) = post_photo(&app, &coach, &id, b"not a photo").await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_photo"))
    );
    assert_eq!(
        post_photo(&app, &other, &id, JPEG).await.0,
        StatusCode::NOT_FOUND
    );
    for _ in 1..5 {
        assert_eq!(
            post_photo(&app, &coach, &id, JPEG).await.0,
            StatusCode::CREATED
        );
    }
    let (status, body) = post_photo(&app, &coach, &id, JPEG).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::CONFLICT, &json!("too_many_photos"))
    );
    let (_, fetched) = call(&app, "GET", &path, Some(&coach), None).await;
    assert_eq!(fetched["photos"].as_array().unwrap().len(), 5);

    // The client sees both in their workout.
    let exercise_id: Uuid = id.parse().unwrap();
    let workout_id: Uuid = sqlx::query_scalar(
        "INSERT INTO workouts (coach_id, client_id, title, date, status, published_at)
         VALUES ($1, $2, 'Спина', '2026-10-06', 'published', now()) RETURNING id",
    )
    .bind(coach_id)
    .bind(client_id)
    .fetch_one(&db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workout_exercises (workout_id, exercise_id, position) VALUES ($1, $2, 0)",
    )
    .bind(workout_id)
    .bind(exercise_id)
    .execute(&db)
    .await
    .unwrap();
    let client = state
        .jwt
        .issue(Role::Client, client_id, MINI_APP_TOKEN_TTL)
        .unwrap();
    let (_, workout) = call(
        &app,
        "GET",
        &format!("/workouts/{workout_id}"),
        Some(&client),
        None,
    )
    .await;
    let shown = &workout["exercises"][0];
    assert_eq!(shown["description"], "Широка рукоятка, хват зверху");
    assert_eq!(shown["photos"].as_array().unwrap().len(), 5);

    // Deleting: only Dasha's own, and the file goes too. An empty description clears it.
    let photo_path = format!("/coach/exercise-photos/{photo_id}");
    assert_eq!(
        call(&app, "DELETE", &photo_path, Some(&other), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, "DELETE", &photo_path, Some(&coach), None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert!(!storage.fake_files().contains(&file));
    assert_eq!(storage.fake_files().len(), 4);
    let (_, cleared) = describe(String::new()).await;
    assert!(cleared["description"].is_null());
}
