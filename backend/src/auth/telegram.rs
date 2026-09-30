//! Verification of Telegram-signed login data.
//!
//! Clients sign in with Mini App `initData`; the coach signs in with the Login
//! Widget. Both are an HMAC-SHA256 over a data-check-string (every field except
//! `hash`, sorted by key, as `key=value` lines joined with `\n`). They differ only
//! in how the HMAC key is derived from the bot token.
//! Spec: https://core.telegram.org/bots/webapps#validating-data-received-via-the-mini-app
//! and https://core.telegram.org/widgets/login#checking-authorization

use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

type HmacSha256 = Hmac<Sha256>;

/// Logins older than this are rejected, so a leaked payload stops working.
pub const MAX_AUTH_AGE_SECS: i64 = 24 * 60 * 60;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum TelegramAuthError {
    #[error("hash is missing")]
    MissingHash,
    #[error("signature does not match")]
    BadSignature,
    #[error("login is too old or from the future")]
    Expired,
    #[error("field `{0}` is missing or malformed")]
    Malformed(&'static str),
}

/// The Telegram user inside Mini App `initData`.
#[derive(Debug, Deserialize)]
pub struct WebAppUser {
    pub id: i64,
    pub first_name: String,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
}

#[derive(Debug)]
pub struct WebAppInitData {
    pub user: WebAppUser,
    pub auth_date: i64,
    /// `startapp` parameter of the link that opened the Mini App, e.g. `w_<id>`.
    pub start_param: Option<String>,
}

/// Checks Mini App `initData` (the raw query string from `Telegram.WebApp.initData`).
pub fn verify_web_app_init_data(
    init_data: &str,
    bot_token: &str,
    now: i64,
) -> Result<WebAppInitData, TelegramAuthError> {
    let mut hash = None;
    let mut fields: Vec<(String, String)> = Vec::new();
    for (key, value) in form_urlencoded::parse(init_data.as_bytes()) {
        if key == "hash" {
            hash = Some(value.into_owned());
        } else {
            fields.push((key.into_owned(), value.into_owned()));
        }
    }
    let hash = hash.ok_or(TelegramAuthError::MissingHash)?;
    fields.sort_by(|a, b| a.0.cmp(&b.0));

    let secret = hmac_sha256(b"WebAppData", bot_token.as_bytes());
    let check = data_check_string(fields.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    verify_hex(&secret, &check, &hash)?;

    let field = |name: &str| {
        fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    };
    let auth_date = field("auth_date")
        .and_then(|v| v.parse().ok())
        .ok_or(TelegramAuthError::Malformed("auth_date"))?;
    check_age(auth_date, now)?;
    let user = field("user")
        .and_then(|v| serde_json::from_str(v).ok())
        .ok_or(TelegramAuthError::Malformed("user"))?;

    Ok(WebAppInitData {
        user,
        auth_date,
        start_param: field("start_param").map(str::to_owned),
    })
}

/// What the Telegram Login Widget passes to its `data-onauth` callback.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginWidgetPayload {
    pub id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub photo_url: Option<String>,
    pub auth_date: i64,
    pub hash: String,
}

/// Checks a Login Widget payload.
pub fn verify_login_widget(
    payload: &LoginWidgetPayload,
    bot_token: &str,
    now: i64,
) -> Result<(), TelegramAuthError> {
    let id = payload.id.to_string();
    let auth_date = payload.auth_date.to_string();
    let mut fields = vec![
        ("auth_date", auth_date.as_str()),
        ("first_name", payload.first_name.as_str()),
        ("id", id.as_str()),
    ];
    for (key, value) in [
        ("last_name", &payload.last_name),
        ("photo_url", &payload.photo_url),
        ("username", &payload.username),
    ] {
        if let Some(value) = value {
            fields.push((key, value.as_str()));
        }
    }
    fields.sort_by_key(|(key, _)| *key);

    let secret = Sha256::digest(bot_token.as_bytes());
    verify_hex(&secret, &data_check_string(fields), &payload.hash)?;
    check_age(payload.auth_date, now)
}

fn data_check_string<'a>(fields: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    fields
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(message);
    mac.finalize().into_bytes().to_vec()
}

