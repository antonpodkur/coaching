//! Backend for Dasha's coaching app: REST API, Telegram bot and auth, and the
//! plan importer. See `docs/ARCHITECTURE.md` for the overall design.

pub mod api;
pub mod auth;
pub mod bot;
pub mod codes;
pub mod config;
pub mod error;
pub mod import;
pub mod invites;
pub mod state;
pub mod telegram;

pub use api::{openapi, router};
