//! How an exercise is counted: kg × reps, bodyweight (with optional extra
//! weight), or time, with seconds in the reps fields.

mod common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use coaching_backend::auth::{Role, jwt::MINI_APP_TOKEN_TTL};
use common::{call, coach_token, seed_client, seed_coach, test_state};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

struct Setup {
    app: Router,
    coach: String,
    client: String,
    client_id: Uuid,
}

async fn setup(db: PgPool) -> Setup {
    let coach_id = seed_coach(&db, 555_000_222).await;
    let client_id = seed_client(&db, coach_id, 777_000_111).await;
    let state = test_state(db);
    Setup {
        app: coaching_backend::router(state.clone()),
        coach: coach_token(&state, coach_id),
        client: state
            .jwt
            .issue(Role::Client, client_id, MINI_APP_TOKEN_TTL)
            .unwrap(),
        client_id,
    }
}

async fn exercise(s: &Setup, name: &str, measure: Option<&str>) -> Value {
    let mut body = json!({ "name": name });
    if let Some(measure) = measure {
        body["measure"] = json!(measure);
    }
    let (status, exercise) = call(
        &s.app,
        "POST",
        "/coach/exercises",
        Some(&s.coach),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{exercise}");
    exercise
}

async fn change_measure(s: &Setup, exercise: &Value, measure: &str) -> (StatusCode, Value) {
    call(
        &s.app,
        "PATCH",
        &format!("/coach/exercises/{}", exercise["id"].as_str().unwrap()),
        Some(&s.coach),
        Some(json!({ "measure": measure })),
    )
    .await
}

fn set(kg: Option<f64>, min: i32, max: i32) -> Value {
    json!({ "id": Uuid::new_v4(), "kg": kg, "reps_min": min, "reps_max": max })
}

/// Creates a workout on 2026-10-06 with these exercises (id and sets) and saves it.
async fn workout(s: &Setup, exercises: Vec<(&Value, Vec<Value>)>) -> (String, StatusCode, Value) {
    let (_, created) = call(
        &s.app,
        "POST",
        "/coach/workouts",
        Some(&s.coach),
        Some(json!({ "client_id": s.client_id, "date": "2026-10-06" })),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_owned();
    let exercises: Vec<Value> = exercises
        .into_iter()
        .map(|(exercise, sets)| {
            json!({
                "id": Uuid::new_v4(), "exercise_id": exercise["id"], "per_side_label": null,
                "note": null, "sets": sets,
            })
        })
        .collect();
    let body = json!({ "title": "Кор", "date": "2026-10-06", "exercises": exercises });
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/coach/workouts/{id}"))
        .header(header::AUTHORIZATION, format!("Bearer {}", s.coach))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, "1")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = s.app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let saved = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (id, status, saved)
}

#[sqlx::test]
async fn the_library_says_how_each_exercise_is_counted(db: PgPool) {
    let s = setup(db).await;

    let rows = exercise(&s, "Тяга гантелі в нахилі", None).await;
    assert_eq!(rows["measure"], "weight", "the default");
    let pull_ups = exercise(&s, "Підтягування", Some("bodyweight")).await;
    assert_eq!(pull_ups["measure"], "bodyweight");
    let plank = exercise(&s, "Планка", Some("time")).await;
    assert_eq!(plank["measure"], "time");

    // Unused exercises switch freely.
    let (status, changed) = change_measure(&s, &rows, "time").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(changed["measure"], "time");
    change_measure(&s, &rows, "weight").await;

    let (_, status, _) = workout(
        &s,
        vec![
            (&rows, vec![set(Some(36.0), 12, 12)]),
            (&pull_ups, vec![set(None, 17, 17)]),
            (&plank, vec![set(None, 45, 60)]),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Weight and bodyweight count the same units, so that switch stays free...
    let (status, changed) = change_measure(&s, &rows, "bodyweight").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(changed["measure"], "bodyweight");
    let (status, _) = change_measure(&s, &rows, "weight").await;
    assert_eq!(status, StatusCode::OK);

    // ...but used reps must not turn into seconds, or back.
    let (status, body) = change_measure(&s, &pull_ups, "time").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "measure_in_use");
    let (status, body) = change_measure(&s, &plank, "bodyweight").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "measure_in_use");
    // Setting the measure it already has is not a switch.
    let (status, _) = change_measure(&s, &plank, "time").await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn timed_and_bodyweight_sets_go_from_builder_to_report(db: PgPool) {
    let s = setup(db).await;
    let pull_ups = exercise(&s, "Підтягування", Some("bodyweight")).await;
    let bike = exercise(&s, "Велотренажер", Some("time")).await;

    // Pull-ups with a +10 kg belt on the last set; ten minutes on the bike,
    // longer than any rep count.
    let (id, status, saved) = workout(
        &s,
        vec![
            (&pull_ups, vec![set(None, 12, 12), set(Some(10.0), 8, 8)]),
            (&bike, vec![set(None, 600, 600)]),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["exercises"][0]["measure"], "bodyweight");
    assert_eq!(saved["exercises"][1]["measure"], "time");
    assert_eq!(saved["exercises"][1]["sets"][0]["reps_min"], 600);
    call(
        &s.app,
        "POST",
        &format!("/coach/workouts/{id}/publish"),
        Some(&s.coach),
        None,
    )
    .await;

    let (_, opened) = call(
        &s.app,
        "GET",
        &format!("/workouts/{id}"),
        Some(&s.client),
        None,
    )
    .await;
    assert_eq!(opened["exercises"][0]["measure"], "bodyweight");
    assert_eq!(opened["exercises"][0]["sets"][1]["target_kg"], 10.0);
    assert_eq!(opened["exercises"][1]["measure"], "time");

    // Eleven minutes instead of ten.
    let bike_set = opened["exercises"][1]["sets"][0]["id"].as_str().unwrap();
    let (status, _) = call(
        &s.app,
        "PUT",
        &format!("/sets/{bike_set}/result"),
        Some(&s.client),
        Some(json!({
            "actual_kg": null, "actual_reps": 660, "completed": true,
            "client_updated_at": "2026-10-06T09:00:00Z",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, results) = call(
        &s.app,
        "GET",
        &format!("/coach/workouts/{id}/results"),
        Some(&s.coach),
        None,
    )
    .await;
    assert_eq!(results["exercises"][1]["measure"], "time");
    let done = &results["exercises"][1]["sets"][0];
    assert_eq!(done["actual_reps"], 660);
    assert_eq!(done["differs"], true);
}
