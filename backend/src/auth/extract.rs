use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

use crate::{
    auth::{Role, jwt::Claims},
    error::AppError,
    state::AppState,
};

/// The signed-in client. Handlers take `client_id` only from here, never from the request.
pub struct CurrentClient(pub Uuid);

/// The signed-in coach.
pub struct CurrentCoach(pub Uuid);

/// Whoever is signed in, the coach or a client: their session token's claims.
pub struct CurrentSession(pub Claims);

impl FromRequestParts<AppState> for CurrentClient {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let client_id = subject_with_role(parts, state, Role::Client)?;
        // Archiving a client ends their sessions now, not when the token runs out.
        let active = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM clients WHERE id = $1 AND archived_at IS NULL) AS "active!""#,
            client_id,
        )
        .fetch_one(&state.db)
        .await?;
        if !active {
            return Err(AppError::Unauthorized);
        }
        Ok(Self(client_id))
    }
}

impl FromRequestParts<AppState> for CurrentCoach {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        subject_with_role(parts, state, Role::Coach).map(Self)
    }
}

impl FromRequestParts<AppState> for CurrentSession {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        claims(parts, state).map(Self)
    }
}

fn claims(parts: &Parts, state: &AppState) -> Result<Claims, AppError> {
    let token = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;
    state.jwt.verify(token).ok_or(AppError::Unauthorized)
}

fn subject_with_role(parts: &Parts, state: &AppState, role: Role) -> Result<Uuid, AppError> {
    let claims = claims(parts, state)?;
    if claims.role != role {
        return Err(AppError::Forbidden("wrong_role"));
    }
    Ok(claims.sub)
}
