//! A small Bunny Stream client: the calls the backend makes, upload signing and
//! webhook verification. Reference: https://bunny.net/docs/stream

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Context;
use axum::http::HeaderMap;
use hmac::{Hmac, KeyInit, Mac};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

const API_BASE: &str = "https://video.bunnycdn.com";
/// Where the phone sends the file, with tus.
pub const TUS_ENDPOINT: &str = "https://video.bunnycdn.com/tusupload";

/// One Bunny Stream library. Deliberately not `Debug`: it holds API keys.
#[derive(Clone)]
pub struct StreamSettings {
    pub library_id: String,
    /// The library's Stream API key. It signs uploads and never leaves the server.
    pub api_key: String,
    /// The library's read-only API key, which Bunny signs webhooks with. When it
    /// is set, unsigned webhooks are refused.
    pub read_only_api_key: Option<String>,
    /// The library's CDN hostname, e.g. `vz-1234abcd-567.b-cdn.net`.
    pub cdn_hostname: String,
}

/// Encoding as Bunny's API reports it (`VideoModel.status`). Webhooks number
/// their statuses differently, which is one reason the backend always asks the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// Created; the file has not fully arrived yet.
    AwaitingUpload,
    InProgress,
    Finished,
    Failed,
}

impl Encoding {
    fn from_api(status: i32) -> Self {
        match status {
            0 => Self::AwaitingUpload,
            4 => Self::Finished,
            // Error, UploadFailed.
            5 | 6 => Self::Failed,
            // Uploaded, Processing, Transcoding, and the JIT encoding steps.
            _ => Self::InProgress,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoState {
    pub encoding: Encoding,
    pub length_secs: i32,
}

/// Talks to Bunny, or keeps an in-memory library (tests and local runs).
#[derive(Clone)]
pub struct StreamClient {
    settings: Arc<StreamSettings>,
    backend: Backend,
}

#[derive(Clone)]
enum Backend {
    Live(reqwest::Client),
    Fake(Arc<Mutex<FakeLibrary>>),
}

#[derive(Default)]
struct FakeLibrary {
    created: u32,
    /// Video ID → (API status code, length in seconds).
    videos: HashMap<String, (i32, i32)>,
    deleted: Vec<String>,
}

impl StreamClient {
    pub fn live(settings: StreamSettings) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .context("building the HTTP client")?;
        Ok(Self {
            settings: Arc::new(settings),
            backend: Backend::Live(http),
        })
    }

    pub fn fake(settings: StreamSettings) -> Self {
        Self {
            settings: Arc::new(settings),
            backend: Backend::Fake(Arc::default()),
        }
    }

    pub fn settings(&self) -> &StreamSettings {
        &self.settings
    }

    /// Creates an empty video that an upload then fills. Returns its ID.
    pub async fn create_video(&self, title: &str) -> anyhow::Result<String> {
        let http = match &self.backend {
            Backend::Fake(library) => {
                let mut library = library.lock().expect("lock is never poisoned");
                library.created += 1;
                let guid = format!("fake-video-{}", library.created);
                library.videos.insert(guid.clone(), (0, 0));
                return Ok(guid);
            }
            Backend::Live(http) => http,
        };

        #[derive(Deserialize)]
        struct Created {
            guid: String,
        }
        let created: Created = http
            .post(self.videos_url())
            .header("AccessKey", &self.settings.api_key)
            .json(&json!({ "title": title }))
            .send()
            .await
            .context("creating a Bunny video")?
            .error_for_status()
            .context("creating a Bunny video")?
            .json()
            .await
            .context("reading the created Bunny video")?;
        Ok(created.guid)
    }

    /// Where the video is in encoding; `None` if Bunny no longer has it.
    pub async fn video(&self, guid: &str) -> anyhow::Result<Option<VideoState>> {
        let http = match &self.backend {
            Backend::Fake(library) => {
                let library = library.lock().expect("lock is never poisoned");
                return Ok(library
                    .videos
                    .get(guid)
                    .map(|&(status, length_secs)| VideoState {
                        encoding: Encoding::from_api(status),
                        length_secs,
                    }));
            }
            Backend::Live(http) => http,
        };

        #[derive(Deserialize)]
        struct Video {
            status: i32,
            length: i32,
        }
        let response = http
            .get(format!("{}/{guid}", self.videos_url()))
            .header("AccessKey", &self.settings.api_key)
            .send()
            .await
            .context("asking Bunny about a video")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let video: Video = response
            .error_for_status()
            .context("asking Bunny about a video")?
            .json()
            .await
            .context("reading a Bunny video")?;
        Ok(Some(VideoState {
            encoding: Encoding::from_api(video.status),
            length_secs: video.length,
        }))
    }

    /// Deletes a video Bunny stores for us. Already gone counts as done.
    pub async fn delete_video(&self, guid: &str) -> anyhow::Result<()> {
        let http = match &self.backend {
            Backend::Fake(library) => {
                let mut library = library.lock().expect("lock is never poisoned");
                library.videos.remove(guid);
                library.deleted.push(guid.to_owned());
                return Ok(());
            }
            Backend::Live(http) => http,
        };
        let response = http
            .delete(format!("{}/{guid}", self.videos_url()))
            .header("AccessKey", &self.settings.api_key)
            .send()
            .await
            .context("deleting a Bunny video")?;
        if response.status() != StatusCode::NOT_FOUND {
            response
                .error_for_status()
                .context("deleting a Bunny video")?;
        }
        Ok(())
    }

    /// The tus `AuthorizationSignature` that lets a browser upload into this one
    /// video until `expires_at` (Unix seconds), without seeing the API key.
    pub fn upload_signature(&self, guid: &str, expires_at: i64) -> String {
        let settings = &self.settings;
        let signed = format!(
            "{}{}{expires_at}{guid}",
            settings.library_id, settings.api_key
        );
        hex::encode(Sha256::digest(signed.as_bytes()))
    }

    pub fn hls_url(&self, guid: &str) -> String {
        format!(
            "https://{}/{guid}/playlist.m3u8",
            self.settings.cdn_hostname
        )
    }

    pub fn thumbnail_url(&self, guid: &str) -> String {
        format!(
            "https://{}/{guid}/thumbnail.jpg",
            self.settings.cdn_hostname
        )
    }

    /// Checks Bunny's `v1` webhook signature: lowercase hex HMAC-SHA256 of the
    /// raw body, keyed with the library's read-only API key.
    pub fn webhook_is_authentic(&self, headers: &HeaderMap, body: &[u8]) -> bool {
        let Some(key) = &self.settings.read_only_api_key else {
            // Not configured: accept, since webhooks only prompt a check with the API.
            return true;
        };
        let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
        if header("x-bunnystream-signature-version") != Some("v1")
            || header("x-bunnystream-signature-algorithm") != Some("hmac-sha256")
        {
            return false;
        }
        let Some(signature) =
            header("x-bunnystream-signature").and_then(|hex| hex::decode(hex).ok())
        else {
            return false;
        };
        let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).expect("HMAC takes any key");
        mac.update(body);
        mac.verify_slice(&signature).is_ok()
    }

    /// Test hook: what the fake library reports for `guid` from now on.
    pub fn fake_set_status(&self, guid: &str, api_status: i32, length_secs: i32) {
        if let Backend::Fake(library) = &self.backend {
            let mut library = library.lock().expect("lock is never poisoned");
            library
                .videos
                .insert(guid.to_owned(), (api_status, length_secs));
        }
    }

    /// Test hook: videos deleted from the fake library, oldest first.
    pub fn fake_deleted(&self) -> Vec<String> {
        match &self.backend {
            Backend::Fake(library) => library
                .lock()
                .expect("lock is never poisoned")
                .deleted
                .clone(),
            Backend::Live(_) => Vec::new(),
        }
    }

    fn videos_url(&self) -> String {
        format!("{API_BASE}/library/{}/videos", self.settings.library_id)
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn client(read_only_api_key: Option<&str>) -> StreamClient {
        StreamClient::fake(StreamSettings {
            library_id: "12345".to_owned(),
            api_key: "stream-api-key".to_owned(),
            read_only_api_key: read_only_api_key.map(str::to_owned),
            cdn_hostname: "vz-test.b-cdn.net".to_owned(),
        })
    }

    #[test]
    fn signs_uploads_the_way_bunny_documents() {
        // Computed with Python's hashlib: sha256("12345" + "stream-api-key" + "1790000000" + "abc-guid").
        assert_eq!(
            client(None).upload_signature("abc-guid", 1_790_000_000),
            "682764341626b2052da091e43e52cb57e36933c81b326066b10b75f9320f754a"
        );
    }

    #[test]
    fn maps_api_statuses() {
        assert_eq!(Encoding::from_api(0), Encoding::AwaitingUpload);
        assert_eq!(Encoding::from_api(3), Encoding::InProgress);
        assert_eq!(Encoding::from_api(4), Encoding::Finished);
        assert_eq!(Encoding::from_api(6), Encoding::Failed);
    }

    #[test]
    fn checks_webhook_signatures() {
        let body = br#"{"VideoLibraryId":12345,"VideoGuid":"abc-guid","Status":3}"#;
        let mut mac = Hmac::<Sha256>::new_from_slice(b"read-only-key").unwrap();
        mac.update(body);
        let good = hex::encode(mac.finalize().into_bytes());

        let signed = |signature: &str| {
            let mut headers = HeaderMap::new();
            headers.insert(
                "x-bunnystream-signature-version",
                HeaderValue::from_static("v1"),
            );
            headers.insert(
                "x-bunnystream-signature-algorithm",
                HeaderValue::from_static("hmac-sha256"),
            );
            headers.insert(
                "x-bunnystream-signature",
                HeaderValue::from_str(signature).unwrap(),
            );
            headers
        };

        let stream = client(Some("read-only-key"));
        assert!(stream.webhook_is_authentic(&signed(&good), body));
        assert!(!stream.webhook_is_authentic(&signed(&good), b"{}"));
        assert!(!stream.webhook_is_authentic(&signed(&"0".repeat(64)), body));
        assert!(!stream.webhook_is_authentic(&HeaderMap::new(), body));
        // Without the key configured, webhooks are only a prompt to ask the API.
        assert!(client(None).webhook_is_authentic(&HeaderMap::new(), body));
    }
}
