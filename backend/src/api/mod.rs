//! HTTP routes and the OpenAPI description the frontend's types are generated from.

pub mod auth;
pub mod client;
pub mod coach;
pub mod health;
pub mod telegram;

use axum::{
    Router,
    http::{
        Method,
        header::{AUTHORIZATION, CONTENT_TYPE, IF_MATCH},
    },
    routing::{get, post, put},
};
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer,
    sensitive_headers::SetSensitiveRequestHeadersLayer, trace::TraceLayer,
};
use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    info(title = "Coaching API", version = "0.1.0"),
    paths(
        health::health,
        auth::telegram_webapp,
        auth::bot_login_start,
        auth::bot_login_poll,
        client::me,
        client::set_timezone,
        coach::clients::list,
        coach::clients::create,
        coach::clients::reinvite,
        coach::import::parse,
    ),
    modifiers(&BearerAuth),
    tags(
        (name = "auth", description = "Telegram sign-in for clients and the coach"),
        (name = "client", description = "The Mini App"),
        (name = "coach", description = "Dasha's workspace"),
    )
)]
struct ApiDoc;

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        openapi
            .components
            .get_or_insert_with(Default::default)
            .add_security_scheme(
                "bearer",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
    }
}

pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(state.config.frontend_origin.clone())
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE, IF_MATCH]);

    Router::new()
        .route("/health", get(health::health))
        .route("/auth/telegram-webapp", post(auth::telegram_webapp))
        .route("/auth/bot-login", post(auth::bot_login_start))
        .route("/auth/bot-login/poll", post(auth::bot_login_poll))
        .route("/me", get(client::me))
        .route("/me/timezone", put(client::set_timezone))
        .route(
            "/coach/clients",
            get(coach::clients::list).post(coach::clients::create),
        )
        .route("/coach/clients/{id}/invite", post(coach::clients::reinvite))
        .route("/coach/import/parse", post(coach::import::parse))
        .route("/telegram/webhook", post(telegram::webhook))
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        // Keeps session tokens out of request traces.
        .layer(SetSensitiveRequestHeadersLayer::new([AUTHORIZATION]))
        .with_state(state)
}
