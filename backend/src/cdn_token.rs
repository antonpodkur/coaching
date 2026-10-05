//! Bunny CDN token authentication, for the private zones: clients' videos and
//! photos are only served through links signed here, and the links expire.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// Signed links last at least this long. They expire on the hour, so a list
/// fetched twice within an hour gets the same links and nothing reloads.
const LINK_HOURS: i64 = 6;

/// A link to `path` on `host`, signed with Bunny's directory token (HS256, as
/// in BunnyWay/BunnyCDN.TokenAuthentication): an HMAC-SHA256, keyed with the
/// token key, over the allowed directory, the expiry and `token_path=<dir>`.
/// The token goes into the URL path, so relative links inside an HLS playlist
/// carry it along, and every file under `allowed` works with it.
pub fn signed_url(host: &str, key: &str, allowed: &str, path: &str, expires: i64) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).expect("HMAC takes any key");
    mac.update(allowed.as_bytes());
    mac.update(expires.to_string().as_bytes());
    mac.update(format!("token_path={allowed}").as_bytes());
    let token = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    let token_path: String = form_urlencoded::byte_serialize(allowed.as_bytes()).collect();
    format!(
        "https://{host}/bcdn_token=HS256-{token}&token_path={token_path}&expires={expires}{path}"
    )
}

/// When links signed now expire: the top of the hour at least `LINK_HOURS` out.
pub fn link_expiry(now: DateTime<Utc>) -> i64 {
    let hour = 60 * 60;
    (now.timestamp() / hour + LINK_HOURS + 1) * hour
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_directories_the_way_bunny_documents() {
        // test_directory_and_path_allowed in BunnyWay/BunnyCDN.TokenAuthentication.
        assert_eq!(
            signed_url(
                "token-tester.b-cdn.net",
                "SecurityKey",
                "/abc",
                "/abc/",
                1_598_024_587
            ),
            "https://token-tester.b-cdn.net/bcdn_token=HS256-uVZvT3SbEoVKYJyDJgbcsDmSFf73cv-uNUVaJiKWpbQ\
             &token_path=%2Fabc&expires=1598024587/abc/"
        );
    }

    #[test]
    fn links_expire_on_the_hour_at_least_six_hours_out() {
        let now: DateTime<Utc> = "2026-10-06T09:59:00Z".parse().unwrap();
        let expires = DateTime::from_timestamp(link_expiry(now), 0).unwrap();
        assert_eq!(expires.to_rfc3339(), "2026-10-06T16:00:00+00:00");
        let later: DateTime<Utc> = "2026-10-06T09:01:00Z".parse().unwrap();
        assert_eq!(link_expiry(later), link_expiry(now));
    }
}
