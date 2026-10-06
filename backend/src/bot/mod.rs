//! What the bot does with incoming updates: invite links, sign-in, and a
//! short answer to anything else. It only talks in private chats.
//!
//! The chat is the client's inbox: new workouts and reminders, each with a
//! button into the app. Training itself happens in the app.

use uuid::Uuid;

use crate::{
    auth::bot_login::{self, Claimed},
    config::Config,
    invites::{self, Accepted},
    state::AppState,
    telegram::{Button, CallbackQuery, Message, OutgoingMessage, TelegramClient, Update, User},
};

/// Label of the menu button that opens the Mini App, for clients and the coach.
const MENU_BUTTON_TEXT: &str = "Відкрити";

/// Points Telegram at this deployment: the webhook (when configured) and the
/// menu button's Mini App URL. Runs on startup. Failures are logged, not fatal:
/// the API still works, and the next start tries again.
pub async fn register(telegram: &TelegramClient, config: &Config) {
    if let Some(url) = &config.telegram_webhook_url {
        match telegram.set_webhook(url, &config.webhook_secret).await {
            Ok(()) => tracing::info!(%url, "Telegram webhook registered"),
            Err(err) => tracing::warn!(error = ?err, "could not register the Telegram webhook"),
        }
    }
    let url = config.mini_app_url();
    match telegram
        .set_default_menu_button(MENU_BUTTON_TEXT, &url)
        .await
    {
        Ok(()) => tracing::info!(%url, "menu button opens the Mini App"),
        Err(err) => tracing::warn!(error = ?err, "could not set the bot's menu button"),
    }
}

pub async fn handle_update(state: &AppState, update: Update) -> anyhow::Result<()> {
    if let Some(message) = update.message {
        handle_message(state, message).await?;
    } else if let Some(query) = update.callback_query {
        handle_callback(state, query).await?;
    }
    Ok(())
}

/// What a `/start` command carries: `t.me/<bot>?start=<payload>`.
#[derive(Debug, PartialEq, Eq)]
enum Start<'a> {
    Invite(&'a str),
    Login(&'a str),
    Plain,
}

fn parse_start(text: &str) -> Option<Start<'_>> {
    let mut words = text.split_whitespace();
    let command = words.next()?;
    if command != "/start" && !command.starts_with("/start@") {
        return None;
    }
    Some(match words.next() {
        Some(payload) => {
            if let Some(code) = payload.strip_prefix(invites::START_PREFIX) {
                Start::Invite(code)
            } else if let Some(code) = payload.strip_prefix(bot_login::START_PREFIX) {
                Start::Login(code)
            } else {
                Start::Plain
            }
        }
        None => Start::Plain,
    })
}

async fn handle_message(state: &AppState, message: Message) -> anyhow::Result<()> {
    let Some(from) = message.from else {
        return Ok(());
    };
    if message.chat.kind != "private" {
        return Ok(());
    }
    let chat_id = message.chat.id;
    let text = message.text.as_deref().unwrap_or_default();

    let reply = match parse_start(text) {
        Some(Start::Invite(code)) => accept_invite(state, &from, chat_id, code).await?,
        Some(Start::Login(code)) => Reply::plain(claim_login(state, &from, chat_id, code).await?),
        Some(Start::Plain) | None => greet(state, &from, chat_id).await?,
    };
    let message_id = state.telegram.send_message(reply.message).await?;
    if reply.pin {
        pin(state, chat_id, message_id).await;
    }
    Ok(())
}

/// The bot's answer, and whether to pin it (the welcome).
struct Reply {
    message: OutgoingMessage,
    pin: bool,
}

impl Reply {
    fn plain(message: OutgoingMessage) -> Self {
        Self {
            message,
            pin: false,
        }
    }
}

