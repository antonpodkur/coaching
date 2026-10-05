//! Who is calling: Telegram-signed logins, our own session tokens, and the axum
//! extractors that handlers use to require a client or the coach.

pub mod bot_login;
pub mod extract;
pub mod jwt;
pub mod telegram;

pub use extract::{CurrentClient, CurrentCoach, CurrentSession};
pub use jwt::{JwtKeys, Role};
