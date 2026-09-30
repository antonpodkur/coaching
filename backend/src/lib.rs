//! Backend for Dasha's coaching app: REST API, Telegram auth and the plan importer.
//! See `docs/ARCHITECTURE.md` for the overall design.

pub mod api;
pub mod auth;
pub mod config;
pub mod error;
pub mod import;
pub mod state;

pub use api::{openapi, router};