/// An invite opened through the bot (`?start=`). Links from before app links
/// existed still arrive this way.
async fn accept_invite(
    state: &AppState,
    from: &User,
    chat_id: i64,
    code: &str,
) -> anyhow::Result<Reply> {
    Ok(match invites::accept(&state.db, code, from.id).await? {
        Accepted::Joined {
            client_id,
            client_name,
            coach_telegram_id,
        } => {
            // Pressing Start lets the bot write to them.
            sqlx::query!(
                "UPDATE clients SET bot_allowed_at = now() WHERE id = $1",
                client_id
            )
            .execute(&state.db)
            .await?;
            announce_join(state, client_id, &client_name, coach_telegram_id).await;
            Reply {
                message: welcome(state, client_id, chat_id, Some(&from.first_name)).await?,
                pin: true,
            }
        }
        Accepted::LinkedElsewhere => Reply::plain(OutgoingMessage::text(chat_id, LINKED_ELSEWHERE)),
        // A repeated update, or an old link tapped again by someone who already joined.
        Accepted::Invalid if is_client(state, from.id).await? => Reply::plain(open_app(
            state,
            chat_id,
            "Ти вже в застосунку. Відкривай тренування кнопкою нижче.".to_owned(),
        )),
        Accepted::Invalid => Reply::plain(OutgoingMessage::text(
            chat_id,
            "Посилання недійсне або застаріло. Попроси нове.",
        )),
    })
}

pub const LINKED_ELSEWHERE: &str = "Цей Telegram-акаунт уже прив’язаний до іншого профілю. \
     Якщо це помилка, напиши тому, хто тебе запросив.";

/// Tells Dasha that a client joined, with a button to their page. Losing this
/// message must not fail the join.
pub async fn announce_join(
    state: &AppState,
    client_id: Uuid,
    client_name: &str,
    coach_telegram_id: i64,
) {
    let note = OutgoingMessage::text(
        coach_telegram_id,
        format!("{client_name} тепер у застосунку."),
    )
    .with_row(vec![Button::WebApp {
        text: "Відкрити клієнта".to_owned(),
        url: format!("{}/clients/{client_id}", state.config.mini_app_url()),
    }]);
    if let Err(err) = state.telegram.send_message(note).await {
        tracing::warn!(error = ?err, "could not tell the coach about a new client");
    }
}

/// Records that the client let the bot write to them and, the first time,
/// sends the pinned welcome. For clients who joined through an app link and
/// allowed messages there. Returns whether the welcome went out now.
pub async fn welcome_client(
    state: &AppState,
    client_id: Uuid,
    first_name: Option<&str>,
) -> anyhow::Result<bool> {
    let Some(chat_id) = sqlx::query_scalar!(
        r#"UPDATE clients SET bot_allowed_at = now()
           WHERE id = $1 AND bot_allowed_at IS NULL AND telegram_id IS NOT NULL
           RETURNING telegram_id AS "telegram_id!""#,
        client_id,
    )
    .fetch_optional(&state.db)
    .await?
    else {
        return Ok(false);
    };
    let message = welcome(state, client_id, chat_id, first_name).await?;
    match state.telegram.send_message(message).await {
        Ok(message_id) => {
            pin(state, chat_id, message_id).await;
            Ok(true)
        }
        Err(err) => {
            // Most likely they had not allowed it after all; the app can ask again.
            sqlx::query!(
                "UPDATE clients SET bot_allowed_at = NULL WHERE id = $1",
                client_id
            )
            .execute(&state.db)
            .await?;
            Err(err)
        }
    }
}

/// What the chat is for, the button into the app, and where to ask the coach.
async fn welcome(
    state: &AppState,
    client_id: Uuid,
    chat_id: i64,
    first_name: Option<&str>,
) -> anyhow::Result<OutgoingMessage> {
    let hello = match first_name {
        Some(name) => format!("Вітаю, {name}!"),
        None => "Вітаю!".to_owned(),
    };
    let text = format!(
        "{hello} Тут з’являтимуться нові тренування й нагадування в день тренування.\n\n\
         Застосунок відкривається кнопкою нижче або «{MENU_BUTTON_TEXT}» біля поля повідомлення. \
         {NOBODY_READS}"
    );
    let message = open_app(state, chat_id, text);
    Ok(with_coach_contact(state, client_id, message).await?)
}

