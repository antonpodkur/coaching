//! Backend for Dasha's coaching app: REST API, Telegram bot and auth, and the
//! plan importer. See `docs/ARCHITECTURE.md` for the overall design.

pub mod api;
pub mod auth;
pub mod bot;
pub mod cdn_token;
pub mod coaches;
pub mod codes;
pub mod config;
pub mod error;
pub mod exercise_photos;
pub mod form_videos;
pub mod history;
pub mod import;
pub mod invites;
pub mod jobs;
pub mod notify;
pub mod nutrition;
pub mod photos;
pub mod push;
pub mod questionnaire;
pub mod state;
pub mod storage;
pub mod telegram;
pub mod timezone;
pub mod video;
pub mod weight;

pub use api::{openapi, router};
