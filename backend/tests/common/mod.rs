//! Helpers shared by the route tests. Each test file uses a different subset.
#![allow(dead_code)]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::Utc;
use coaching_backend::{
    auth::{Role, jwt::BROWSER_TOKEN_TTL},
    config::Config,
    state::AppState,
    telegram::{OutgoingMessage, Sent, TelegramClient},
};
use hmac::{Hmac, KeyInit, Mac};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::Sha256;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub const BOT_TOKEN: &str = "123456:TEST-bot-token";
pub const BOT_USERNAME: &str = "dasha_test_bot";
pub const WEBHOOK_SECRET: &str = "test-webhook-secret-123";
pub const FRONTEND_URL: &str = "https://app.example.com";

pub fn test_config() -> Config {
    Config {
        database_url: String::new(),
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        bot_token: BOT_TOKEN.to_owned(),
        bot_username: BOT_USERNAME.to_owned(),
        webhook_secret: WEBHOOK_SECRET.to_owned(),
        telegram_webhook_url: None,
        coach_telegram_id: None,
        jwt_secret: "test-secret-that-is-long-enough-for-hs256".to_owned(),
        frontend_url: FRONTEND_URL.to_owned(),
        frontend_origin: header::HeaderValue::from_static(FRONTEND_URL),
        bunny: None,
        client_videos: None,
        storage: None,
    }
}

/// App state whose Telegram client records messages instead of sending them.
pub fn test_state(db: PgPool) -> AppState {
    AppState::new(db, test_config(), TelegramClient::recording())
}

pub fn coach_token(state: &AppState, coach_id: Uuid) -> String {
    state
        .jwt
        .issue(Role::Coach, coach_id, BROWSER_TOKEN_TTL)
        .unwrap()
}

/// Builds Mini App `initData` signed the way Telegram signs it.
pub fn signed_init_data(telegram_id: i64, bot_token: &str) -> String {
    launch(telegram_id, bot_token, None, false)
}

/// `initData` for an app opened by a link with `start_param`, and with or
/// without the user's permission for the bot to message them.
pub fn launch(
    telegram_id: i64,
    bot_token: &str,
    start_param: Option<&str>,
    allows_write_to_pm: bool,
) -> String {
    let user = json!({
        "id": telegram_id, "first_name": "Максим", "allows_write_to_pm": allows_write_to_pm,
    })
    .to_string();
    let mut fields = vec![
        ("auth_date".to_owned(), Utc::now().timestamp().to_string()),
        ("user".to_owned(), user),
    ];
    if let Some(param) = start_param {
        fields.push(("start_param".to_owned(), param.to_owned()));
    }
    fields.sort();
    let check = fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut secret = Hmac::<Sha256>::new_from_slice(b"WebAppData").unwrap();
    secret.update(bot_token.as_bytes());
    let secret = secret.finalize().into_bytes();
    let mut mac = Hmac::<Sha256>::new_from_slice(&secret).unwrap();
    mac.update(check.as_bytes());
    let hash = hex::encode(mac.finalize().into_bytes());

    let mut query = form_urlencoded::Serializer::new(String::new());
    for (key, value) in &fields {
        query.append_pair(key, value);
    }
    query.append_pair("hash", &hash).finish()
}

pub async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = match body {
        Some(body) => request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    }
    .unwrap();
    send(app, request).await
}

/// Posts an update to the bot webhook with the given secret header, if any.
pub async fn webhook_with_secret(app: &Router, secret: Option<&str>, body: &str) -> StatusCode {
    let mut request = Request::builder()
        .method("POST")
        .uri("/telegram/webhook")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(secret) = secret {
        request = request.header("X-Telegram-Bot-Api-Secret-Token", secret);
    }
    send(app, request.body(Body::from(body.to_owned())).unwrap())
        .await
        .0
}

pub async fn webhook(app: &Router, update: Value) -> StatusCode {
    webhook_with_secret(app, Some(WEBHOOK_SECRET), &update.to_string()).await
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

/// A private-chat text message from `telegram_id`, as Telegram sends it.
pub fn text_update(telegram_id: i64, text: &str) -> Value {
    json!({
        "update_id": 1,
        "message": {
            "message_id": 10,
            "date": 0,
            "chat": { "id": telegram_id, "type": "private" },
            "from": { "id": telegram_id, "is_bot": false, "first_name": "Максим" },
            "text": text,
        },
    })
}

/// `telegram_id` pressed an inline button carrying `data` on message 42.
pub fn button_update(telegram_id: i64, data: &str) -> Value {
    json!({
        "update_id": 2,
        "callback_query": {
            "id": "callback-1",
            "from": { "id": telegram_id, "is_bot": false, "first_name": "Даша" },
            "chat_instance": "1",
            "data": data,
            "message": {
                "message_id": 42,
                "date": 0,
                "chat": { "id": telegram_id, "type": "private" },
            },
        },
    })
}

pub async fn seed_coach(db: &PgPool, telegram_id: i64) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO coaches (telegram_id, name) VALUES ($1, 'Даша') RETURNING id",
        telegram_id,
    )
    .fetch_one(db)
    .await
    .unwrap()
}

pub async fn seed_client(db: &PgPool, coach_id: Uuid, telegram_id: i64) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO clients (coach_id, name, telegram_id, bot_allowed_at)
         VALUES ($1, 'Максим К.', $2, now()) RETURNING id",
        coach_id,
        telegram_id,
    )
    .fetch_one(db)
    .await
    .unwrap()
}

/// Messages the bot sent to one chat, oldest first.
pub fn messages_to(state: &AppState, chat_id: i64) -> Vec<OutgoingMessage> {
    state
        .telegram
        .sent()
        .into_iter()
        .filter_map(|sent| match sent {
            Sent::Message(message) if message.chat_id == chat_id => Some(message),
            _ => None,
        })
        .collect()
}

pub fn last_message_to(state: &AppState, chat_id: i64) -> OutgoingMessage {
    messages_to(state, chat_id)
        .pop()
        .unwrap_or_else(|| panic!("no message to {chat_id}"))
}