/// Compares in constant time, so the check does not leak how many bytes matched.
fn verify_hex(secret: &[u8], check: &str, hash_hex: &str) -> Result<(), TelegramAuthError> {
    let expected = hex::decode(hash_hex).map_err(|_| TelegramAuthError::BadSignature)?;
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts keys of any length");
    mac.update(check.as_bytes());
    mac.verify_slice(&expected)
        .map_err(|_| TelegramAuthError::BadSignature)
}

fn check_age(auth_date: i64, now: i64) -> Result<(), TelegramAuthError> {
    const CLOCK_SKEW_SECS: i64 = 60;
    if now - auth_date > MAX_AUTH_AGE_SECS || auth_date - now > CLOCK_SKEW_SECS {
        return Err(TelegramAuthError::Expired);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Vectors computed independently with Python's `hmac` module, following
    // Telegram's docs. Replace or extend with a real payload from the dev bot.
    const BOT_TOKEN: &str = "123456:TEST-bot-token";
    const AUTH_DATE: i64 = 1_759_200_000;
    const INIT_DATA: &str = "auth_date=1759200000&query_id=AAH-test-query&start_param=w_42&user=%7B%22id%22%3A777000111%2C%22first_name%22%3A%22%D0%9C%D0%B0%D0%BA%D1%81%D0%B8%D0%BC%22%2C%22last_name%22%3A%22%D0%9A.%22%2C%22username%22%3A%22maksym_k%22%2C%22language_code%22%3A%22uk%22%7D&hash=e849d21dc1c59773038c5df28daa128412a2d4b5ce07c1be8d42d71685bd874c";

    #[test]
    fn accepts_valid_init_data() {
        let data = verify_web_app_init_data(INIT_DATA, BOT_TOKEN, AUTH_DATE + 60).unwrap();
        assert_eq!(data.user.id, 777_000_111);
        assert_eq!(data.user.first_name, "Максим");
        assert_eq!(data.start_param.as_deref(), Some("w_42"));
    }

    #[test]
    fn rejects_tampered_init_data() {
        let tampered = INIT_DATA.replace("777000111", "777000112");
        assert_eq!(
            verify_web_app_init_data(&tampered, BOT_TOKEN, AUTH_DATE).unwrap_err(),
            TelegramAuthError::BadSignature
        );
    }

    #[test]
    fn rejects_init_data_signed_for_another_bot() {
        assert_eq!(
            verify_web_app_init_data(INIT_DATA, "654321:other-bot", AUTH_DATE).unwrap_err(),
            TelegramAuthError::BadSignature
        );
    }

    #[test]
    fn rejects_old_init_data() {
        let a_day_and_a_second_later = AUTH_DATE + MAX_AUTH_AGE_SECS + 1;
        assert_eq!(
            verify_web_app_init_data(INIT_DATA, BOT_TOKEN, a_day_and_a_second_later).unwrap_err(),
            TelegramAuthError::Expired
        );
    }

    #[test]
    fn rejects_init_data_without_hash() {
        let (without_hash, _) = INIT_DATA.split_once("&hash=").unwrap();
        assert_eq!(
            verify_web_app_init_data(without_hash, BOT_TOKEN, AUTH_DATE).unwrap_err(),
            TelegramAuthError::MissingHash
        );
    }

    fn login_payload() -> LoginWidgetPayload {
        LoginWidgetPayload {
            id: 555_000_222,
            first_name: "Даша".into(),
            last_name: None,
            username: Some("daria_k".into()),
            photo_url: None,
            auth_date: AUTH_DATE,
            hash: "6bacae6cc378306bb2e35addfb25e1440f4be37d059a335e3d7bc6441610013a".into(),
        }
    }

    #[test]
    fn accepts_valid_login_widget_payload() {
        assert_eq!(
            verify_login_widget(&login_payload(), BOT_TOKEN, AUTH_DATE),
            Ok(())
        );
    }

    #[test]
    fn rejects_tampered_login_widget_payload() {
        let payload = LoginWidgetPayload {
            username: Some("someone_else".into()),
            ..login_payload()
        };
        assert_eq!(
            verify_login_widget(&payload, BOT_TOKEN, AUTH_DATE),
            Err(TelegramAuthError::BadSignature)
        );
    }
}
