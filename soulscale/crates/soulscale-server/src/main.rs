//! Soulscale server binary. Configuration: environment / `.env` (see `.env.example`).

use std::time::Duration;

use soulscale_server::{Config, install_crypto, router, spawn_ticker, state};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();
    install_crypto();

    let cfg = Config::from_env()?;
    if !cfg.static_dir.join("index.html").exists() {
        tracing::warn!(dir = %cfg.static_dir.display(), "client build not found: run `npm run build` in client/ (API and /ws still work)");
    }
    if cfg.dev_login {
        tracing::warn!("dev logins are ON: anyone can connect without Discord. Set SOULSCALE_DEV_LOGIN=off in production.");
    }
    if !cfg.discord_ready() {
        tracing::warn!("DISCORD_CLIENT_ID / DISCORD_CLIENT_SECRET not set: Discord sign-in disabled");
    }
    let bind = cfg.bind;
    let st = state(cfg)?;
    spawn_ticker(st.clone());
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!("Soulscale server listening on http://{bind}");
    axum::serve(listener, router(st))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("shutting down");
        })
        .await?;
    // Give in-flight replay writes a moment to land.
    tokio::time::sleep(Duration::from_millis(200)).await;
    Ok(())
}
