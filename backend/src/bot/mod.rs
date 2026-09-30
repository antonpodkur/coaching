//! What the bot does with incoming updates: invite links, coach sign-in, and a
//! short answer to anything else. It only talks in private chats.

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
        Some(Start::Login(code)) => claim_login(state, &from, chat_id, code).await?,
        Some(Start::Plain) | None => greet(state, &from, chat_id).await?,
    };
    state.telegram.send_message(reply).await
}

async fn accept_invite(
    state: &AppState,
    from: &User,
    chat_id: i64,
    code: &str,
) -> anyhow::Result<OutgoingMessage> {
    match invites::accept(&state.db, code, from.id).await? {
        Accepted::Joined {
            client_name,
            coach_telegram_id,
        } => {
            // Let Dasha know; losing this message must not fail the invite.
            let note = OutgoingMessage::text(
                coach_telegram_id,
                format!("{client_name} тепер у застосунку."),
            );
            if let Err(err) = state.telegram.send_message(note).await {
                tracing::warn!(error = ?err, "could not tell the coach about a new client");
            }
            Ok(open_app(
                state,
                chat_id,
                format!(
                    "Вітаю, {}! Тут будуть твої тренування від Даші: план, відео техніки й звіт після тренування.",
                    from.first_name
                ),
            ))
        }
        Accepted::LinkedElsewhere => Ok(OutgoingMessage::text(
            chat_id,
            "Цей Telegram-акаунт уже прив’язаний до іншого профілю. Напиши Даші — вона допоможе.",
        )),
        // A repeated update, or an old link tapped again by someone who already joined.
        Accepted::Invalid if is_client(state, from.id).await? => Ok(open_app(
            state,
            chat_id,
            "Ти вже в застосунку. Відкривай тренування кнопкою нижче.".to_owned(),
        )),
        Accepted::Invalid => Ok(OutgoingMessage::text(
            chat_id,
            "Посилання недійсне або застаріло. Попроси в Даші нове.",
        )),
    }
}

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
        } => OutgoingMessage::text(
            chat_id,
            format!(
                "Увійти в кабінет тренера?\n\nКод на екрані має бути {display_code}. \
                 Якщо ти не входила зараз — натисни «Скасувати»."
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
        ]),
        Claimed::NotACoach => {
            OutgoingMessage::text(chat_id, "Цей акаунт не має доступу до кабінету тренера.")
        }
        Claimed::Invalid => OutgoingMessage::text(
            chat_id,
            "Посилання для входу недійсне або застаріло. Спробуй ще раз у браузері.",
        ),
    })
}

async fn greet(state: &AppState, from: &User, chat_id: i64) -> anyhow::Result<OutgoingMessage> {
    if is_client(state, from.id).await? {
        return Ok(open_app(
            state,
            chat_id,
            "Я надсилаю тренування й нагадування. Питання щодо тренувань — пиши Даші напряму."
                .to_owned(),
        ));
    }
    let is_coach = sqlx::query_scalar!("SELECT id FROM coaches WHERE telegram_id = $1", from.id)
        .fetch_optional(&state.db)
        .await?
        .is_some();
    if is_coach {
        let text = format!(
            "Кабінет тренера — кнопкою нижче або «{MENU_BUTTON_TEXT}» біля поля повідомлення. \
             На комп’ютері: {}/coach",
            state.config.frontend_url
        );
        return Ok(
            OutgoingMessage::text(chat_id, text).with_row(vec![Button::WebApp {
                text: "Відкрити кабінет".to_owned(),
                url: state.config.mini_app_url(),
            }]),
        );
    }
    // The ID is what's needed to add this person as the coach (see README).
    tracing::info!(
        telegram_id = from.id,
        "message from an unknown Telegram user"
    );
    Ok(OutgoingMessage::text(
        chat_id,
        "Щоб почати, попроси в Даші посилання-запрошення.",
    ))
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
            "Вхід підтверджено. Повернись у браузер.",
        ),
        (true, false) => ("Вхід скасовано", "Вхід скасовано."),
        (false, _) => (
            "Посилання застаріло",
            "Посилання для входу застаріло. Спробуй ще раз у браузері.",
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

async fn is_client(state: &AppState, telegram_id: i64) -> sqlx::Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT id FROM clients WHERE telegram_id = $1 AND archived_at IS NULL",
        telegram_id
    )
    .fetch_optional(&state.db)
    .await?
    .is_some())
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
