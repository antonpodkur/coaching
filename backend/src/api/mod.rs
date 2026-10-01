//! HTTP routes and the OpenAPI description the frontend's types are generated from.

pub mod auth;
pub mod client;
pub mod coach;
pub mod health;
pub mod stream;
pub mod telegram;
pub mod workouts;

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
        workouts::list,
        workouts::get,
        workouts::log_set,
        workouts::finish,
        coach::clients::list,
        coach::clients::create,
        coach::clients::get,
        coach::clients::update,
        coach::clients::reinvite,
        coach::me::set_timezone,
        coach::exercises::list,
        coach::exercises::create,
        coach::exercises::get,
        coach::exercises::update,
        coach::exercises::start_video_upload,
        coach::exercises::finish_video_upload,
        coach::import::parse,
        coach::workouts::create,
        coach::workouts::get,
        coach::workouts::save,
        coach::workouts::publish,
        coach::workouts::delete,
        coach::workouts::list_for_client,
        coach::workouts::results,
        coach::workouts::mark_report_seen,
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
        .route("/me/workouts", get(workouts::list))
        .route("/workouts/{id}", get(workouts::get))
        .route("/workouts/{id}/finish", post(workouts::finish))
        .route("/sets/{id}/result", put(workouts::log_set))
        .route(
            "/coach/clients",
            get(coach::clients::list).post(coach::clients::create),
        )
        .route(
            "/coach/clients/{id}",
            get(coach::clients::get).patch(coach::clients::update),
        )
        .route("/coach/clients/{id}/invite", post(coach::clients::reinvite))
        .route("/coach/me/timezone", put(coach::me::set_timezone))
        .route(
            "/coach/clients/{id}/workouts",
            get(coach::workouts::list_for_client),
        )
        .route("/coach/workouts", post(coach::workouts::create))
        .route(
            "/coach/workouts/{id}",
            get(coach::workouts::get)
                .put(coach::workouts::save)
                .delete(coach::workouts::delete),
        )
        .route(
            "/coach/workouts/{id}/publish",
            post(coach::workouts::publish),
        )
        .route(
            "/coach/workouts/{id}/results",
            get(coach::workouts::results),
        )
        .route(
            "/coach/workouts/{id}/report/seen",
            post(coach::workouts::mark_report_seen),
        )
        .route(
            "/coach/exercises",
            get(coach::exercises::list).post(coach::exercises::create),
        )
        .route(
            "/coach/exercises/{id}",
            get(coach::exercises::get).patch(coach::exercises::update),
        )
        .route(
            "/coach/exercises/{id}/video-upload",
            post(coach::exercises::start_video_upload),
        )
        .route(
            "/coach/exercises/{id}/video-uploaded",
            post(coach::exercises::finish_video_upload),
        )
        .route("/coach/import/parse", post(coach::import::parse))
        .route("/telegram/webhook", post(telegram::webhook))
        .route("/webhooks/stream", post(stream::webhook))
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        // Keeps session tokens out of request traces.
        .layer(SetSensitiveRequestHeadersLayer::new([AUTHORIZATION]))
        .with_state(state)
}