/// The chat with the bot is not where to ask the coach anything.
const NOBODY_READS: &str =
    "Цей чат ніхто не читає: з питаннями щодо тренувань пиши в особисті повідомлення.";

/// Adds a button to the coach's own Telegram, "Даша в Telegram", when her
/// username is known. Her name stays as written: Ukrainian would change it by case.
async fn with_coach_contact(
    state: &AppState,
    client_id: Uuid,
    message: OutgoingMessage,
) -> sqlx::Result<OutgoingMessage> {
    let coach = sqlx::query!(
        "SELECT co.name, co.username FROM clients c JOIN coaches co ON co.id = c.coach_id
         WHERE c.id = $1",
        client_id,
    )
    .fetch_optional(&state.db)
    .await?;
    Ok(match coach {
        Some(coach) => match coach.username {
            Some(username) => message.with_row(vec![Button::Url {
                text: format!("{} в Telegram", coach.name),
                url: format!("https://t.me/{username}"),
            }]),
            None => message,
        },
        None => message,
    })
}

/// Pins the welcome so the app button stays at the top of the chat.
async fn pin(state: &AppState, chat_id: i64, message_id: i64) {
    if let Err(err) = state.telegram.pin_chat_message(chat_id, message_id).await {
        tracing::warn!(error = ?err, "could not pin the welcome");
    }
}

/// A sign-in link from the app outside Telegram, opened in the bot.
async fn claim_login(
    state: &AppState,
    from: &User,
    chat_id: i64,
    code: &str,
) -> anyhow::Result<OutgoingMessage> {
    Ok(match bot_login::claim(&state.db, code, from.id).await? {
        Claimed::Confirm {
            login_id,
            display_code,
        } => {
            // Pressing Start lets the bot write to them: a client who joined
            // through an app link gets the pinned welcome now, above this.
            if let Some(client_id) = client_id(state, from.id).await?
                && let Err(err) = welcome_client(state, client_id, Some(&from.first_name)).await
            {
                tracing::warn!(error = ?err, "could not welcome a client in the bot");
            }
            confirm_login(chat_id, login_id, &display_code)
        }
        Claimed::NoAccount => OutgoingMessage::text(
            chat_id,
            "Цей Telegram-акаунт ще не має доступу. Спершу відкрий посилання-запрошення від \
             тренера, а потім увійди знову.",
        ),
        Claimed::Invalid => OutgoingMessage::text(
            chat_id,
            "Посилання для входу недійсне або застаріло. Спробуй увійти ще раз.",
        ),
    })
}

fn confirm_login(chat_id: i64, login_id: Uuid, display_code: &str) -> OutgoingMessage {
    OutgoingMessage::text(
        chat_id,
        format!(
            "Увійти в застосунок?\n\nКод на екрані має бути {display_code}. \
             Якщо це не твій вхід — натисни «Скасувати»."
        ),
    )
    .with_row(vec![
        Button::Callback {
            text: "Підтвердити".to_owned(),
            data: format!("login_ok:{login_id}"),
        },
        Button::Callback {
            text: "Скасувати".to_owned(),
            data: format!("login_no:{login_id}"),
        },
    ])
}

