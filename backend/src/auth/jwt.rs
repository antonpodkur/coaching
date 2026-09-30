use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Clients re-authenticate with fresh `initData` on every Mini App launch.
pub const CLIENT_TOKEN_TTL: Duration = Duration::hours(12);
/// The coach signs in with the Login Widget much less often.
pub const COACH_TOKEN_TTL: Duration = Duration::days(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Client,
    Coach,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// `clients.id` or `coaches.id`, depending on `role`.
    pub sub: Uuid,
    pub role: Role,
    pub iat: i64,
    pub exp: i64,
}

/// HS256 session tokens. Rotating `JWT_SECRET` signs everyone out.
pub struct JwtKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
}

impl JwtKeys {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
            validation: Validation::new(Algorithm::HS256),
        }
    }

    pub fn issue(&self, role: Role, sub: Uuid, ttl: Duration) -> anyhow::Result<String> {
        let now = Utc::now();
        let claims = Claims {
            sub,
            role,
            iat: now.timestamp(),
            exp: (now + ttl).timestamp(),
        };
        Ok(encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &self.encoding,
        )?)
    }

    /// `None` for anything that is not a valid, unexpired token we issued.
    pub fn verify(&self, token: &str) -> Option<Claims> {
        decode::<Claims>(token, &self.decoding, &self.validation)
            .ok()
            .map(|data| data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret-that-is-long-enough-for-hs256";

    #[test]
    fn round_trips_a_token() {
        let keys = JwtKeys::new(SECRET);
        let id = Uuid::new_v4();
        let token = keys.issue(Role::Coach, id, COACH_TOKEN_TTL).unwrap();
        let claims = keys.verify(&token).unwrap();
        assert_eq!((claims.sub, claims.role), (id, Role::Coach));
    }

    #[test]
    fn rejects_expired_and_foreign_tokens() {
        let keys = JwtKeys::new(SECRET);
        let expired = keys
            .issue(Role::Client, Uuid::new_v4(), Duration::hours(-1))
            .unwrap();
        assert!(keys.verify(&expired).is_none());

        let other = JwtKeys::new("another-secret-that-is-also-long-enough");
        let foreign = other
            .issue(Role::Client, Uuid::new_v4(), CLIENT_TOKEN_TTL)
            .unwrap();
        assert!(keys.verify(&foreign).is_none());
    }
}
