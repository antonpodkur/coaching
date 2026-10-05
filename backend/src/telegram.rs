//! A small Telegram Bot API client: just the methods the bot uses, over reqwest.
//! Reference: https://core.telegram.org/bots/api

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, bail, ensure};
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
    Pinned {
        chat_id: i64,
        message_id: i64,
    },
    /// A file downloaded through the Bot API, e.g. a profile photo.
    Downloaded {
        file_id: String,
    },
    /// A message card prepared for `user_id` to share from the Mini App.
    Prepared {
        user_id: i64,
        text: String,
        button: Button,
    },
}

/// A message card the Mini App can share to a chat (`WebApp.shareMessage`):
/// text with one link button under it.
#[derive(Debug, Clone, PartialEq)]
pub struct ShareableMessage {
    /// Shown in Telegram's share dialog above the chat list.
    pub title: String,
    pub description: String,
    pub text: String,
    /// A link button; `web_app` buttons only work in chats with the bot.
    pub button: Button,
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
    /// Profile photos by user: Telegram's unique ID and the JPEG.
    profile_photos: Mutex<std::collections::HashMap<i64, (String, Vec<u8>)>>,
}

/// A user's current profile photo, in the size the app keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfilePhoto {
    /// For `download_file`.
    pub file_id: String,
    /// Stays the same for the same photo, so an unchanged one is not fetched again.
    pub unique_id: String,
}

