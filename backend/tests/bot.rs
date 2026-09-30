//! The bot end to end: invites, coach sign-in and the webhook, with a Telegram
//! client that records messages instead of sending them.

mod common;

use axum::http::StatusCode;
use coaching_backend::telegram::{Button, Sent};
use common::{
    BOT_TOKEN, FRONTEND_URL, WEBHOOK_SECRET, button_update, call, coach_token, last_message_to,
    messages_to, seed_client, seed_coach, signed_init_data, test_state, text_update, webhook,
    webhook_with_secret,
};
use serde_json::{Value, json};
use sqlx::PgPool;

const COACH_TG: i64 = 555_000_222;
const CLIENT_TG: i64 = 777_000_111;
const STRANGER_TG: i64 = 999_000_333;

/// The `/start` payload inside a `t.me/<bot>?start=<payload>` link.
fn start_payload(url: &str) -> String {
    url.split_once("?start=").unwrap().1.to_owned()
}

fn web_app_url(message: &coaching_backend::telegram::OutgoingMessage) -> Option<String> {
    message
        .keyboard
        .iter()
        .flatten()
        .find_map(|button| match button {
            Button::WebApp { url, .. } => Some(url.clone()),
            _ => None,
        })
}

fn callback_data(message: &coaching_backend::telegram::OutgoingMessage) -> Vec<String> {
    message
        .keyboard
        .iter()
        .flatten()
        .filter_map(|button| match button {
            Button::Callback { data, .. } => Some(data.clone()),
            _ => None,
        })
        .collect()
}

