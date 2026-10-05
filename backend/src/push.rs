//! Web push: notifications the installed app shows itself, like any app on
//! the phone, where a tap opens the right screen. Each phone (or computer)
//! that allowed them has a row in `push_subscriptions`; the bot's messages
//! keep going out as well.
//!
//! Sending is best effort: a push that fails is logged, not retried, and a
//! subscription the push service reports gone is deleted.
//! References: RFC 8030 (push), RFC 8291 (encryption), RFC 8292 (VAPID).

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, anyhow};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::StatusCode;
use serde::Serialize;
use uuid::Uuid;
use web_push_native::{
    Auth, WebPushBuilder,
    jwt_simple::algorithms::ES256KeyPair,
    p256::{self, elliptic_curve::sec1::ToEncodedPoint},
};

use crate::state::AppState;

/// How long a push service keeps a message for a phone that is off. Reminders
/// and reports are stale after a day; this is also the VAPID token's lifetime.
const KEEP_FOR: Duration = Duration::from_secs(12 * 60 * 60);

/// Push services browsers use. Subscriptions elsewhere are refused, so the
/// server never posts to an address someone made up.
const PUSH_SERVICES: [&str; 4] = [
    "fcm.googleapis.com",
    "push.services.mozilla.com",
    "push.apple.com",
    "notify.windows.com",
];

/// What a notification shows: a title, a line, and the screen a tap opens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PushMessage {
    pub title: String,
    pub body: String,
    /// A path in the app, e.g. `/app/nutrition`.
    pub url: String,
    /// A newer notification with the same tag replaces the older one.
    pub tag: String,
}

/// Who gets a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recipient {
    Client(Uuid),
    Coach(Uuid),
}

/// One push that went out, as the recording client keeps it (tests).
#[derive(Debug, Clone)]
pub struct Pushed {
    pub endpoint: String,
    /// The encrypted request body, as the push service would get it.
    pub body: Vec<u8>,
    pub headers: Vec<(String, String)>,
}

/// Signs and encrypts pushes, and sends them or records them (tests).
/// Deliberately not `Debug`: it holds the private key.
#[derive(Clone)]
pub struct PushClient {
    keys: Arc<ES256KeyPair>,
    /// The public key, for the browser's `pushManager.subscribe`.
    public_key: String,
    /// Who sends: push services may contact this address about problems.
    contact: String,
    backend: Backend,
}

#[derive(Clone)]
enum Backend {
    Live(reqwest::Client),
    Recording(Arc<Mutex<Vec<Pushed>>>),
}

/// What a push service said about one subscription.
enum Outcome {
    Delivered,
    /// The browser unsubscribed or the app was removed: forget it.
    Gone,
}

impl PushClient {
    /// `private_key` is the VAPID key, 32 bytes in URL-safe base64
    /// (`cargo run --bin vapid-key` makes one); `contact` an `https:` or
    /// `mailto:` address.
    pub fn live(private_key: &str, contact: String) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .context("building the HTTP client")?;
        Self::new(private_key, contact, Backend::Live(http))
    }

    /// Records pushes instead of sending them. A subscription whose address
    /// contains `gone` answers like an unsubscribed browser.
    pub fn recording(private_key: &str, contact: String) -> anyhow::Result<Self> {
        Self::new(private_key, contact, Backend::Recording(Arc::default()))
    }

    fn new(private_key: &str, contact: String, backend: Backend) -> anyhow::Result<Self> {
        let raw = URL_SAFE_NO_PAD
            .decode(private_key.trim())
            .context("VAPID_PRIVATE_KEY is not URL-safe base64")?;
        let keys =
            ES256KeyPair::from_bytes(&raw).map_err(|err| anyhow!("VAPID_PRIVATE_KEY: {err}"))?;
        Ok(Self {
            keys: Arc::new(keys),
            public_key: public_key(&raw)?,
            contact,
            backend,
        })
    }

    pub fn public_key(&self) -> &str {
        &self.public_key
    }

    /// Pushes recorded so far, oldest first (recording client only).
    pub fn sent(&self) -> Vec<Pushed> {
        match &self.backend {
            Backend::Recording(sent) => sent.lock().expect("lock poisoned").clone(),
            Backend::Live(_) => Vec::new(),
        }
    }

    async fn send(&self, subscription: &Subscription, payload: &[u8]) -> anyhow::Result<Outcome> {
        let auth: [u8; 16] = URL_SAFE_NO_PAD
            .decode(&subscription.auth)?
            .try_into()
            .map_err(|_| anyhow!("the subscription's auth secret is not 16 bytes"))?;
        let request = WebPushBuilder::new(
            subscription.endpoint.parse()?,
            p256::PublicKey::from_sec1_bytes(&URL_SAFE_NO_PAD.decode(&subscription.p256dh)?)?,
            Auth::from(auth),
        )
        .with_valid_duration(KEEP_FOR)
        .with_vapid(&self.keys, &self.contact)
        .build(payload)
        .map_err(|err| anyhow!("building the push: {err}"))?;

        let http = match &self.backend {
            Backend::Live(http) => http,
            Backend::Recording(sent) => {
                let headers = request
                    .headers()
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.to_string(),
                            value.to_str().unwrap_or_default().to_owned(),
                        )
                    })
                    .collect();
                sent.lock().expect("lock poisoned").push(Pushed {
                    endpoint: subscription.endpoint.clone(),
                    body: request.body().clone(),
                    headers,
                });
                let gone = subscription.endpoint.contains("gone");
                return Ok(if gone {
                    Outcome::Gone
                } else {
                    Outcome::Delivered
                });
            }
        };
        let response = http.execute(reqwest::Request::try_from(request)?).await?;
        match response.status() {
            status if status.is_success() => Ok(Outcome::Delivered),
            StatusCode::NOT_FOUND | StatusCode::GONE => Ok(Outcome::Gone),
            status => {
                let text = response.text().await.unwrap_or_default();
                Err(anyhow!("the push service answered {status}: {text}"))
            }
        }
    }
}

