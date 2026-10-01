//! A small Telegram Bot API client: just the methods the bot uses, over reqwest.
//! Reference: https://core.telegram.org/bots/api

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, bail};
use serde::Deserialize;
use serde_json::{Value, json};

/// An incoming update. Only the parts the bot handles are read.
#[derive(Debug, Deserialize)]
pub struct Update {
    pub message: Option<Message>,
    pub callback_query: Option<CallbackQuery>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub message_id: i64,
    pub chat: Chat,
    pub from: Option<User>,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Chat {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub id: i64,
    pub first_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub id: String,
    pub from: User,
    /// The message with the button; absent if it is too old for Telegram to include.
    pub message: Option<Message>,
    pub data: Option<String>,
}

/// A message to send, with an optional inline keyboard (rows of buttons).
#[derive(Debug, Clone, PartialEq)]
pub struct OutgoingMessage {
    pub chat_id: i64,
    pub text: String,
    pub keyboard: Vec<Vec<Button>>,
}

impl OutgoingMessage {
    pub fn text(chat_id: i64, text: impl Into<String>) -> Self {
        Self {
            chat_id,
            text: text.into(),
            keyboard: Vec::new(),
        }
    }

    pub fn with_row(mut self, row: Vec<Button>) -> Self {
        self.keyboard.push(row);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Button {
    /// Opens the Mini App at `url` (HTTPS only).
    WebApp { text: String, url: String },
    /// Opens a link.
    Url { text: String, url: String },
    /// Sends `data` back to the bot as a callback query.
    Callback { text: String, data: String },
}

impl Button {
    fn to_json(&self) -> Value {
        match self {
            Self::WebApp { text, url } => json!({ "text": text, "web_app": { "url": url } }),
            Self::Url { text, url } => json!({ "text": text, "url": url }),
            Self::Callback { text, data } => json!({ "text": text, "callback_data": data }),
        }
    }
}

/// A call the bot made; what [`TelegramClient::recording`] keeps for tests.
#[derive(Debug, Clone, PartialEq)]
pub enum Sent {
    Message(OutgoingMessage),
    Edited {
        chat_id: i64,
        message_id: i64,
        text: String,
    },
    CallbackAnswer {
        id: String,
        text: String,
    },
    WebhookSet {
        url: String,
    },
    MenuButtonSet {
        text: String,
        url: String,
    },
}

/// Talks to Telegram, or records what it would have sent (tests).
///
/// Deliberately not `Debug`: the live variant's URL contains the bot token.
#[derive(Clone)]
pub enum TelegramClient {
    Live {
        http: reqwest::Client,
        /// `https://api.telegram.org/bot<token>`.
        base_url: String,
    },
    Recording(Arc<Recorder>),
}

/// What the test client keeps: calls made, and whether to fail them.
#[derive(Default)]
pub struct Recorder {
    sent: Mutex<Vec<Sent>>,
    failing: std::sync::atomic::AtomicBool,
}

impl TelegramClient {
    pub fn live(bot_token: &str) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("building the HTTP client")?;
        Ok(Self::Live {
            http,
            base_url: format!("https://api.telegram.org/bot{bot_token}"),
        })
    }

    pub fn recording() -> Self {
        Self::Recording(Arc::default())
    }

    /// Test hook: make every call fail, as if Telegram were down.
    pub fn set_failing(&self, failing: bool) {
        if let Self::Recording(recorder) = self {
            recorder
                .failing
                .store(failing, std::sync::atomic::Ordering::SeqCst);
        }
    }

    /// Everything sent so far; always empty for the live client.
    pub fn sent(&self) -> Vec<Sent> {
        match self {
            Self::Live { .. } => Vec::new(),
            Self::Recording(recorder) => recorder
                .sent
                .lock()
                .expect("lock is never poisoned")
                .clone(),
        }
    }

    pub async fn send_message(&self, message: OutgoingMessage) -> anyhow::Result<()> {
        let mut body = json!({ "chat_id": message.chat_id, "text": message.text });
        if !message.keyboard.is_empty() {
            let rows: Vec<Vec<Value>> = message
                .keyboard
                .iter()
                .map(|row| row.iter().map(Button::to_json).collect())
                .collect();
            body["reply_markup"] = json!({ "inline_keyboard": rows });
        }
        self.call("sendMessage", body, Sent::Message(message)).await
    }

    /// Replaces a message's text and removes its buttons.
    pub async fn edit_message_text(
        &self,
        chat_id: i64,
        message_id: i64,
        text: &str,
    ) -> anyhow::Result<()> {
        let body = json!({ "chat_id": chat_id, "message_id": message_id, "text": text });
        let sent = Sent::Edited {
            chat_id,
            message_id,
            text: text.to_owned(),
        };
        self.call("editMessageText", body, sent).await
    }

    /// Stops the button's loading spinner and shows `text` briefly.
    pub async fn answer_callback_query(&self, id: &str, text: &str) -> anyhow::Result<()> {
        let body = json!({ "callback_query_id": id, "text": text });
        let sent = Sent::CallbackAnswer {
            id: id.to_owned(),
            text: text.to_owned(),
        };
        self.call("answerCallbackQuery", body, sent).await
    }

    pub async fn set_webhook(&self, url: &str, secret_token: &str) -> anyhow::Result<()> {
        let body = json!({
            "url": url,
            "secret_token": secret_token,
            "allowed_updates": ["message", "callback_query"],
        });
        let sent = Sent::WebhookSet {
            url: url.to_owned(),
        };
        self.call("setWebhook", body, sent).await
    }

    /// Makes the button next to the message box open the Mini App, in every
    /// private chat with the bot.
    pub async fn set_default_menu_button(&self, text: &str, url: &str) -> anyhow::Result<()> {
        let body = json!({
            "menu_button": { "type": "web_app", "text": text, "web_app": { "url": url } },
        });
        let sent = Sent::MenuButtonSet {
            text: text.to_owned(),
            url: url.to_owned(),
        };
        self.call("setChatMenuButton", body, sent).await
    }

    async fn call(&self, method: &str, body: Value, record: Sent) -> anyhow::Result<()> {
        let (http, base_url) = match self {
            Self::Recording(recorder) => {
                if recorder.failing.load(std::sync::atomic::Ordering::SeqCst) {
                    bail!("{method} failed: Telegram is unreachable (test)");
                }
                recorder
                    .sent
                    .lock()
                    .expect("lock is never poisoned")
                    .push(record);
                return Ok(());
            }
            Self::Live { http, base_url } => (http, base_url),
        };

        #[derive(Deserialize)]
        struct Reply {
            ok: bool,
            description: Option<String>,
        }

        // `without_url` keeps the bot token out of error messages and logs.
        let reply: Reply = http
            .post(format!("{base_url}/{method}"))
            .json(&body)
            .send()
            .await
            .map_err(reqwest::Error::without_url)
            .with_context(|| format!("calling {method}"))?
            .json()
            .await
            .map_err(reqwest::Error::without_url)
            .with_context(|| format!("reading the {method} reply"))?;
        if !reply.ok {
            bail!("{method} failed: {}", reply.description.unwrap_or_default());
        }
        Ok(())
    }
}
