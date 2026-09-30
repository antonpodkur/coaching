use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

pub type AppResult<T> = Result<T, AppError>;

/// Errors a handler can return. Each maps to a status code and a machine-readable
/// `error` code that the frontend turns into Ukrainian text.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("{0}")]
    Forbidden(&'static str),
    #[error("not_found")]
    NotFound,
    #[error("{0}")]
    BadRequest(&'static str),
    /// The request clashes with existing data, e.g. `name_taken`.
    #[error("{0}")]
    Conflict(&'static str),
    /// A feature whose settings are missing, e.g. `video_not_configured`.
    #[error("{0}")]
    Unavailable(&'static str),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::Internal(err.into())
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    /// Machine-readable code, e.g. `unauthorized`, `not_invited`, `unknown_timezone`.
    pub error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(err) => {
                tracing::error!(error = ?err, "request failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        let error = match self {
            Self::Internal(_) => "internal".to_owned(),
            other => other.to_string(),
        };
        (status, Json(ErrorBody { error })).into_response()
    }
}
