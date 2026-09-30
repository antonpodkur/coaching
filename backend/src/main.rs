use anyhow::Context;
use coaching_backend::{config::Config, state::AppState, telegram::TelegramClient};
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

    let telegram = TelegramClient::live(&config.bot_token)?;
    if let Some(url) = &config.telegram_webhook_url {
        // Not fatal: the API still works, and the next start retries.
        match telegram.set_webhook(url, &config.webhook_secret).await {
            Ok(()) => tracing::info!(%url, "Telegram webhook registered"),
            Err(err) => tracing::warn!(error = ?err, "could not register the Telegram webhook"),
        }
    }

    let addr = config.bind_addr;
    let app = coaching_backend::router(AppState::new(db, config, telegram));
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
