//! The workout builder's API: create, save as one document, copy, publish.

mod common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use coaching_backend::state::AppState;
use common::{call, coach_token, messages_to, seed_client, seed_coach, test_state};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const COACH_TG: i64 = 555_000_222;
const CLIENT_TG: i64 = 777_000_111;

struct Setup {
    app: Router,
    state: AppState,
    token: String,
    client_id: Uuid,
    pull_ups: Uuid,
    rows: Uuid,
}

async fn setup(db: PgPool) -> Setup {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let exercise = |name: &'static str| {
        let db = db.clone();
        async move {
            sqlx::query_scalar!(
                "INSERT INTO exercises (coach_id, name) VALUES ($1, $2) RETURNING id",
                coach_id,
                name,
            )
            .fetch_one(&db)
            .await
            .unwrap()
        }
    };
    let pull_ups = exercise("Підтягування").await;
    let rows = exercise("Тяга гантелі в нахилі").await;
    let state = test_state(db);
    let token = coach_token(&state, coach_id);
    Setup {
        app: coaching_backend::router(state.clone()),
        state,
        token,
        client_id,
        pull_ups,
        rows,
    }
}

async fn create(s: &Setup, body: Value) -> Value {
    let (status, workout) = call(
        &s.app,
        "POST",
        "/coach/workouts",
        Some(&s.token),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{workout}");
    workout
}

/// PUTs the whole workout with `If-Match`.
async fn save(s: &Setup, id: &Value, version: Option<i64>, body: Value) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method("PUT")
        .uri(format!("/coach/workouts/{}", id.as_str().unwrap()))
        .header(header::AUTHORIZATION, format!("Bearer {}", s.token))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(version) = version {
        request = request.header(header::IF_MATCH, format!("\"{version}\""));
    }
    let response = s
        .app
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn set(id: Uuid, kg: Option<f64>, reps_min: i32, reps_max: i32) -> Value {
    json!({ "id": id, "kg": kg, "reps_min": reps_min, "reps_max": reps_max })
}

/// Her back workout, cut down: pull-ups, then dumbbell rows per arm.
fn back_workout(s: &Setup, ids: &[Uuid; 7]) -> Value {
    json!({
        "title": "  Спина  ",
        "date": "2026-10-06",
        "exercises": [
            {
                "id": ids[0], "exercise_id": s.pull_ups, "per_side_label": null, "note": "",
                "sets": [set(ids[1], None, 17, 17), set(ids[2], None, 14, 14)],
            },
            {
                "id": ids[3], "exercise_id": s.rows, "per_side_label": "на кожну руку",
                "note": "Лікоть уздовж корпусу",
                "sets": [set(ids[4], Some(36.0), 12, 12), set(ids[5], Some(36.0), 8, 10), set(ids[6], Some(27.5), 15, 15)],
            },
        ],
    })
}

fn new_ids() -> [Uuid; 7] {
    std::array::from_fn(|_| Uuid::new_v4())
}

#[sqlx::test]
async fn the_builder_saves_the_whole_workout_and_keeps_row_ids(db: PgPool) {
    let s = setup(db.clone()).await;
    let workout = create(&s, json!({ "client_id": s.client_id })).await;
    assert_eq!(workout["status"], "draft");
    assert_eq!(workout["version"], 1);
    assert!(workout["date"].is_null() && workout["exercises"].as_array().unwrap().is_empty());

    let ids = new_ids();
    let (status, saved) = save(&s, &workout["id"], Some(1), back_workout(&s, &ids)).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["version"], 2);
    assert_eq!(saved["title"], "Спина");
    assert_eq!(saved["date"], "2026-10-06");
    let exercises = saved["exercises"].as_array().unwrap();
    assert_eq!(exercises[0]["name"], "Підтягування");
    assert!(exercises[0]["note"].is_null(), "an empty note is no note");
    assert_eq!(exercises[1]["per_side_label"], "на кожну руку");
    assert_eq!(exercises[1]["sets"][1], set(ids[5], Some(36.0), 8, 10));
    assert_eq!(exercises[1]["sets"][2]["kg"], 27.5);

    // The client logged a result on the first row set.
    sqlx::query!(
        "UPDATE workout_sets SET actual_kg = 36, actual_reps = 12, completed_at = now() WHERE id = $1",
        ids[4],
    )
    .execute(&db)
    .await
    .unwrap();

    // Swap the exercises, drop a pull-up set, change a target.
    let edited = json!({
        "title": "Спина",
        "date": "2026-10-06",
        "exercises": [
            {
                "id": ids[3], "exercise_id": s.rows, "per_side_label": "на кожну руку", "note": null,
                "sets": [set(ids[4], Some(38.0), 10, 10), set(ids[6], Some(27.5), 15, 15)],
            },
            {
                "id": ids[0], "exercise_id": s.pull_ups, "per_side_label": null, "note": null,
                "sets": [set(ids[1], None, 17, 17)],
            },
        ],
    });
    let (status, saved) = save(&s, &workout["id"], Some(2), edited).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["version"], 3);
    assert_eq!(saved["exercises"][0]["id"], json!(ids[3]));
    assert_eq!(
        saved["exercises"][0]["sets"][0],
        set(ids[4], Some(38.0), 10, 10)
    );
    assert_eq!(saved["exercises"][1]["sets"].as_array().unwrap().len(), 1);
    let result = sqlx::query!(
        r#"SELECT actual_kg::float8 AS "kg", actual_reps FROM workout_sets WHERE id = $1"#,
        ids[4],
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(
        (result.kg, result.actual_reps),
        (Some(36.0), Some(12)),
        "results survive edits"
    );

    // A save based on an old version is refused, and so is one without a version.
    let (status, body) = save(&s, &workout["id"], Some(2), back_workout(&s, &ids)).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::CONFLICT, &json!("version_conflict"))
    );
    let (status, body) = save(&s, &workout["id"], None, back_workout(&s, &ids)).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("version_required"))
    );
}