/// Profile photo sizes go up to 640 px; this one is enough for an avatar.
const AVATAR_PHOTO_WIDTH: u32 = 320;

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

    /// Test hook: the user's profile photo, or none (hidden or removed).
    pub fn set_profile_photo(&self, user_id: i64, photo: Option<(&str, Vec<u8>)>) {
        if let Self::Recording(recorder) = self {
            let mut photos = recorder
                .profile_photos
                .lock()
                .expect("lock is never poisoned");
            match photo {
                Some((unique_id, jpeg)) => photos.insert(user_id, (unique_id.to_owned(), jpeg)),
                None => photos.remove(&user_id),
            };
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

    /// Returns the sent message's ID, e.g. to pin it.
    pub async fn send_message(&self, message: OutgoingMessage) -> anyhow::Result<i64> {
        let mut body = json!({ "chat_id": message.chat_id, "text": message.text });
        if !message.keyboard.is_empty() {
            let rows: Vec<Vec<Value>> = message
                .keyboard
                .iter()
                .map(|row| row.iter().map(Button::to_json).collect())
                .collect();
            body["reply_markup"] = json!({ "inline_keyboard": rows });
        }
        let sent = self
            .call("sendMessage", body, Sent::Message(message))
            .await?;
        sent["message_id"]
            .as_i64()
            .context("sendMessage returned no message_id")
    }

    /// Pins a message in a private chat without a notification.
    pub async fn pin_chat_message(&self, chat_id: i64, message_id: i64) -> anyhow::Result<()> {
        let body = json!({
            "chat_id": chat_id,
            "message_id": message_id,
            "disable_notification": true,
        });
        let sent = Sent::Pinned {
            chat_id,
            message_id,
        };
        self.call("pinChatMessage", body, sent).await?;
        Ok(())
    }

    /// Stores a message `user_id` can then share from the Mini App; returns its ID.
    /// Only to private chats with people, not bots, groups or channels.
    pub async fn save_prepared_message(
        &self,
        user_id: i64,
        message: ShareableMessage,
    ) -> anyhow::Result<String> {
        let body = json!({
            "user_id": user_id,
            "result": {
                "type": "article",
                "id": uuid::Uuid::new_v4().simple().to_string(),
                "title": message.title,
                "description": message.description,
                "input_message_content": { "message_text": message.text },
                "reply_markup": { "inline_keyboard": [[message.button.to_json()]] },
            },
            "allow_user_chats": true,
        });
        let sent = Sent::Prepared {
            user_id,
            text: message.text,
            button: message.button,
        };
        let prepared = self.call("savePreparedInlineMessage", body, sent).await?;
        prepared["id"]
            .as_str()
            .map(str::to_owned)
            .context("savePreparedInlineMessage returned no id")
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
        self.call("editMessageText", body, sent).await?;
        Ok(())
    }

    /// Stops the button's loading spinner and shows `text` briefly.
    pub async fn answer_callback_query(&self, id: &str, text: &str) -> anyhow::Result<()> {
        let body = json!({ "callback_query_id": id, "text": text });
        let sent = Sent::CallbackAnswer {
            id: id.to_owned(),
            text: text.to_owned(),
        };
        self.call("answerCallbackQuery", body, sent).await?;
        Ok(())
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
        self.call("setWebhook", body, sent).await?;
        Ok(())
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
        self.call("setChatMenuButton", body, sent).await?;
        Ok(())
    }

    /// The user's current profile photo, or `None` if they have none or their
    /// privacy settings hide it from bots.
    pub async fn profile_photo(&self, user_id: i64) -> anyhow::Result<Option<ProfilePhoto>> {
        let (http, base_url) = match self {
            Self::Recording(recorder) => {
                if recorder.failing.load(std::sync::atomic::Ordering::SeqCst) {
                    bail!("getUserProfilePhotos failed: Telegram is unreachable (test)");
                }
                let photos = recorder
                    .profile_photos
                    .lock()
                    .expect("lock is never poisoned");
                return Ok(photos.get(&user_id).map(|(unique_id, _)| ProfilePhoto {
                    file_id: format!("file-{user_id}-{unique_id}"),
                    unique_id: unique_id.clone(),
                }));
            }
            Self::Live { http, base_url } => (http, base_url),
        };

        #[derive(Deserialize)]
        struct Photos {
            photos: Vec<Vec<PhotoSize>>,
        }
        #[derive(Deserialize)]
        struct PhotoSize {
            file_id: String,
            file_unique_id: String,
            width: u32,
        }
        let result = call_api(
            http,
            base_url,
            "getUserProfilePhotos",
            json!({ "user_id": user_id, "limit": 1 }),
        )
        .await?;
        let photos: Photos = serde_json::from_value(result).context("reading profile photos")?;
        let Some(mut sizes) = photos.photos.into_iter().next() else {
            return Ok(None);
        };
        // The smallest size wide enough for an avatar, or else the largest.
        sizes.sort_by_key(|size| size.width);
        let index = sizes
            .iter()
            .position(|size| size.width >= AVATAR_PHOTO_WIDTH)
            .unwrap_or(sizes.len().saturating_sub(1));
        Ok(sizes.into_iter().nth(index).map(|size| ProfilePhoto {
            file_id: size.file_id,
            unique_id: size.file_unique_id,
        }))
    }

    /// Downloads a file the Bot API knows, e.g. a profile photo, up to `max_bytes`.
    /// Its link carries the bot token, so files are copied, never linked.
    pub async fn download_file(&self, file_id: &str, max_bytes: usize) -> anyhow::Result<Vec<u8>> {
        let (http, base_url) = match self {
            Self::Recording(recorder) => {
                let photos = recorder
                    .profile_photos
                    .lock()
                    .expect("lock is never poisoned");
                let jpeg = photos
                    .iter()
                    .find(|(user_id, (unique_id, _))| {
                        file_id == format!("file-{user_id}-{unique_id}")
                    })
                    .map(|(_, (_, jpeg))| jpeg.clone())
                    .context("no such file (test)")?;
                drop(photos);
                recorder
                    .sent
                    .lock()
                    .expect("lock is never poisoned")
                    .push(Sent::Downloaded {
                        file_id: file_id.to_owned(),
                    });
                return Ok(jpeg);
            }
            Self::Live { http, base_url } => (http, base_url),
        };

        #[derive(Deserialize)]
        struct File {
            file_path: String,
        }
        let result = call_api(http, base_url, "getFile", json!({ "file_id": file_id })).await?;
        let file: File = serde_json::from_value(result).context("reading the file's path")?;
        // https://api.telegram.org/bot<token> → https://api.telegram.org/file/bot<token>
        let url = format!(
            "{}/{}",
            base_url.replacen("/bot", "/file/bot", 1),
            file.file_path
        );
        let response = http
            .get(url)
            .send()
            .await
            .map_err(reqwest::Error::without_url)
            .context("downloading a file")?
            .error_for_status()
            .map_err(reqwest::Error::without_url)
            .context("downloading a file")?;
        let bytes = response
            .bytes()
            .await
            .map_err(reqwest::Error::without_url)
            .context("downloading a file")?;
        ensure!(
            bytes.len() <= max_bytes,
            "the file is larger than {max_bytes} bytes"
        );
        Ok(bytes.to_vec())
    }

    /// Calls a Bot API method and returns its `result`.
    async fn call(&self, method: &str, body: Value, record: Sent) -> anyhow::Result<Value> {
        let (http, base_url) = match self {
            Self::Recording(recorder) => {
                if recorder.failing.load(std::sync::atomic::Ordering::SeqCst) {
                    bail!("{method} failed: Telegram is unreachable (test)");
                }
                let mut sent = recorder.sent.lock().expect("lock is never poisoned");
                // Results shaped like Telegram's, numbered by call.
                let n = sent.len() + 1;
                let result = match &record {
                    Sent::Message(_) => json!({ "message_id": n }),
                    Sent::Prepared { .. } => json!({ "id": format!("prepared-{n}") }),
                    _ => json!(true),
                };
                sent.push(record);
                return Ok(result);
            }
            Self::Live { http, base_url } => (http, base_url),
        };
        call_api(http, base_url, method, body).await
    }
}

/// Calls a Bot API method over the network and returns its `result`.
async fn call_api(
    http: &reqwest::Client,
    base_url: &str,
    method: &str,
    body: Value,
) -> anyhow::Result<Value> {
    #[derive(Deserialize)]
    struct Reply {
        ok: bool,
        description: Option<String>,
        #[serde(default)]
        result: Value,
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
    Ok(reply.result)
}
