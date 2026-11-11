//! Discord HTTP calls: OAuth code exchange, identity lookup, optional instance check.

use std::time::Duration;

use anyhow::{Context, anyhow, bail};
use serde::Deserialize;
use soulscale_core::protocol::UserInfo;

use crate::config::Config;

const API: &str = "https://discord.com/api/v10";

pub fn http_client() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("soulscale-server/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(10))
        .build()
        .context("building HTTP client")
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Exchange the `authorize` code for an access token (the client secret stays here).
pub async fn exchange_code(http: &reqwest::Client, cfg: &Config, code: &str) -> anyhow::Result<String> {
    let (Some(id), Some(secret)) = (&cfg.client_id, &cfg.client_secret) else {
        bail!("DISCORD_CLIENT_ID / DISCORD_CLIENT_SECRET are not configured");
    };
    let form = [
        ("client_id", id.as_str()),
        ("client_secret", secret.as_str()),
        ("grant_type", "authorization_code"),
        ("code", code),
    ];
    let resp = http.post(format!("{API}/oauth2/token")).form(&form).send().await.context("contacting Discord")?;
    let status = resp.status();
    if !status.is_success() {
        // Do not echo Discord's body: it can contain request details.
        bail!("Discord rejected the code exchange ({status})");
    }
    Ok(resp.json::<TokenResponse>().await.context("reading Discord token response")?.access_token)
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
}

/// Who owns this access token (`GET /users/@me`).
pub async fn fetch_user(http: &reqwest::Client, token: &str) -> anyhow::Result<UserInfo> {
    let resp = http.get(format!("{API}/users/@me")).bearer_auth(token).send().await.context("contacting Discord")?;
    if !resp.status().is_success() {
        bail!("Discord did not accept the access token ({})", resp.status());
    }
    let u: DiscordUser = resp.json().await.context("reading Discord user")?;
    // cdn.discordapp.com/avatars/ is on Discord's Activity CSP allow-list.
    let avatar = u.avatar.as_ref().map(|h| format!("https://cdn.discordapp.com/avatars/{}/{h}.png?size=96", u.id));
    Ok(UserInfo { id: u.id, name: clean_name(u.global_name.as_deref().unwrap_or(&u.username)), avatar })
}

#[derive(Deserialize)]
struct ActivityInstance {
    users: Vec<String>,
}

/// With a bot token configured, confirm `user_id` is connected to `instance_id`.
/// Returns Ok(true) when no bot token is set (check disabled).
pub async fn verify_instance(http: &reqwest::Client, cfg: &Config, instance: &str, user_id: &str) -> anyhow::Result<bool> {
    let (Some(app), Some(bot)) = (&cfg.client_id, &cfg.bot_token) else {
        return Ok(true);
    };
    if !instance.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Ok(false);
    }
    let resp = http
        .get(format!("{API}/applications/{app}/activity-instances/{instance}"))
        .header("Authorization", format!("Bot {bot}"))
        .send()
        .await
        .context("contacting Discord")?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(false);
    }
    if !resp.status().is_success() {
        return Err(anyhow!("instance lookup failed ({})", resp.status()));
    }
    Ok(resp.json::<ActivityInstance>().await?.users.iter().any(|u| u == user_id))
}

/// Display names: trim, strip control characters, cap length.
pub fn clean_name(raw: &str) -> String {
    let s: String = raw.chars().filter(|c| !c.is_control()).take(32).collect();
    let s = s.trim();
    if s.is_empty() { "Soul".into() } else { s.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_cleaned() {
        assert_eq!(clean_name("  wofl\u{0007} "), "wofl");
        assert_eq!(clean_name(""), "Soul");
        assert_eq!(clean_name(&"x".repeat(80)).len(), 32);
    }
}
