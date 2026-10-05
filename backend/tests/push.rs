//! Web push end to end: phones subscribe, notifications reach them encrypted
//! for that phone alone, next to the bot's messages, and only once.

mod common;

use axum::http::StatusCode;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use coaching_backend::{
    auth::{Role, jwt::BROWSER_TOKEN_TTL},
    notify::{self, Kind},
    push::{self, PushClient, Pushed},
    state::AppState,
};
use common::{FRONTEND_URL, call, coach_token, messages_to, seed_client, seed_coach, test_state};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;
use web_push_native::p256::{self, elliptic_curve::sec1::ToEncodedPoint};

const COACH_TG: i64 = 555_000_222;
const CLIENT_TG: i64 = 777_000_111;

fn with_push(db: PgPool) -> AppState {
    let (private, _) = push::new_keys();
    test_state(db).with_push(Some(
        PushClient::recording(&private, FRONTEND_URL.to_owned()).unwrap(),
    ))
}

/// A phone's browser: its keys, and the subscription it hands the app.
struct Phone {
    secret: p256::SecretKey,
    auth: [u8; 16],
    endpoint: String,
}

impl Phone {
    fn new(seed: u8, endpoint: &str) -> Self {
        Self {
            secret: p256::SecretKey::from_slice(&[seed; 32]).unwrap(),
            auth: [seed.wrapping_add(1); 16],
            endpoint: endpoint.to_owned(),
        }
    }

    fn subscription(&self) -> Value {
        let point = self.secret.public_key().to_encoded_point(false);
        json!({
            "endpoint": self.endpoint,
            "keys": {
                "p256dh": URL_SAFE_NO_PAD.encode(point.as_bytes()),
                "auth": URL_SAFE_NO_PAD.encode(self.auth),
            },
        })
    }

    /// What the phone shows for a push, after decrypting it.
    fn read(&self, pushed: &Pushed) -> Value {
        assert_eq!(pushed.endpoint, self.endpoint);
        let plain = web_push_native::decrypt(
            pushed.body.clone(),
            &self.secret,
            &web_push_native::Auth::from(self.auth),
        )
        .unwrap();
        serde_json::from_slice(&plain).unwrap()
    }
}

async fn subscribe(app: &axum::Router, token: &str, phone: &Phone) -> StatusCode {
    let (status, _) = call(
        app,
        "PUT",
        "/push/subscription",
        Some(token),
        Some(phone.subscription()),
    )
    .await;
    status
}

async fn published_workout(db: &PgPool, client_id: Uuid, title: &str) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO workouts (coach_id, client_id, title, date, status, published_at)
         SELECT coach_id, id, $2, current_date, 'published', now() FROM clients WHERE id = $1
         RETURNING id",
        client_id,
        title,
    )
    .fetch_one(db)
    .await
    .unwrap()
}

fn client_token(state: &AppState, client_id: Uuid) -> String {
    state
        .jwt
        .issue(Role::Client, client_id, BROWSER_TOKEN_TTL)
        .unwrap()
}

#[sqlx::test]
async fn the_key_is_there_only_when_push_is_set_up(db: PgPool) {
    let app = coaching_backend::router(test_state(db.clone()));
    let (status, body) = call(&app, "GET", "/push/key", None, None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], "push_not_configured");

    let state = with_push(db);
    let app = coaching_backend::router(state.clone());
    let (status, body) = call(&app, "GET", "/push/key", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["public_key"],
        state.push.as_ref().unwrap().public_key()
    );
}

#[sqlx::test]
async fn a_new_workout_reaches_the_phone_and_the_bot_once(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let state = with_push(db.clone());
    let app = coaching_backend::router(state.clone());
    let phone = Phone::new(7, "https://fcm.googleapis.com/fcm/send/phone-1");
    let token = client_token(&state, client_id);
    assert_eq!(
        subscribe(&app, &token, &phone).await,
        StatusCode::NO_CONTENT
    );

    // Telegram is down: the phone still gets its push, and the bot's message waits.
    state.telegram.set_failing(true);
    let workout_id = published_workout(&db, client_id, "Спина").await;
    assert!(notify::workout_published(&state, workout_id).await.unwrap());
    let pushed = state.push.as_ref().unwrap().sent();
    assert_eq!(pushed.len(), 1);
    let shown = phone.read(&pushed[0]);
    assert_eq!(shown["title"], "Нове тренування");
    assert!(shown["body"].as_str().unwrap().starts_with("«Спина», "));
    assert_eq!(shown["url"], format!("/app/workouts/{workout_id}"));
    let header = |name: &str| {
        pushed[0]
            .headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    };
    assert_eq!(header("content-encoding").as_deref(), Some("aes128gcm"));
    assert!(header("authorization").unwrap().starts_with("vapid t="));
    assert!(header("ttl").is_some());
    assert!(messages_to(&state, CLIENT_TG).is_empty());

    // The bot's retry goes out; the phone does not buzz a second time.
    state.telegram.set_failing(false);
    let id = notify::queue(
        &state,
        Kind::WorkoutPublished,
        workout_id,
        Utc::now().date_naive(),
    )
    .await
    .unwrap()
    .expect("still unsent");
    sqlx::query!(
        "UPDATE notifications SET last_attempt_at = now() - interval '1 hour' WHERE id = $1",
        id
    )
    .execute(&db)
    .await
    .unwrap();
    assert_eq!(
        notify::deliver(&state, id, Utc::now()).await.unwrap(),
        notify::Delivery::Sent
    );
    assert_eq!(messages_to(&state, CLIENT_TG).len(), 1);
    assert_eq!(state.push.as_ref().unwrap().sent().len(), 1);
}