#[sqlx::test]
async fn saves_are_checked(db: PgPool) {
    let s = setup(db.clone()).await;
    let workout = create(&s, json!({ "client_id": s.client_id })).await;
    let id = &workout["id"];

    let mut bad_reps = back_workout(&s, &new_ids());
    bad_reps["exercises"][0]["sets"][0]["reps_min"] = json!(0);
    let (status, body) = save(&s, id, Some(1), bad_reps).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_set"))
    );

    let mut range_backwards = back_workout(&s, &new_ids());
    range_backwards["exercises"][1]["sets"][1]["reps_max"] = json!(6);
    let (status, _) = save(&s, id, Some(1), range_backwards).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let mut duplicate_ids = back_workout(&s, &new_ids());
    duplicate_ids["exercises"][1]["sets"][0]["id"] = duplicate_ids["exercises"][0]["id"].clone();
    let (status, body) = save(&s, id, Some(1), duplicate_ids).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_ids"))
    );

    // Another coach's exercise.
    let other_coach = seed_coach(&db, 111).await;
    let theirs = sqlx::query_scalar!(
        "INSERT INTO exercises (coach_id, name) VALUES ($1, 'Чуже') RETURNING id",
        other_coach,
    )
    .fetch_one(&db)
    .await
    .unwrap();
    let mut foreign_exercise = back_workout(&s, &new_ids());
    foreign_exercise["exercises"][0]["exercise_id"] = json!(theirs);
    let (status, body) = save(&s, id, Some(1), foreign_exercise).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("unknown_exercise"))
    );

    // Rows of another workout cannot be pulled in by ID.
    let ids = new_ids();
    let other = create(&s, json!({ "client_id": s.client_id })).await;
    let (status, _) = save(&s, &other["id"], Some(1), back_workout(&s, &ids)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = save(&s, id, Some(1), back_workout(&s, &ids)).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_ids"))
    );

    // Nothing above changed the workout.
    let (_, unchanged) = call(
        &s.app,
        "GET",
        &format!("/coach/workouts/{}", id.as_str().unwrap()),
        Some(&s.token),
        None,
    )
    .await;
    assert_eq!(unchanged["version"], 1);

    // Another coach sees none of it.
    let other_token = coach_token(&s.state, other_coach);
    let path = format!("/coach/workouts/{}", id.as_str().unwrap());
    let (status, _) = call(&s.app, "GET", &path, Some(&other_token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &s.app,
        "POST",
        "/coach/workouts",
        Some(&other_token),
        Some(json!({ "client_id": s.client_id })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn copying_makes_next_weeks_workout(db: PgPool) {
    let s = setup(db).await;
    let source = create(&s, json!({ "client_id": s.client_id })).await;
    let ids = new_ids();
    save(&s, &source["id"], Some(1), back_workout(&s, &ids)).await;

    let copy = create(
        &s,
        json!({ "client_id": s.client_id, "copy_from": source["id"] }),
    )
    .await;
    assert_eq!(copy["title"], "Спина");
    assert_eq!(copy["date"], "2026-10-13", "same weekday, a week later");
    assert_eq!(copy["status"], "draft");
    let exercises = copy["exercises"].as_array().unwrap();
    assert_eq!(exercises.len(), 2);
    assert_ne!(exercises[0]["id"], json!(ids[0]), "rows are new");
    assert_ne!(exercises[1]["sets"][0]["id"], json!(ids[4]));
    assert_eq!(exercises[1]["sets"][1]["reps_max"], 10);
    assert_eq!(exercises[1]["note"], "Лікоть уздовж корпусу");

    let dated = create(
        &s,
        json!({ "client_id": s.client_id, "copy_from": source["id"], "date": "2026-10-08" }),
    )
    .await;
    assert_eq!(dated["date"], "2026-10-08");

    let (_, list) = call(
        &s.app,
        "GET",
        &format!("/coach/clients/{}/workouts", s.client_id),
        Some(&s.token),
        None,
    )
    .await;
    let dates: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["date"].clone())
        .collect();
    assert_eq!(
        dates,
        [
            json!("2026-10-13"),
            json!("2026-10-08"),
            json!("2026-10-06")
        ]
    );
    assert_eq!(list[2]["exercise_count"], 2);
    assert_eq!(list[2]["set_count"], 5);

    let (status, _) = call(
        &s.app,
        "DELETE",
        &format!("/coach/workouts/{}", dated["id"].as_str().unwrap()),
        Some(&s.token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, list) = call(
        &s.app,
        "GET",
        &format!("/coach/clients/{}/workouts", s.client_id),
        Some(&s.token),
        None,
    )
    .await;
    assert_eq!(list.as_array().unwrap().len(), 2);
}

#[sqlx::test]
async fn publishing_tells_the_client_once(db: PgPool) {
    let s = setup(db).await;
    let workout = create(&s, json!({ "client_id": s.client_id })).await;
    let publish_path = format!(
        "/coach/workouts/{}/publish",
        workout["id"].as_str().unwrap()
    );

    let (status, body) = call(&s.app, "POST", &publish_path, Some(&s.token), None).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("date_required"))
    );

    let mut no_sets = back_workout(&s, &new_ids());
    no_sets["exercises"][0]["sets"] = json!([]);
    save(&s, &workout["id"], Some(1), no_sets).await;
    let (status, body) = call(&s.app, "POST", &publish_path, Some(&s.token), None).await;
    assert_eq!(
        (status, &body["error"]),
        (StatusCode::BAD_REQUEST, &json!("exercise_without_sets"))
    );

    save(&s, &workout["id"], Some(2), back_workout(&s, &new_ids())).await;
    let (status, published) = call(&s.app, "POST", &publish_path, Some(&s.token), None).await;
    assert_eq!(status, StatusCode::OK, "{published}");
    assert_eq!(published["workout"]["status"], "published");
    assert!(published["workout"]["published_at"].is_string());
    assert_eq!(published["client_notified"], true);

    let messages = messages_to(&s.state, CLIENT_TG);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].text,
        "Даша: нове тренування «Спина», вт, 6 жовтня."
    );

    // Publishing again, e.g. after an edit, does not message the client again.
    let (_, again) = call(&s.app, "POST", &publish_path, Some(&s.token), None).await;
    assert_eq!(again["client_notified"], true);
    assert_eq!(messages_to(&s.state, CLIENT_TG).len(), 1);
}

#[sqlx::test]
async fn a_client_who_has_not_joined_is_not_notified(db: PgPool) {
    let s = setup(db.clone()).await;
    let pending = sqlx::query_scalar!(
        "INSERT INTO clients (coach_id, name) SELECT coach_id, 'Аліна Р.' FROM clients WHERE id = $1
         RETURNING id",
        s.client_id,
    )
    .fetch_one(&db)
    .await
    .unwrap();
    let workout = create(&s, json!({ "client_id": pending })).await;
    save(&s, &workout["id"], Some(1), back_workout(&s, &new_ids())).await;
    let (status, published) = call(
        &s.app,
        "POST",
        &format!(
            "/coach/workouts/{}/publish",
            workout["id"].as_str().unwrap()
        ),
        Some(&s.token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(published["client_notified"], false);
    assert_eq!(published["workout"]["status"], "published");
}
