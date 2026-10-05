//! A small Bunny Storage client for photos: one private zone, written with the
//! zone's password and read only through signed CDN links.
//! Reference: https://docs.bunny.net/reference/storage-api

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Context;
use reqwest::StatusCode;

use crate::cdn_token;

/// One storage zone and the pull zone in front of it. Deliberately not
/// `Debug`: it holds the zone's password and the token key.
#[derive(Clone)]
pub struct StorageSettings {
    /// The zone's name, e.g. `coaching-photos`.
    pub zone: String,
    /// The zone's region endpoint, e.g. `storage.bunnycdn.com` for Frankfurt.
    pub hostname: String,
    /// The zone's (read and write) password. It never leaves the server.
    pub password: String,
    /// The pull zone's hostname, e.g. `coaching-photos.b-cdn.net`.
    pub cdn_hostname: String,
    /// The pull zone's token authentication key: files are only served
    /// through links signed with it.
    pub token_key: String,
}

/// Talks to Bunny, or keeps files in memory (tests and local runs).
#[derive(Clone)]
pub struct StorageClient {
    settings: Arc<StorageSettings>,
    backend: Backend,
}

#[derive(Clone)]
enum Backend {
    Live(reqwest::Client),
    /// Path → content type.
    Fake(Arc<Mutex<HashMap<String, String>>>),
}

impl StorageClient {
    pub fn live(settings: StorageSettings) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("building the HTTP client")?;
        Ok(Self {
            settings: Arc::new(settings),
            backend: Backend::Live(http),
        })
    }

    pub fn fake(settings: StorageSettings) -> Self {
        Self {
            settings: Arc::new(settings),
            backend: Backend::Fake(Arc::default()),
        }
    }

    /// Stores `bytes` at `path` (no leading slash), replacing any file there.
    pub async fn put(&self, path: &str, bytes: Vec<u8>, content_type: &str) -> anyhow::Result<()> {
        let http = match &self.backend {
            Backend::Fake(files) => {
                let mut files = files.lock().expect("lock is never poisoned");
                files.insert(path.to_owned(), content_type.to_owned());
                return Ok(());
            }
            Backend::Live(http) => http,
        };
        http.put(self.file_url(path))
            .header("AccessKey", &self.settings.password)
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(bytes)
            .send()
            .await
            .context("uploading to Bunny Storage")?
            .error_for_status()
            .context("uploading to Bunny Storage")?;
        Ok(())
    }

    /// Deletes the file at `path`; one that is already gone is fine.
    pub async fn delete(&self, path: &str) -> anyhow::Result<()> {
        let http = match &self.backend {
            Backend::Fake(files) => {
                files.lock().expect("lock is never poisoned").remove(path);
                return Ok(());
            }
            Backend::Live(http) => http,
        };
        let response = http
            .delete(self.file_url(path))
            .header("AccessKey", &self.settings.password)
            .send()
            .await
            .context("deleting from Bunny Storage")?;
        if response.status() != StatusCode::NOT_FOUND {
            response
                .error_for_status()
                .context("deleting from Bunny Storage")?;
        }
        Ok(())
    }

    /// A CDN link to the file at `path` that works until `expires` (Unix
    /// seconds) for everything under the `allowed` directory.
    pub fn signed_url(&self, allowed: &str, path: &str, expires: i64) -> String {
        cdn_token::signed_url(
            &self.settings.cdn_hostname,
            &self.settings.token_key,
            &format!("/{allowed}"),
            &format!("/{path}"),
            expires,
        )
    }

    /// Test hook: the paths stored in the fake zone, sorted.
    pub fn fake_files(&self) -> Vec<String> {
        match &self.backend {
            Backend::Fake(files) => {
                let mut paths: Vec<_> = files
                    .lock()
                    .expect("lock is never poisoned")
                    .keys()
                    .cloned()
                    .collect();
                paths.sort();
                paths
            }
            Backend::Live(_) => Vec::new(),
        }
    }

    fn file_url(&self, path: &str) -> String {
        format!(
            "https://{}/{}/{path}",
            self.settings.hostname, self.settings.zone
        )
    }
}
