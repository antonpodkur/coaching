//! Nutrition targets: Dasha sets them, the client sees them and hears about
//! each change from the bot.

mod common;

use axum::http::StatusCode;
use chrono::Utc;
use coaching_backend::{
    auth::{Role, jwt::MINI_APP_TOKEN_TTL},
    jobs,
    telegram::Button,
};
use common::{FRONTEND_URL, call, coach_token, messages_to, seed_client, seed_coach, test_state};
use serde_json::{Value, json};
use sqlx::PgPool;

const CLIENT_TG: i64 = 777_000_111;

#[sqlx::test]
async fn dasha_sets_targets_and_the_client_hears_of_each_change(db: PgPool) {
    let coach_id = seed_coach(&db, 555_000_222).await;
    let other_coach_id = seed_coach(&db, 555_000_333).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let state = test_state(db);
    let app = coaching_backend::router(state.clone());
    let client = state
        .jwt
        .issue(Role::Client, client_id, MINI_APP_TOKEN_TTL)
        .unwrap();
    let coach = coach_token(&state, coach_id);
    let path = format!("/coach/clients/{client_id}/nutrition");
    let set = async |body: Value| call(&app, "POST", &path, Some(&coach), Some(body)).await;

    let (status, mine) = call(&app, "GET", "/me/nutrition", Some(&client), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(mine["current"].is_null());

    for wrong in [
        json!({ "protein_g": 0, "fat_g": 0, "carbs_g": 0 }),
        json!({ "protein_g": 100, "fat_g": 1500, "carbs_g": 200 }),
    ] {
        let (status, body) = set(wrong).await;
        assert_eq!(
            (status, &body["error"]),
            (StatusCode::BAD_REQUEST, &json!("invalid_grams"))
        );
    }

    let (status, saved) = set(
        json!({ "protein_g": 100, "fat_g": 50, "carbs_g": 200, "note": "  2 л води на день " }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["target"]["kcal"], 1650);
    assert_eq!(saved["target"]["note"], "2 л води на день");
    assert_eq!(saved["client_notified"], true);
    let first = &messages_to(&state, CLIENT_TG)[0];
    assert_eq!(
        first.text,
        "Даша склала тобі норму харчування на день:\nбілки — 100 г\nжири — 50 г\n\
         вуглеводи — 200 г\nРазом близько 1650 ккал.\n\n«2 л води на день»"
    );
    assert!(matches!(
        &first.keyboard[0][0],
        Button::WebApp { url, .. } if *url == format!("{FRONTEND_URL}/app/nutrition")
    ));

    let (_, mine) = call(&app, "GET", "/me/nutrition", Some(&client), None).await;
    assert_eq!(mine["current"]["protein_g"], 100);

    // A change replaces the target, keeps the old one, and is announced once.
    set(json!({ "protein_g": 120, "fat_g": 50, "carbs_g": 180 })).await;
    jobs::tick(&state, Utc::now()).await.unwrap();
    let messages = messages_to(&state, CLIENT_TG);
    assert_eq!(messages.len(), 2);
    assert!(
        messages[1]
            .text
            .starts_with("Даша оновила твою норму харчування на день:")
    );
    assert!(messages[1].text.ends_with("Разом близько 1650 ккал."));
    let (_, history) = call(&app, "GET", &path, Some(&coach), None).await;
    let targets = history["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 2);
    assert_eq!(
        (&targets[0]["protein_g"], &targets[1]["protein_g"]),
        (&json!(120), &json!(100))
    );

    // Only Dasha, not another coach or the client.
    let other = coach_token(&state, other_coach_id);
    assert_eq!(
        call(&app, "GET", &path, Some(&other), None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, "GET", &path, Some(&client), None).await.0,
        StatusCode::FORBIDDEN
    );
}

#[sqlx::test]
async fn without_bot_messages_the_client_still_sees_the_target(db: PgPool) {
    let coach_id = seed_coach(&db, 555_000_222).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    sqlx::query("UPDATE clients SET bot_allowed_at = NULL WHERE id = $1")
        .bind(client_id)
        .execute(&db)
        .await
        .unwrap();
    let state = test_state(db);
    let app = coaching_backend::router(state.clone());
    let coach = coach_token(&state, coach_id);
    let path = format!("/coach/clients/{client_id}/nutrition");
    let body = json!({ "protein_g": 90, "fat_g": 60, "carbs_g": 150 });
    let (status, saved) = call(&app, "POST", &path, Some(&coach), Some(body)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["client_notified"], false);
    assert!(messages_to(&state, CLIENT_TG).is_empty());

    let client = state
        .jwt
        .issue(Role::Client, client_id, MINI_APP_TOKEN_TTL)
        .unwrap();
    let (_, mine) = call(&app, "GET", "/me/nutrition", Some(&client), None).await;
    assert_eq!(mine["current"]["kcal"], 4 * 90 + 9 * 60 + 4 * 150);
}
