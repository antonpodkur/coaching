//! Joining through an app link (`t.me/<bot>?startapp=inv_<code>`): the Mini App
//! opens straight away and the client joins at sign-in, without the bot's Start.
//! The bot writes to them only once they allow it, starting with a pinned welcome.

mod common;

use axum::{Router, http::StatusCode};
use coaching_backend::{
    notify,
    state::AppState,
    telegram::{Button, Sent},
};
use common::{
    BOT_TOKEN, FRONTEND_URL, call, coach_token, last_message_to, launch, messages_to, seed_coach,
    test_state, text_update, webhook,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const COACH_TG: i64 = 555_000_222;
const CLIENT_TG: i64 = 777_000_111;
const STRANGER_TG: i64 = 999_000_333;

struct Setup {
    app: Router,
    state: AppState,
    coach: String,
}

async fn setup(db: PgPool) -> Setup {
    let coach_id = seed_coach(&db, COACH_TG).await;
    let state = test_state(db);
    Setup {
        app: coaching_backend::router(state.clone()),
        coach: coach_token(&state, coach_id),
        state,
    }
}

/// Dasha adds a client; returns the created client and their invite.
async fn invite(s: &Setup, name: &str) -> Value {
    let (status, created) = call(
        &s.app,
        "POST",
        "/coach/clients",
        Some(&s.coach),
        Some(json!({ "name": name })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    created
}

/// The `start_param` an invite link carries: `inv_<code>`.
fn start_param(created: &Value) -> String {
    let url = created["invite"]["url"].as_str().unwrap();
    url.split_once("?startapp=").unwrap().1.to_owned()
}

async fn open_app(
    s: &Setup,
    telegram_id: i64,
    start_param: Option<&str>,
    allowed: bool,
) -> (StatusCode, Value) {
    call(
        &s.app,
        "POST",
        "/auth/telegram-webapp",
        None,
        Some(json!({ "init_data": launch(telegram_id, BOT_TOKEN, start_param, allowed) })),
    )
    .await
}

fn pinned(state: &AppState, chat_id: i64) -> usize {
    state
        .telegram
        .sent()
        .iter()
        .filter(|sent| matches!(sent, Sent::Pinned { chat_id: id, .. } if *id == chat_id))
        .count()
}

#[sqlx::test]
async fn an_app_link_joins_and_the_bot_waits_for_permission(db: PgPool) {
    let s = setup(db).await;
    let created = invite(&s, "Максим К.").await;
    let client_id = created["client"]["id"].as_str().unwrap().to_owned();
    let param = start_param(&created);

    // Tapping the link opens the app: joined, with no Start in the bot.
    let (status, session) = open_app(&s, CLIENT_TG, Some(&param), false).await;
    assert_eq!(status, StatusCode::OK, "{session}");
    assert_eq!(session["role"], "client");
    assert_eq!(session["client"]["id"], client_id);
    assert_eq!(session["client"]["bot_allowed"], false);

    // Dasha hears about it, with a button to the client's page.
    let note = last_message_to(&s.state, COACH_TG);
    assert_eq!(note.text, "Максим К. тепер у застосунку.");
    assert!(matches!(
        &note.keyboard[0][0],
        Button::WebApp { url, .. } if *url == format!("{FRONTEND_URL}/app/clients/{client_id}")
    ));
    // The bot may not write to the client yet.
    assert!(messages_to(&s.state, CLIENT_TG).is_empty());

    // A workout published meanwhile is in the app, but no message goes out.
    let workout_id: Uuid = sqlx::query_scalar(
        "INSERT INTO workouts (coach_id, client_id, title, date, status, published_at)
         SELECT coach_id, id, 'Спина', '2026-10-06', 'published', now() FROM clients
         WHERE id = $1::uuid RETURNING id",
    )
    .bind(&client_id)
    .fetch_one(&s.state.db)
    .await
    .unwrap();
    assert!(
        !notify::workout_published(&s.state, workout_id)
            .await
            .unwrap()
    );
    assert!(messages_to(&s.state, CLIENT_TG).is_empty());

    // The client allows messages in the app: the pinned welcome arrives, once.
    let token = session["token"].as_str().unwrap();
    let (status, _) = call(&s.app, "POST", "/me/bot-allowed", Some(token), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let welcome = last_message_to(&s.state, CLIENT_TG);
    assert!(welcome.text.starts_with("Вітаю!"), "{}", welcome.text);
    assert!(matches!(
        &welcome.keyboard[0][0],
        Button::WebApp { url, .. } if *url == format!("{FRONTEND_URL}/app")
    ));
    assert_eq!(pinned(&s.state, CLIENT_TG), 1);
    call(&s.app, "POST", "/me/bot-allowed", Some(token), None).await;
    assert_eq!(messages_to(&s.state, CLIENT_TG).len(), 1, "welcomed once");

    let (_, me) = call(&s.app, "GET", "/me", Some(token), None).await;
    assert_eq!(me["bot_allowed"], true);

    // Opening the same link again just signs in.
    let (status, again) = open_app(&s, CLIENT_TG, Some(&param), false).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(again["client"]["id"], client_id);
}

#[sqlx::test]
async fn permission_given_in_telegrams_dialog_brings_the_welcome_at_once(db: PgPool) {
    let s = setup(db).await;
    // Dasha opened the app once, so her username is known for "Написати Даші".
    sqlx::query("UPDATE coaches SET username = 'daria_coach'")
        .execute(&s.state.db)
        .await
        .unwrap();
    let created = invite(&s, "Максим К.").await;

    let (status, session) = open_app(&s, CLIENT_TG, Some(&start_param(&created)), true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(session["client"]["bot_allowed"], true);

    let welcome = last_message_to(&s.state, CLIENT_TG);
    assert!(welcome.text.starts_with("Вітаю, Максим!"));
    assert!(matches!(
        &welcome.keyboard[1][0],
        Button::Url { url, .. } if url == "https://t.me/daria_coach"
    ));
    assert_eq!(pinned(&s.state, CLIENT_TG), 1);
}

#[sqlx::test]
async fn used_and_foreign_links_are_explained(db: PgPool) {
    let s = setup(db).await;
    let first = invite(&s, "Максим К.").await;
    let param = start_param(&first);
    open_app(&s, CLIENT_TG, Some(&param), false).await;

    // Someone else tapping the same, now used, link.
    let (status, body) = open_app(&s, STRANGER_TG, Some(&param), false).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "invite_invalid");
    // Without any link it is the usual "ask for an invite".
    let (_, body) = open_app(&s, STRANGER_TG, None, false).await;
    assert_eq!(body["error"], "not_invited");

    // A client's account cannot take over another client's invite.
    let second = invite(&s, "Олена К.").await;
    let (status, body) = open_app(&s, CLIENT_TG, Some(&start_param(&second)), false).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "linked_elsewhere");
}

#[sqlx::test]
async fn dasha_checking_her_own_link_does_not_use_it_up(db: PgPool) {
    let s = setup(db).await;
    let created = invite(&s, "Максим К.").await;
    let param = start_param(&created);

    let (_, session) = open_app(&s, COACH_TG, Some(&param), false).await;
    assert_eq!(session["role"], "coach");

    let (status, session) = open_app(&s, CLIENT_TG, Some(&param), false).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(session["role"], "client");
}

#[sqlx::test]
async fn the_invite_comes_as_a_card_to_share(db: PgPool) {
    let s = setup(db).await;
    let created = invite(&s, "Максим К.").await;
    let url = created["invite"]["url"].as_str().unwrap();
    assert!(url.starts_with("https://t.me/dasha_test_bot?startapp=inv_"));
    assert!(created["invite"]["prepared_message_id"].is_string());

    // Prepared for Dasha's account, with a button that opens the app link.
    let card = s
        .state
        .telegram
        .sent()
        .into_iter()
        .find_map(|sent| match sent {
            Sent::Prepared {
                user_id, button, ..
            } => Some((user_id, button)),
            _ => None,
        })
        .unwrap();
    assert_eq!(card.0, COACH_TG);
    assert!(matches!(card.1, Button::Url { url: link, .. } if link == url));

    // Without Telegram there is no card, but the link still works.
    s.state.telegram.set_failing(true);
    let created = invite(&s, "Олена К.").await;
    assert!(created["invite"]["prepared_message_id"].is_null());
    assert!(created["invite"]["url"].is_string());
}

#[sqlx::test]
async fn a_client_who_writes_to_the_bot_lets_it_write_back(db: PgPool) {
    let s = setup(db).await;
    let created = invite(&s, "Максим К.").await;
    open_app(&s, CLIENT_TG, Some(&start_param(&created)), false).await;
    assert!(messages_to(&s.state, CLIENT_TG).is_empty());

    // They open the chat and press Start: the welcome, pinned.
    webhook(&s.app, text_update(CLIENT_TG, "/start")).await;
    assert!(
        last_message_to(&s.state, CLIENT_TG)
            .text
            .starts_with("Вітаю, Максим!")
    );
    assert_eq!(pinned(&s.state, CLIENT_TG), 1);

    // Anything typed later gets a pointer to the app and to Dasha.
    webhook(&s.app, text_update(CLIENT_TG, "Привіт, а можна питання?")).await;
    let reply = last_message_to(&s.state, CLIENT_TG);
    assert!(reply.text.contains("пиши Даші особисто"));
    assert!(matches!(&reply.keyboard[0][0], Button::WebApp { .. }));
    assert_eq!(pinned(&s.state, CLIENT_TG), 1);
}
