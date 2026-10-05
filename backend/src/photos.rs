//! Photos the phone sends: JPEGs it already shrank, kept in the private
//! storage zone and shown only through signed links. Clients' gym photos and
//! Dasha's exercise photos both go this way.

use utoipa::ToSchema;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
    storage::StorageClient,
};

/// A photo shrunk on the phone is a few hundred KB; this leaves plenty of room.
pub const MAX_BYTES: usize = 4 * 1024 * 1024;

/// A photo file's bytes as a request body. Only describes the body in the
/// API, so the field is never read.
#[derive(ToSchema)]
#[schema(value_type = String, format = Binary)]
#[allow(dead_code)]
pub struct PhotoFile(Vec<u8>);

/// The storage zone, or 503 when it is not set up.
pub fn storage(state: &AppState) -> AppResult<&StorageClient> {
    state
        .storage
        .as_ref()
        .ok_or(AppError::Unavailable("photos_not_configured"))
}

/// Refuses anything but a JPEG of a sensible size before it is stored. A
/// JPEG starts with FF D8 FF.
pub fn check(jpeg: &[u8]) -> AppResult<()> {
    if jpeg.len() > MAX_BYTES || !jpeg.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Err(AppError::BadRequest("invalid_photo"));
    }
    Ok(())
}