async fn greet(state: &AppState, from: &User, chat_id: i64) -> anyhow::Result<Reply> {
    if let Some(client_id) = client_id(state, from.id).await? {
        // Writing to the bot lets it write back. Someone who joined through an
        // app link without allowing messages gets the welcome now.
        let newly_allowed = sqlx::query_scalar!(
            "UPDATE clients SET bot_allowed_at = now()
             WHERE id = $1 AND bot_allowed_at IS NULL RETURNING id",
            client_id,
        )
        .fetch_optional(&state.db)
        .await?
        .is_some();
        if newly_allowed {
            return Ok(Reply {
                message: welcome(state, client_id, chat_id, Some(&from.first_name)).await?,
                pin: true,
            });
        }
        let message = open_app(
            state,
            chat_id,
            format!("Тренування — у застосунку, кнопка нижче. {NOBODY_READS}"),
        );
        return Ok(Reply::plain(
            with_coach_contact(state, client_id, message).await?,
        ));
    }
    let is_coach = sqlx::query_scalar!("SELECT id FROM coaches WHERE telegram_id = $1", from.id)
        .fetch_optional(&state.db)
        .await?
        .is_some();
    if is_coach {
        let text = format!(
            "Кабінет тренера — кнопкою нижче або «{MENU_BUTTON_TEXT}» біля поля повідомлення. \
             На комп’ютері: {}",
            state.config.mini_app_url()
        );
        return Ok(Reply::plain(OutgoingMessage::text(chat_id, text).with_row(
            vec![Button::WebApp {
                text: "Відкрити кабінет".to_owned(),
                url: state.config.mini_app_url(),
            }],
        )));
    }
    // The ID is what's needed to add this person as the coach (see README).
    tracing::info!(
        telegram_id = from.id,
        "message from an unknown Telegram user"
    );
    Ok(Reply::plain(OutgoingMessage::text(
        chat_id,
        "Щоб почати, відкрий своє посилання-запрошення. Якщо його немає — попроси.",
    )))
}

async fn handle_callback(state: &AppState, query: CallbackQuery) -> anyhow::Result<()> {
    let decision = query
        .data
        .as_deref()
        .and_then(|data| data.split_once(':'))
        .and_then(|(action, id)| {
            let approve = match action {
                "login_ok" => true,
                "login_no" => false,
                _ => return None,
            };
            Some((approve, Uuid::parse_str(id).ok()?))
        });
    let Some((approve, login_id)) = decision else {
        return state.telegram.answer_callback_query(&query.id, "").await;
    };

    let decided = bot_login::decide(&state.db, login_id, query.from.id, approve).await?;
    let (toast, result) = match (decided, approve) {
        (true, true) => (
            "Вхід підтверджено",
            "Вхід підтверджено. Можна повертатися в застосунок.",
        ),
        (true, false) => ("Вхід скасовано", "Вхід скасовано."),
        (false, _) => (
            "Посилання застаріло",
            "Посилання для входу застаріло. Спробуй увійти ще раз.",
        ),
    };
    state
        .telegram
        .answer_callback_query(&query.id, toast)
        .await?;
    if let Some(message) = query.message {
        state
            .telegram
            .edit_message_text(message.chat.id, message.message_id, result)
            .await?;
    }
    Ok(())
}

fn open_app(state: &AppState, chat_id: i64, text: String) -> OutgoingMessage {
    OutgoingMessage::text(chat_id, text).with_row(vec![Button::WebApp {
        text: "Відкрити тренування".to_owned(),
        url: state.config.mini_app_url(),
    }])
}

async fn client_id(state: &AppState, telegram_id: i64) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar!(
        "SELECT id FROM clients WHERE telegram_id = $1 AND archived_at IS NULL",
        telegram_id
    )
    .fetch_optional(&state.db)
    .await
}

async fn is_client(state: &AppState, telegram_id: i64) -> sqlx::Result<bool> {
    Ok(client_id(state, telegram_id).await?.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_start_payloads() {
        assert_eq!(parse_start("/start inv_abc"), Some(Start::Invite("abc")));
        assert_eq!(parse_start("/start login_xyz"), Some(Start::Login("xyz")));
        assert_eq!(
            parse_start("/start@dasha_bot inv_abc"),
            Some(Start::Invite("abc"))
        );
        assert_eq!(parse_start("/start"), Some(Start::Plain));
        assert_eq!(parse_start("/start something"), Some(Start::Plain));
        assert_eq!(parse_start("/started"), None);
        assert_eq!(parse_start("привіт"), None);
    }
}
