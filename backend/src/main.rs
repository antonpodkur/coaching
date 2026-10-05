use anyhow::Context;
use coaching_backend::{
    bot, config::Config, state::AppState, storage::StorageClient, telegram::TelegramClient,
    video::StreamClient,
};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("coaching_backend=info,tower_http=info")),
        )
        .init();

    let config = Config::from_env()?;
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("connecting to Postgres")?;
    sqlx::migrate!()
        .run(&db)
        .await
        .context("running migrations")?;
    if let Some(telegram_id) = config.coach_telegram_id
        && coaching_backend::coaches::ensure(&db, telegram_id).await?
    {
        tracing::info!(telegram_id, "created the coach from COACH_TELEGRAM_ID");
    }

    let telegram = TelegramClient::live(&config.bot_token)?;
    bot::register(&telegram, &config).await;

    let video = config.bunny.clone().map(StreamClient::live).transpose()?;
    if video.is_none() {
        tracing::warn!("Bunny Stream is not configured; video uploads are off");
    }

    let client_videos = config
        .client_videos
        .clone()
        .map(StreamClient::live)
        .transpose()?;
    if client_videos.is_none() {
        tracing::warn!("the client video library is not configured; clients cannot send videos");
    }

    let storage = config
        .storage
        .clone()
        .map(StorageClient::live)
        .transpose()?;
    if storage.is_none() {
        tracing::warn!("photo storage is not configured; clients cannot add photos");
    }

    let addr = config.bind_addr;
    let state = AppState::new(db, config, telegram)
        .with_video(video)
        .with_client_videos(client_videos)
        .with_storage(storage);
    // Reminders, Dasha's summary and retries run inside this process.
    tokio::spawn(coaching_backend::jobs::run(state.clone()));
    let app = coaching_backend::router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("installing the Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("installing the SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}
