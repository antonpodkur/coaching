//! HTTP routes and the OpenAPI description the frontend's types are generated from.

pub mod auth;
pub mod client;
pub mod coach;
pub mod form_videos;
pub mod health;
pub mod nutrition;
pub mod push;
pub mod questionnaire;
pub mod stream;
pub mod telegram;
pub mod weight;
pub mod workouts;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{
        Method,
        header::{AUTHORIZATION, CONTENT_TYPE, IF_MATCH},
    },
    routing::{delete, get, post, put},
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
        auth::refresh,
        auth::installed,
        push::key,
        push::subscribe,
        push::unsubscribe,
        client::me,
        client::set_timezone,
        client::allow_bot,
        client::set_avatar,
        client::remove_avatar,
        workouts::list,
        workouts::get,
        workouts::log_set,
        workouts::finish,
        form_videos::start,
        form_videos::finish,
        form_videos::delete,
        questionnaire::get_mine,
        questionnaire::save_answers,
        questionnaire::add_photo,
        questionnaire::start_video,
        questionnaire::finish_video,
        questionnaire::delete,
        questionnaire::get_for_coach,
        weight::list_mine,
        weight::log,
        weight::delete,
        weight::list_for_coach,
        nutrition::get_mine,
        nutrition::history,
        nutrition::set,
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
        coach::exercises::add_photo,
        coach::exercises::delete_photo,
        coach::import::parse,
        coach::workouts::list,
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
        (name = "push", description = "Notifications from the installed app (web push)"),
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
        .route("/auth/refresh", post(auth::refresh))
        .route("/auth/installed", post(auth::installed))
        .route("/push/key", get(push::key))
        .route(
            "/push/subscription",
            put(push::subscribe).delete(push::unsubscribe),
        )
        .route("/me", get(client::me))
        .route("/me/timezone", put(client::set_timezone))
        .route("/me/bot-allowed", post(client::allow_bot))
        .route(
            "/me/avatar",
            put(client::set_avatar)
                .delete(client::remove_avatar)
                .layer(DefaultBodyLimit::max(crate::photos::MAX_BYTES)),
        )
        .route("/me/workouts", get(workouts::list))
        .route("/workouts/{id}", get(workouts::get))
        .route("/workouts/{id}/finish", post(workouts::finish))
        .route("/sets/{id}/result", put(workouts::log_set))
        .route("/workout-exercises/{id}/videos", post(form_videos::start))
        .route("/form-videos/{id}/uploaded", post(form_videos::finish))
        .route("/form-videos/{id}", delete(form_videos::delete))
        .route(
            "/me/questionnaire",
            get(questionnaire::get_mine).put(questionnaire::save_answers),
        )
        .route(
            "/me/gym/photos",
            post(questionnaire::add_photo).layer(DefaultBodyLimit::max(crate::photos::MAX_BYTES)),
        )
        .route("/me/gym/videos", post(questionnaire::start_video))
        .route(
            "/me/gym/videos/{id}/uploaded",
            post(questionnaire::finish_video),
        )
        .route("/me/gym/{id}", delete(questionnaire::delete))
        .route("/me/weight", get(weight::list_mine))
        .route("/me/weight/{date}", put(weight::log).delete(weight::delete))
        .route("/me/nutrition", get(nutrition::get_mine))
        .route(
            "/coach/clients",
            get(coach::clients::list).post(coach::clients::create),
        )
        .route(
            "/coach/clients/{id}",
            get(coach::clients::get).patch(coach::clients::update),
        )
        .route("/coach/clients/{id}/invite", post(coach::clients::reinvite))
        .route(
            "/coach/clients/{id}/questionnaire",
            get(questionnaire::get_for_coach),
        )
        .route("/coach/clients/{id}/weight", get(weight::list_for_coach))
        .route(
            "/coach/clients/{id}/nutrition",
            get(nutrition::history).post(nutrition::set),
        )
        .route("/coach/me/timezone", put(coach::me::set_timezone))
        .route(
            "/coach/clients/{id}/workouts",
            get(coach::workouts::list_for_client),
        )
        .route(
            "/coach/workouts",
            get(coach::workouts::list).post(coach::workouts::create),
        )
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
        .route(
            "/coach/exercises/{id}/photos",
            post(coach::exercises::add_photo)
                .layer(DefaultBodyLimit::max(crate::photos::MAX_BYTES)),
        )
        .route(
            "/coach/exercise-photos/{id}",
            delete(coach::exercises::delete_photo),
        )
        .route("/coach/import/parse", post(coach::import::parse))
        .route("/telegram/webhook", post(telegram::webhook))
        .route("/webhooks/stream", post(stream::webhook))
        .route(
            "/webhooks/client-videos",
            post(stream::client_videos_webhook),
        )
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        // Keeps session tokens out of request traces.
        .layer(SetSensitiveRequestHeadersLayer::new([AUTHORIZATION]))
        .with_state(state)
}