#[sqlx::test]
async fn push_alone_reaches_a_client_the_bot_cannot(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    sqlx::query!(
        "UPDATE clients SET bot_allowed_at = NULL WHERE id = $1",
        client_id
    )
    .execute(&db)
    .await
    .unwrap();
    let state = with_push(db.clone());
    let app = coaching_backend::router(state.clone());

    // Neither channel yet: nobody to tell.
    let first = published_workout(&db, client_id, "").await;
    assert!(!notify::workout_published(&state, first).await.unwrap());

    let phone = Phone::new(3, "https://web.push.apple.com/phone-2");
    let token = client_token(&state, client_id);
    assert_eq!(
        subscribe(&app, &token, &phone).await,
        StatusCode::NO_CONTENT
    );
    let target = sqlx::query_scalar!(
        "INSERT INTO nutrition_targets (client_id, protein_g, fat_g, carbs_g)
         VALUES ($1, 120, 60, 250) RETURNING id",
        client_id,
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(notify::nutrition_changed(&state, target).await.unwrap());
    let pushed = state.push.as_ref().unwrap().sent();
    assert_eq!(pushed.len(), 1);
    let shown = phone.read(&pushed[0]);
    assert_eq!(shown["title"], "Нова норма харчування");
    assert_eq!(
        shown["body"],
        "Білки 120 г · жири 60 г · вуглеводи 250 г · близько 2020 ккал"
    );
    assert_eq!(shown["url"], "/app/nutrition");
    assert!(
        messages_to(&state, CLIENT_TG).is_empty(),
        "the bot may not write to them"
    );
}

#[sqlx::test]
async fn dasha_gets_her_summary_on_her_phone(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let state = with_push(db.clone());
    let app = coaching_backend::router(state.clone());
    let phone = Phone::new(
        5,
        "https://updates.push.services.mozilla.com/wpush/v2/dasha",
    );
    assert_eq!(
        subscribe(&app, &coach_token(&state, coach_id), &phone).await,
        StatusCode::NO_CONTENT
    );
    // Today's workout, not opened.
    published_workout(&db, client_id, "Ноги").await;

    let today = Utc::now().date_naive();
    let id = notify::queue(&state, Kind::CoachDaily, coach_id, today)
        .await
        .unwrap()
        .unwrap();
    notify::deliver(&state, id, Utc::now()).await.unwrap();
    let pushed = state.push.as_ref().unwrap().sent();
    assert_eq!(pushed.len(), 1);
    let shown = phone.read(&pushed[0]);
    assert_eq!(shown["title"], "Підсумок дня");
    assert_eq!(shown["body"], "Не відкрили тренування: 1");
    assert_eq!(shown["url"], "/app");
    assert_eq!(
        messages_to(&state, COACH_TG).len(),
        1,
        "and the bot's message"
    );
}

#[sqlx::test]
async fn subscriptions_move_with_sign_in_and_stop_when_gone(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let client_id = seed_client(&db, coach_id, CLIENT_TG).await;
    let state = with_push(db.clone());
    let app = coaching_backend::router(state.clone());
    let client = client_token(&state, client_id);
    let coach = coach_token(&state, coach_id);
    let owners = |endpoint: &'static str| {
        let db = db.clone();
        async move {
            sqlx::query!(
                "SELECT client_id, coach_id FROM push_subscriptions WHERE endpoint = $1",
                endpoint
            )
            .fetch_optional(&db)
            .await
            .unwrap()
            .map(|row| (row.client_id, row.coach_id))
        }
    };

    // Only browsers' push services, with real keys.
    let made_up = Phone::new(1, "https://example.com/collect");
    assert_eq!(
        subscribe(&app, &client, &made_up).await,
        StatusCode::BAD_REQUEST
    );
    let mut bad_keys = Phone::new(1, "https://fcm.googleapis.com/fcm/send/x").subscription();
    bad_keys["keys"]["auth"] = json!("c2hvcnQ");
    let (status, body) = call(
        &app,
        "PUT",
        "/push/subscription",
        Some(&client),
        Some(bad_keys),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_subscription");

    // The client signs out on a shared phone and Dasha signs in: it moves to her.
    let shared = Phone::new(2, "https://fcm.googleapis.com/fcm/send/shared");
    assert_eq!(
        subscribe(&app, &client, &shared).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        owners("https://fcm.googleapis.com/fcm/send/shared").await,
        Some((Some(client_id), None))
    );
    assert_eq!(
        subscribe(&app, &coach, &shared).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        owners("https://fcm.googleapis.com/fcm/send/shared").await,
        Some((None, Some(coach_id)))
    );

    // Someone else cannot remove it; she can.
    let remove = json!({ "endpoint": "https://fcm.googleapis.com/fcm/send/shared" });
    let (status, _) = call(
        &app,
        "DELETE",
        "/push/subscription",
        Some(&client),
        Some(remove.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(
        owners("https://fcm.googleapis.com/fcm/send/shared")
            .await
            .is_some()
    );
    call(
        &app,
        "DELETE",
        "/push/subscription",
        Some(&coach),
        Some(remove),
    )
    .await;
    assert!(
        owners("https://fcm.googleapis.com/fcm/send/shared")
            .await
            .is_none()
    );

    // A phone whose app was removed: the push service says so, and it is forgotten.
    let removed = Phone::new(4, "https://fcm.googleapis.com/fcm/send/gone-1");
    assert_eq!(
        subscribe(&app, &client, &removed).await,
        StatusCode::NO_CONTENT
    );
    let workout_id = published_workout(&db, client_id, "").await;
    notify::workout_published(&state, workout_id).await.unwrap();
    assert_eq!(state.push.as_ref().unwrap().sent().len(), 1);
    assert!(
        owners("https://fcm.googleapis.com/fcm/send/gone-1")
            .await
            .is_none()
    );
}
