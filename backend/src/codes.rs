//! Secrets that travel in links (invites, coach sign-in) and are stored hashed.

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// A new random secret for a link, and the hash to store instead of it.
///
/// 32 hex characters with 122 random bits from the OS generator; fits Telegram's
/// 64-character `start` parameter together with a prefix.
pub fn new_secret() -> (String, String) {
    let secret = Uuid::new_v4().simple().to_string();
    let hash = hash(&secret);
    (secret, hash)
}

pub fn hash(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

/// Four digits shown both on the page and in the bot, so the coach only
/// confirms a sign-in she started herself.
pub fn display_code() -> String {
    let bytes = Uuid::new_v4().into_bytes();
    (u16::from_be_bytes([bytes[0], bytes[1]]) % 9000 + 1000).to_string()
}

/// Compares two secrets without leaking, through timing, how much of them matched.
pub fn secrets_match(given: &str, expected: &str) -> bool {
    Sha256::digest(given.as_bytes()) == Sha256::digest(expected.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_unique_and_stored_hashed() {
        let (a, a_hash) = new_secret();
        let (b, _) = new_secret();
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
        assert_eq!(a_hash, hash(&a));
        assert_ne!(a_hash, a);
    }

    #[test]
    fn display_codes_have_four_digits() {
        for _ in 0..200 {
            let code = display_code();
            assert_eq!(code.len(), 4);
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn compares_secrets() {
        assert!(secrets_match("abc", "abc"));
        assert!(!secrets_match("abc", "abd"));
    }
}
