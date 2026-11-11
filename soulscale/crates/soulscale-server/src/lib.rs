//! Soulscale server library (the binary in main.rs is a thin wrapper).
//!
//! * Serves the built client (`client/dist`) that Discord loads inside the Activity iframe.
//! * `POST /api/token`: exchanges the Embedded App SDK `authorize` code for an access token
//!   (the client secret never leaves this process).
//! * `GET /ws`: authoritative matches over WebSocket, one room per Activity instance.
//! * Writes a playtest report, round CSV and replay JSON to `data/replays/` for every
//!   finished match.

pub mod clock;
pub mod config;
pub mod discord;
pub mod rooms;
pub mod ws;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use soulscale_core::CardDb;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};

pub use crate::config::Config;
use crate::rooms::{Room, SaveJob};

pub struct AppState {
    pub cfg: Config,
    pub http: reqwest::Client,
    pub db: &'static CardDb,
    rooms: Mutex<HashMap<String, Arc<Mutex<Room>>>>,
}

impl AppState {
    pub fn room(&self, id: &str) -> Arc<Mutex<Room>> {
        let mut rooms = self.rooms.lock().expect("rooms lock");
        rooms.entry(id.to_string()).or_insert_with(|| Arc::new(Mutex::new(Room::new(id.to_string(), self.db)))).clone()
    }

    /// Write a finished match's files without blocking the room.
    pub fn persist(&self, job: SaveJob) {
        let dir = self.cfg.data_dir.join("replays");
        tokio::task::spawn_blocking(move || {
            let write = || -> std::io::Result<()> {
                std::fs::create_dir_all(&dir)?;
                std::fs::write(dir.join(format!("{}.txt", job.basename)), job.report)?;
                std::fs::write(dir.join(format!("{}.rounds.csv", job.basename)), job.csv)?;
                std::fs::write(dir.join(format!("{}.replay.json", job.basename)), job.replay)?;
                Ok(())
            };
            match write() {
                Ok(()) => tracing::info!(file = %job.basename, "match saved"),
                Err(e) => tracing::warn!(error = %e, "could not save match files"),
            }
        });
    }

    fn tick(&self) {
        let now = Instant::now();
        let rooms: Vec<(String, Arc<Mutex<Room>>)> =
            self.rooms.lock().expect("rooms lock").iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let mut dead = Vec::new();
        for (id, room) in rooms {
            let mut r = room.lock().expect("room lock");
            if let Some(job) = r.tick(now, self.cfg.reconnect_grace, self.cfg.turn_timeout) {
                self.persist(job);
            }
            // Forget empty rooms: quickly if idle, after 2h if a match was abandoned mid-way.
            if let Some(since) = r.empty_since {
                let ttl = if r.is_idle() { Duration::from_secs(15 * 60) } else { Duration::from_secs(2 * 3600) };
                if now.duration_since(since) > ttl {
                    dead.push(id);
                }
            }
        }
        if !dead.is_empty() {
            let mut rooms = self.rooms.lock().expect("rooms lock");
            for id in dead {
                rooms.remove(&id);
            }
        }
    }
}

#[derive(Deserialize)]
struct TokenReq {
    code: String,
}

#[derive(Serialize)]
struct TokenResp {
    access_token: String,
}

async fn token(State(app): State<Arc<AppState>>, Json(req): Json<TokenReq>) -> Response {
    if !app.cfg.discord_ready() {
        return (StatusCode::SERVICE_UNAVAILABLE, "Discord OAuth is not configured on this server").into_response();
    }
    if req.code.is_empty() || req.code.len() > 512 {
        return (StatusCode::BAD_REQUEST, "bad code").into_response();
    }
    match discord::exchange_code(&app.http, &app.cfg, &req.code).await {
        Ok(access_token) => Json(TokenResp { access_token }).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "token exchange failed");
            (StatusCode::BAD_GATEWAY, "token exchange failed").into_response()
        }
    }
}

#[derive(Serialize)]
struct PublicConfig {
    client_id: Option<String>,
    dev_login: bool,
    rules_version: &'static str,
}

async fn public_config(State(app): State<Arc<AppState>>) -> Json<PublicConfig> {
    Json(PublicConfig {
        client_id: app.cfg.client_id.clone(),
        dev_login: app.cfg.dev_login,
        rules_version: soulscale_core::RULES_VERSION,
    })
}

async fn health() -> &'static str {
    "ok"
}

/// Hashed build assets cache forever; everything else must revalidate so a redeploy
/// shows up on the next Activity launch.
async fn cache_headers(req: Request, next: Next) -> Response {
    let immutable = req.uri().path().contains("/assets/");
    let mut res = next.run(req).await;
    let value = if immutable { "public, max-age=31536000, immutable" } else { "no-cache" };
    res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    res.headers_mut().insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    res
}

/// Build the shared state (Discord HTTP client, card DB, empty room table).
pub fn state(cfg: Config) -> anyhow::Result<Arc<AppState>> {
    Ok(Arc::new(AppState {
        http: discord::http_client()?,
        db: soulscale_core::db(),
        rooms: Mutex::new(HashMap::new()),
        cfg,
    }))
}

/// The HTTP router: API, WebSocket and the static client.
pub fn router(state: Arc<AppState>) -> Router {
    let index = state.cfg.static_dir.join("index.html");
    let statics = ServeDir::new(&state.cfg.static_dir).fallback(ServeFile::new(index));
    Router::new()
        .route("/api/token", post(token))
        .route("/api/config", get(public_config))
        .route("/api/health", get(health))
        .route("/ws", get(ws::ws_handler))
        .fallback_service(statics)
        .layer(middleware::from_fn(cache_headers))
        .layer(CompressionLayer::new())
        .with_state(state)
}

/// Start the 1-second housekeeping ticker (disconnect takeover, idle auto-play, cleanup).
pub fn spawn_ticker(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut iv = tokio::time::interval(Duration::from_secs(1));
        loop {
            iv.tick().await;
            state.tick();
        }
    });
}

/// Install the TLS crypto provider (ring) used for calls to Discord.
pub fn install_crypto() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