/// The browser's key for `applicationServerKey`: the uncompressed point.
fn public_key(private_key: &[u8]) -> anyhow::Result<String> {
    let secret = p256::SecretKey::from_slice(private_key).context("not a P-256 private key")?;
    let point = secret.public_key().to_encoded_point(false);
    Ok(URL_SAFE_NO_PAD.encode(point.as_bytes()))
}

/// A new VAPID key pair: the private key for `VAPID_PRIVATE_KEY`, and the
/// public key browsers will see.
pub fn new_keys() -> (String, String) {
    let keys = ES256KeyPair::generate();
    let private = keys.to_bytes();
    let public = public_key(&private).expect("a generated key is valid");
    (URL_SAFE_NO_PAD.encode(private), public)
}

/// Whether a subscription's address belongs to a browser's push service.
pub fn known_push_service(endpoint: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(endpoint) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    url.scheme() == "https"
        && PUSH_SERVICES
            .iter()
            .any(|service| host == *service || host.ends_with(&format!(".{service}")))
}

/// Whether `p256dh` and `auth` are the browser's keys: a P-256 point and 16 bytes.
pub fn valid_keys(p256dh: &str, auth: &str) -> bool {
    let point = URL_SAFE_NO_PAD.decode(p256dh).ok();
    let secret = URL_SAFE_NO_PAD.decode(auth).ok();
    point.is_some_and(|point| p256::PublicKey::from_sec1_bytes(&point).is_ok())
        && secret.is_some_and(|secret| secret.len() == 16)
}

struct Subscription {
    id: Uuid,
    endpoint: String,
    p256dh: String,
    auth: String,
}

/// Sends `message` to every phone of `recipient`. Best effort: failures are
/// logged, and subscriptions the push service reports gone are deleted.
pub async fn notify(state: &AppState, recipient: Recipient, message: &PushMessage) {
    let Some(push) = &state.push else {
        return;
    };
    if let Err(err) = notify_all(state, push, recipient, message).await {
        tracing::warn!(error = ?err, "could not send push notifications");
    }
}

async fn notify_all(
    state: &AppState,
    push: &PushClient,
    recipient: Recipient,
    message: &PushMessage,
) -> anyhow::Result<()> {
    let (client_id, coach_id) = match recipient {
        Recipient::Client(id) => (Some(id), None),
        Recipient::Coach(id) => (None, Some(id)),
    };
    let subscriptions = sqlx::query_as!(
        Subscription,
        "SELECT id, endpoint, p256dh, auth FROM push_subscriptions
         WHERE client_id = $1 OR coach_id = $2",
        client_id,
        coach_id,
    )
    .fetch_all(&state.db)
    .await?;
    let payload = serde_json::to_vec(message)?;
    for subscription in subscriptions {
        match push.send(&subscription, &payload).await {
            Ok(Outcome::Delivered) => {}
            Ok(Outcome::Gone) => {
                sqlx::query!(
                    "DELETE FROM push_subscriptions WHERE id = $1",
                    subscription.id
                )
                .execute(&state.db)
                .await?;
            }
            Err(err) => tracing::warn!(error = ?err, "a push notification failed"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_browsers_push_services() {
        assert!(known_push_service(
            "https://fcm.googleapis.com/fcm/send/abc"
        ));
        assert!(known_push_service("https://web.push.apple.com/QGuQyavXut"));
        assert!(known_push_service(
            "https://updates.push.services.mozilla.com/wpush/v2/x"
        ));
        assert!(known_push_service(
            "https://wns2-am3p.notify.windows.com/w/?token=x"
        ));
        assert!(!known_push_service(
            "http://fcm.googleapis.com/fcm/send/abc"
        ));
        assert!(!known_push_service(
            "https://evil.example.com/fcm.googleapis.com"
        ));
        assert!(!known_push_service(
            "https://fcm.googleapis.com.evil.example/x"
        ));
        assert!(!known_push_service("not a url"));
    }

    #[test]
    fn new_keys_work_and_give_the_browsers_key() {
        let (private, public) = new_keys();
        let client = PushClient::recording(&private, "https://app.example.com".into()).unwrap();
        assert_eq!(client.public_key(), public);
        // The browser takes an uncompressed point: 65 bytes starting with 4.
        let point = URL_SAFE_NO_PAD.decode(&public).unwrap();
        assert_eq!((point.len(), point[0]), (65, 4));
        assert!(PushClient::recording("not-a-key", String::new()).is_err());
    }
}