#[sqlx::test]
async fn an_invite_links_the_client_and_opens_the_app(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let state = test_state(db);
    let token = coach_token(&state, coach_id);
    let app = coaching_backend::router(state.clone());

    let (status, created) = call(
        &app,
        "POST",
        "/coach/clients",
        Some(&token),
        Some(json!({ "name": "  Максим К.  " })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["client"]["name"], "Максим К.");
    assert_eq!(created["client"]["joined"], false);
    let url = created["invite"]["url"].as_str().unwrap();
    assert!(url.starts_with("https://t.me/dasha_test_bot?start=inv_"));
    let start = format!("/start {}", start_payload(url));

    assert_eq!(
        webhook(&app, text_update(CLIENT_TG, &start)).await,
        StatusCode::OK
    );
    let welcome = last_message_to(&state, CLIENT_TG);
    assert!(welcome.text.starts_with("Вітаю, Максим!"));
    assert_eq!(web_app_url(&welcome), Some(format!("{FRONTEND_URL}/app")));
    assert!(last_message_to(&state, COACH_TG).text.contains("Максим К."));

    let (_, clients) = call(&app, "GET", "/coach/clients", Some(&token), None).await;
    assert_eq!(clients[0]["joined"], true);
    assert!(clients[0]["invite_expires_at"].is_null(), "invite is spent");

    // The client can now sign in to the Mini App.
    let (status, _) = call(
        &app,
        "POST",
        "/auth/telegram-webapp",
        None,
        Some(json!({ "init_data": signed_init_data(CLIENT_TG, BOT_TOKEN) })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // The same link is useless to anyone else...
    webhook(&app, text_update(STRANGER_TG, &start)).await;
    assert!(
        last_message_to(&state, STRANGER_TG)
            .text
            .contains("недійсне")
    );
    // ...and a repeat tap by the client just offers the app again.
    webhook(&app, text_update(CLIENT_TG, &start)).await;
    let again = last_message_to(&state, CLIENT_TG);
    assert!(again.text.contains("вже в застосунку"));
    assert!(web_app_url(&again).is_some());
}

#[sqlx::test]
async fn invites_expire_can_be_replaced_and_stay_with_their_coach(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let other_coach_id = seed_coach(&db, 111).await;
    let state = test_state(db.clone());
    let token = coach_token(&state, coach_id);
    let app = coaching_backend::router(state.clone());

    let (_, created) = call(
        &app,
        "POST",
        "/coach/clients",
        Some(&token),
        Some(json!({ "name": "Олена" })),
    )
    .await;
    let client_id = created["client"]["id"].as_str().unwrap().to_owned();
    let first_link = created["invite"]["url"].as_str().unwrap().to_owned();

    // A new link replaces the old one.
    let (status, second) = call(
        &app,
        "POST",
        &format!("/coach/clients/{client_id}/invite"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    webhook(
        &app,
        text_update(CLIENT_TG, &format!("/start {}", start_payload(&first_link))),
    )
    .await;
    assert!(last_message_to(&state, CLIENT_TG).text.contains("недійсне"));

    // An expired link does not work either.
    sqlx::query("UPDATE clients SET invite_expires_at = now() - interval '1 minute'")
        .execute(&db)
        .await
        .unwrap();
    let second_link = second["url"].as_str().unwrap();
    webhook(
        &app,
        text_update(CLIENT_TG, &format!("/start {}", start_payload(second_link))),
    )
    .await;
    assert!(last_message_to(&state, CLIENT_TG).text.contains("недійсне"));

    // Another coach cannot touch this client.
    let other_token = coach_token(&state, other_coach_id);
    let (status, _) = call(
        &app,
        "POST",
        &format!("/coach/clients/{client_id}/invite"),
        Some(&other_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, theirs) = call(&app, "GET", "/coach/clients", Some(&other_token), None).await;
    assert_eq!(theirs, json!([]));

    let (status, body) = call(
        &app,
        "POST",
        "/coach/clients",
        Some(&token),
        Some(json!({ "name": "   " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_name");
}

#[sqlx::test]
async fn one_telegram_account_cannot_join_two_profiles(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    seed_client(&db, coach_id, CLIENT_TG).await;
    let state = test_state(db);
    let token = coach_token(&state, coach_id);
    let app = coaching_backend::router(state.clone());

    let (_, created) = call(
        &app,
        "POST",
        "/coach/clients",
        Some(&token),
        Some(json!({ "name": "Друга" })),
    )
    .await;
    let link = created["invite"]["url"].as_str().unwrap();
    webhook(
        &app,
        text_update(CLIENT_TG, &format!("/start {}", start_payload(link))),
    )
    .await;

    assert!(
        last_message_to(&state, CLIENT_TG)
            .text
            .contains("іншого профілю")
    );
    let (_, clients) = call(&app, "GET", "/coach/clients", Some(&token), None).await;
    let second = clients
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "Друга")
        .unwrap();
    assert_eq!(second["joined"], false);
}

#[sqlx::test]
async fn the_webhook_requires_telegrams_secret(db: PgPool) {
    seed_coach(&db, COACH_TG).await;
    let state = test_state(db);
    let app = coaching_backend::router(state.clone());
    let update = text_update(COACH_TG, "/start").to_string();

    assert_eq!(
        webhook_with_secret(&app, None, &update).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        webhook_with_secret(&app, Some("wrong-secret-value-123"), &update).await,
        StatusCode::UNAUTHORIZED
    );
    assert!(state.telegram.sent().is_empty());

    // With the secret, even an update the bot cannot read is acknowledged.
    assert_eq!(
        webhook_with_secret(&app, Some(WEBHOOK_SECRET), "not json").await,
        StatusCode::OK
    );
}

#[sqlx::test]
async fn plain_start_greets_by_role(db: PgPool) {
    let coach_id = seed_coach(&db, COACH_TG).await;
    seed_client(&db, coach_id, CLIENT_TG).await;
    let state = test_state(db);
    let app = coaching_backend::router(state.clone());

    for id in [COACH_TG, CLIENT_TG, STRANGER_TG] {
        webhook(&app, text_update(id, "/start")).await;
    }
    let coach = last_message_to(&state, COACH_TG);
    assert!(matches!(
        &coach.keyboard[0][0],
        Button::Url { url, .. } if url == &format!("{FRONTEND_URL}/coach")
    ));
    assert!(web_app_url(&last_message_to(&state, CLIENT_TG)).is_some());
    assert!(
        last_message_to(&state, STRANGER_TG)
            .text
            .contains("запрошення")
    );

    // Group chats are ignored.
    let mut group = text_update(COACH_TG, "/start");
    group["message"]["chat"] = json!({ "id": -100, "type": "group" });
    webhook(&app, group).await;
    assert!(messages_to(&state, -100).is_empty());
}

async fn start_login(app: &axum::Router) -> (String, String, String) {
    let (status, login) = call(app, "POST", "/auth/bot-login", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let bot_url = login["bot_url"].as_str().unwrap();
    assert!(bot_url.starts_with("https://t.me/dasha_test_bot?start=login_"));
    (
        format!("/start {}", start_payload(bot_url)),
        login["poll_secret"].as_str().unwrap().to_owned(),
        login["display_code"].as_str().unwrap().to_owned(),
    )
}

async fn poll(app: &axum::Router, poll_secret: &str) -> Value {
    let (status, body) = call(
        app,
        "POST",
        "/auth/bot-login/poll",
        None,
        Some(json!({ "poll_secret": poll_secret })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    body
}

#[sqlx::test]
async fn the_coach_signs_in_by_confirming_in_the_bot(db: PgPool) {
    seed_coach(&db, COACH_TG).await;
    let state = test_state(db);
    let app = coaching_backend::router(state.clone());

    let (start, poll_secret, display_code) = start_login(&app).await;
    assert_eq!(poll(&app, &poll_secret).await["status"], "pending");

    webhook(&app, text_update(COACH_TG, &start)).await;
    let ask = last_message_to(&state, COACH_TG);
    assert!(ask.text.contains(&display_code), "shows the same code");
    let buttons = callback_data(&ask);
    assert_eq!(buttons.len(), 2);
    assert_eq!(
        poll(&app, &poll_secret).await["status"],
        "pending",
        "opening the link alone approves nothing"
    );

    webhook(&app, button_update(COACH_TG, &buttons[0])).await;
    let sent = state.telegram.sent();
    assert!(sent.contains(&Sent::CallbackAnswer {
        id: "callback-1".into(),
        text: "Вхід підтверджено".into(),
    }));
    assert!(sent.iter().any(|s| matches!(s,
        Sent::Edited { chat_id: COACH_TG, message_id: 42, text } if text.contains("Повернись у браузер"))));

    let approved = poll(&app, &poll_secret).await;
    assert_eq!(approved["status"], "approved");
    assert_eq!(approved["coach"]["name"], "Даша");
    let token = approved["token"].as_str().unwrap();
    let (status, _) = call(&app, "GET", "/coach/clients", Some(token), None).await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(
        poll(&app, &poll_secret).await["status"],
        "expired",
        "the token is handed out once"
    );
}

#[sqlx::test]
async fn bot_sign_in_refuses_strangers_and_honours_cancel(db: PgPool) {
    seed_coach(&db, COACH_TG).await;
    let state = test_state(db.clone());
    let app = coaching_backend::router(state.clone());

    // Someone who is not a coach opens a login link.
    let (start, poll_secret, _) = start_login(&app).await;
    webhook(&app, text_update(STRANGER_TG, &start)).await;
    assert!(
        last_message_to(&state, STRANGER_TG)
            .text
            .contains("не має доступу")
    );

    // The coach opens it; a stranger pressing her Confirm button changes nothing.
    webhook(&app, text_update(COACH_TG, &start)).await;
    let buttons = callback_data(&last_message_to(&state, COACH_TG));
    webhook(&app, button_update(STRANGER_TG, &buttons[0])).await;
    assert_eq!(poll(&app, &poll_secret).await["status"], "pending");

    // She cancels.
    webhook(&app, button_update(COACH_TG, &buttons[1])).await;
    assert_eq!(poll(&app, &poll_secret).await["status"], "cancelled");
    // A cancelled login cannot be confirmed afterwards.
    webhook(&app, button_update(COACH_TG, &buttons[0])).await;
    assert_eq!(poll(&app, &poll_secret).await["status"], "cancelled");

    // An expired login is refused in the bot and reported as expired.
    let (start, poll_secret, _) = start_login(&app).await;
    sqlx::query("UPDATE coach_logins SET expires_at = now() - interval '1 second'")
        .execute(&db)
        .await
        .unwrap();
    webhook(&app, text_update(COACH_TG, &start)).await;
    assert!(
        last_message_to(&state, COACH_TG)
            .text
            .contains("недійсне або застаріло")
    );
    assert_eq!(poll(&app, &poll_secret).await["status"], "expired");
    assert_eq!(poll(&app, "made-up-secret").await["status"], "expired");
}
