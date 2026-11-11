//! Server configuration from environment variables (a `.env` file in the working
//! directory is loaded first if present). See `.env.example` at the repo root.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;

#[derive(Clone, Debug)]
pub struct Config {
    /// SOULSCALE_BIND (default 127.0.0.1:3001)
    pub bind: SocketAddr,
    /// SOULSCALE_STATIC_DIR (default ./client/dist): the built client.
    pub static_dir: PathBuf,
    /// SOULSCALE_DATA_DIR (default ./data): replays and playtest exports.
    pub data_dir: PathBuf,
    /// DISCORD_CLIENT_ID: your application's OAuth2 client ID.
    pub client_id: Option<String>,
    /// DISCORD_CLIENT_SECRET: OAuth2 client secret (never sent to clients).
    pub client_secret: Option<String>,
    /// DISCORD_BOT_TOKEN (optional): lets the server check a player really is in the
    /// Activity instance they claim.
    pub bot_token: Option<String>,
    /// SOULSCALE_DEV_LOGIN: allow "dev_name" logins for browser testing outside Discord.
    /// Defaults to on when no client secret is configured, off otherwise.
    pub dev_login: bool,
    /// SOULSCALE_TURN_TIMEOUT_SECS (default 120): a Duel player idle this long has the
    /// AI play that round for them.
    pub turn_timeout: Duration,
    /// SOULSCALE_RECONNECT_GRACE_SECS (default 30): a disconnected Duel player's seat
    /// passes to the AI after this long, and back when they return.
    pub reconnect_grace: Duration,
}

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

fn secs(name: &str, default: u64) -> anyhow::Result<Duration> {
    Ok(Duration::from_secs(match var(name) {
        Some(v) => v.parse().with_context(|| format!("{name} must be a whole number of seconds"))?,
        None => default,
    }))
}

impl Config {
    pub fn from_env() -> anyhow::Result<Config> {
        let bind = var("SOULSCALE_BIND").unwrap_or_else(|| "127.0.0.1:3001".into());
        let client_secret = var("DISCORD_CLIENT_SECRET");
        let dev_login = match var("SOULSCALE_DEV_LOGIN").as_deref() {
            Some("1" | "true" | "yes" | "on") => true,
            Some("0" | "false" | "no" | "off") => false,
            Some(other) => anyhow::bail!("SOULSCALE_DEV_LOGIN must be on/off, got {other:?}"),
            None => client_secret.is_none(),
        };
        Ok(Config {
            bind: bind.parse().with_context(|| format!("SOULSCALE_BIND {bind:?} is not host:port"))?,
            static_dir: var("SOULSCALE_STATIC_DIR").unwrap_or_else(|| "client/dist".into()).into(),
            data_dir: var("SOULSCALE_DATA_DIR").unwrap_or_else(|| "data".into()).into(),
            client_id: var("DISCORD_CLIENT_ID"),
            client_secret,
            bot_token: var("DISCORD_BOT_TOKEN"),
            dev_login,
            turn_timeout: secs("SOULSCALE_TURN_TIMEOUT_SECS", 120)?,
            reconnect_grace: secs("SOULSCALE_RECONNECT_GRACE_SECS", 30)?,
        })
    }

    pub fn discord_ready(&self) -> bool {
        self.client_id.is_some() && self.client_secret.is_some()
    }
}
