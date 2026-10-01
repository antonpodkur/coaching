//! The client's side: seeing published workouts, logging sets (also late,
//! from an offline queue), "last time", and finishing with a report.

mod common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use coaching_backend::{
    auth::{Role, jwt::MINI_APP_TOKEN_TTL},
    state::AppState,
    telegram::Button,
};
use common::{
    FRONTEND_URL, call, coach_token, last_message_to, messages_to, seed_client, seed_coach,
    test_state,
};
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
    coach: String,
    client: String,
    client_id: Uuid,
    rows: Uuid,
}

async fn setup(db: PgPool) -> Setup {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let rows = sqlx::query_scalar!(
        "INSERT INTO exercises (coach_id, name) VALUES ($1, 'Тяга гантелі в нахилі') RETURNING id",
        coach_id,
    )
    .fetch_one(&db)
    .await
    .unwrap();
    let state = test_state(db);
    Setup {
        app: coaching_backend::router(state.clone()),
        coach: coach_token(&state, coach_id),
        client: state
            .jwt
            .issue(Role::Client, client_id, MINI_APP_TOKEN_TTL)
            .unwrap(),
        state,
        client_id,
        rows,
    }
}

/// Dasha builds a dumbbell-row workout (36 × 12, 36 × 8-10, 28 × 15) on `date`
/// and, unless `publish` is false, publishes it. Returns the workout id.
async fn workout(s: &Setup, date: &str, publish: bool) -> String {
    let (_, created) = call(
        &s.app,
        "POST",
        "/coach/workouts",
        Some(&s.coach),
        Some(json!({ "client_id": s.client_id, "date": date })),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_owned();
    let set = |kg: f64, min: i32, max: i32| json!({ "id": Uuid::new_v4(), "kg": kg, "reps_min": min, "reps_max": max });
    let body = json!({
        "title": "Спина",
        "date": date,
        "exercises": [{
            "id": Uuid::new_v4(), "exercise_id": s.rows, "per_side_label": "на кожну руку",
            "note": null, "sets": [set(36.0, 12, 12), set(36.0, 8, 10), set(28.0, 15, 15)],
        }],
    });
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/coach/workouts/{id}"))
        .header(header::AUTHORIZATION, format!("Bearer {}", s.coach))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, "1")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = s.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response.into_body().collect().await.unwrap();
    if publish {
        let (status, _) = call(
            &s.app,
            "POST",
            &format!("/coach/workouts/{id}/publish"),
            Some(&s.coach),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }
    id
}

async fn open(s: &Setup, id: &str) -> Value {
    let (status, workout) = call(
        &s.app,
        "GET",
        &format!("/workouts/{id}"),
        Some(&s.client),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{workout}");
    workout
}

async fn log(
    s: &Setup,
    set: &Value,
    kg: Option<f64>,
    reps: i32,
    completed: bool,
    at: &str,
) -> StatusCode {
    call(
        &s.app,
        "PUT",
        &format!("/sets/{}/result", set["id"].as_str().unwrap()),
        Some(&s.client),
        Some(json!({ "actual_kg": kg, "actual_reps": reps, "completed": completed, "client_updated_at": at })),
    )
    .await
    .0
}

#[sqlx::test]
async fn clients_see_only_their_published_workouts(db: PgPool) {
    let s = setup(db.clone()).await;
    let draft = workout(&s, "2026-10-08", false).await;
    let later = workout(&s, "2026-10-13", true).await;
    let today = workout(&s, "2026-10-06", true).await;

    let (status, list) = call(&s.app, "GET", "/me/workouts", Some(&s.client), None).await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["id"].clone())
        .collect();
    assert_eq!(ids, [json!(today), json!(later)], "published only, by date");
    assert_eq!(list[0]["set_count"], 3);
    assert_eq!(list[0]["done_set_count"], 0);

    let (status, _) = call(
        &s.app,
        "GET",
        &format!("/workouts/{draft}"),
        Some(&s.client),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "drafts stay hidden");

    // Another client, and the coach's token, get nothing here.
    let coach_id = sqlx::query_scalar!("SELECT coach_id FROM clients WHERE id = $1", s.client_id)
        .fetch_one(&db)
        .await
        .unwrap();
    let other = seed_client(&db, coach_id, 888).await;
    let other_token = s
        .state
        .jwt
        .issue(Role::Client, other, MINI_APP_TOKEN_TTL)
        .unwrap();
    let (status, _) = call(
        &s.app,
        "GET",
        &format!("/workouts/{today}"),
        Some(&other_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(&s.app, "GET", "/me/workouts", Some(&s.coach), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // The publish message opens the workout itself.
    let message = last_message_to(&s.state, CLIENT_TG);
    assert!(matches!(
        &message.keyboard[0][0],
        Button::WebApp { url, .. } if *url == format!("{FRONTEND_URL}/app/workouts/{today}")
    ));
}

#[sqlx::test]
async fn sets_are_logged_and_late_offline_writes_do_not_win(db: PgPool) {
    let s = setup(db).await;
    let id = workout(&s, "2026-10-06", true).await;
    let workout = open(&s, &id).await;
    let sets = workout["exercises"][0]["sets"].as_array().unwrap().clone();
    assert_eq!(workout["exercises"][0]["per_side_label"], "на кожну руку");
    assert_eq!(sets[1]["target_reps_min"], 8);
    assert_eq!(sets[1]["completed"], false);

    // ✓ as planned, then the second set with fewer reps.
    assert_eq!(
        log(&s, &sets[0], Some(36.0), 12, true, "2026-10-06T10:00:00Z").await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        log(&s, &sets[1], Some(36.0), 7, true, "2026-10-06T10:05:00Z").await,
        StatusCode::NO_CONTENT
    );
    // Repeating a write is harmless.
    assert_eq!(
        log(&s, &sets[1], Some(36.0), 7, true, "2026-10-06T10:05:00Z").await,
        StatusCode::NO_CONTENT
    );
    // An older change that waited offline does not undo the newer one.
    assert_eq!(
        log(&s, &sets[1], Some(36.0), 10, false, "2026-10-06T10:01:00Z").await,
        StatusCode::NO_CONTENT
    );

    let logged = open(&s, &id).await;
    let logged = &logged["exercises"][0]["sets"];
    assert_eq!(
        (
            logged[0]["completed"].clone(),
            logged[0]["actual_reps"].clone()
        ),
        (json!(true), json!(12))
    );
    assert_eq!(
        (
            logged[1]["completed"].clone(),
            logged[1]["actual_reps"].clone()
        ),
        (json!(true), json!(7))
    );
    assert_eq!(logged[2]["completed"], false);

    // Un-ticking with a newer change works.
    log(&s, &sets[0], Some(36.0), 12, false, "2026-10-06T10:10:00Z").await;
    assert_eq!(
        open(&s, &id).await["exercises"][0]["sets"][0]["completed"],
        false
    );

    assert_eq!(
        log(&s, &sets[2], Some(-5.0), 15, true, "2026-10-06T10:11:00Z").await,
        StatusCode::BAD_REQUEST
    );
    let foreign = json!({ "id": Uuid::new_v4() });
    assert_eq!(
        log(&s, &foreign, None, 1, true, "2026-10-06T10:12:00Z").await,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test]
async fn last_time_shows_what_was_actually_done(db: PgPool) {
    let s = setup(db).await;
    let first = workout(&s, "2026-10-06", true).await;
    let sets = open(&s, &first).await["exercises"][0]["sets"]
        .as_array()
        .unwrap()
        .clone();
    log(&s, &sets[0], Some(34.0), 12, true, "2026-10-06T10:00:00Z").await;
    log(&s, &sets[1], Some(34.0), 9, true, "2026-10-06T10:05:00Z").await;
    // Typed but never ticked: not part of "last time".
    log(&s, &sets[2], Some(26.0), 15, false, "2026-10-06T10:10:00Z").await;

    let next = workout(&s, "2026-10-13", true).await;
    let opened = open(&s, &next).await;
    assert_eq!(
        opened["exercises"][0]["last_time"],
        json!([{ "kg": 34.0, "reps": 12 }, { "kg": 34.0, "reps": 9 }])
    );
    assert!(
        open(&s, &first).await["exercises"][0]["last_time"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    // Dasha's builder shows the same.
    let (_, built) = call(
        &s.app,
        "GET",
        &format!("/coach/workouts/{next}"),
        Some(&s.coach),
        None,
    )
    .await;
    assert_eq!(built["exercises"][0]["last_time"][1]["reps"], 9);
}

#[sqlx::test]
async fn finishing_sends_dasha_one_report(db: PgPool) {
    let s = setup(db).await;
    let id = workout(&s, "2026-10-06", true).await;
    let sets = open(&s, &id).await["exercises"][0]["sets"]
        .as_array()
        .unwrap()
        .clone();
    log(&s, &sets[0], Some(36.0), 12, true, "2026-10-06T10:00:00Z").await;
    log(&s, &sets[1], Some(36.0), 7, true, "2026-10-06T10:05:00Z").await;

    let finish = format!("/workouts/{id}/finish");
    let report =
        json!({ "effort": "hard", "comment": " Другий підхід пішов важко. ", "duration_min": 52 });
    let (status, _) = call(&s.app, "POST", &finish, Some(&s.client), Some(report)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let done = open(&s, &id).await;
    assert_eq!(done["status"], "done");
    assert_eq!(done["report"]["effort"], "hard");
    assert_eq!(done["report"]["comment"], "Другий підхід пішов важко.");
    assert_eq!(done["report"]["duration_min"], 52);

    let message = last_message_to(&s.state, COACH_TG);
    assert_eq!(
        message.text,
        "Максим К.: звіт про «Спина», вт, 6 жовтня.\n2 з 3 підходів · 1 інакше, ніж у плані · важко\n\n«Другий підхід пішов важко.»"
    );
    assert!(matches!(
        &message.keyboard[0][0],
        Button::WebApp { url, .. } if *url == format!("{FRONTEND_URL}/app/clients/{}", s.client_id)
    ));

    // Sending the report again (say, from the offline queue) updates it quietly.
    let again = json!({ "effort": "ok", "comment": "" });
    let (status, _) = call(&s.app, "POST", &finish, Some(&s.client), Some(again)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(open(&s, &id).await["report"]["effort"], "ok");
    let reports_to_dasha = messages_to(&s.state, COACH_TG)
        .into_iter()
        .filter(|message| message.text.contains("звіт"))
        .count();
    assert_eq!(reports_to_dasha, 1);

    // Results that arrive after the report still count.
    log(&s, &sets[2], Some(28.0), 15, true, "2026-10-06T10:20:00Z").await;
    assert_eq!(
        open(&s, &id).await["exercises"][0]["sets"][2]["completed"],
        true
    );
}
