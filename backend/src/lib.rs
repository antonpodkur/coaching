//! Backend for Dasha's coaching app: REST API, Telegram bot and auth, and the
//! plan importer. See `docs/ARCHITECTURE.md` for the overall design.

pub mod api;
pub mod auth;
pub mod bot;
pub mod codes;
pub mod config;
pub mod error;
pub mod history;
pub mod import;
pub mod invites;
pub mod notify;
pub mod state;
pub mod telegram;
pub mod video;

pub use api::{openapi, router};
