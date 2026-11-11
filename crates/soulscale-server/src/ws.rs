//! WebSocket connections: authenticate with `hello`, join the instance's room, then pump
//! client messages into the room and room messages back out.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use soulscale_core::protocol::{ClientMsg, ServerMsg, UserInfo};
use soulscale_core::view::Catalog;
use tokio::sync::mpsc;

use crate::AppState;
use crate::discord;
use crate::rooms::{Conn, OUTBOX};

static NEXT_CONN: AtomicU64 = AtomicU64::new(1);

const MAX_MESSAGE: usize = 32 * 1024;
const HELLO_TIMEOUT: Duration = Duration::from_secs(20);
/// Token bucket: sustained 10 msgs/s, bursts of 40.
const BUCKET_MAX: f64 = 40.0;
const BUCKET_RATE: f64 = 10.0;

pub async fn ws_handler(ws: WebSocketUpgrade, State(app): State<Arc<AppState>>) -> Response {
    ws.max_message_size(MAX_MESSAGE).max_frame_size(MAX_MESSAGE).on_upgrade(move |socket| run(socket, app))
}

fn encode(msg: &ServerMsg) -> String {
    serde_json::to_string(msg).unwrap_or_else(|_| r#"{"t":"error","message":"encoding failed"}"#.into())
}

async fn authenticate(app: &AppState, hello: ClientMsg) -> Result<(UserInfo, String, bool), String> {
    let ClientMsg::Hello { token, dev_name, instance } = hello else {
        return Err("The first message must be hello.".into());
    };
    let instance = instance.trim().to_string();
    if instance.is_empty() || instance.len() > 128 {
        return Err("Missing or invalid instance id.".into());
    }
    if let Some(token) = token.filter(|t| !t.is_empty()) {
        let user = discord::fetch_user(&app.http, &token).await.map_err(|e| e.to_string())?;
        match discord::verify_instance(&app.http, &app.cfg, &instance, &user.id).await {
            Ok(true) => {}
            Ok(false) => return Err("You are not in that Activity instance.".into()),
            Err(e) => {
                // Don't lock people out because a lookup hiccuped; log it instead.
                tracing::warn!(error = %e, "instance verification failed; allowing");
            }
        }
        return Ok((user, instance, false));
    }
    if app.cfg.dev_login {
        let name = discord::clean_name(dev_name.as_deref().unwrap_or("Tester"));
        let id = format!("dev:{}", name.to_lowercase());
        // Dev rooms are namespaced so a dev client can never land in a real Discord room.
        return Ok((UserInfo { id, name, avatar: None }, format!("dev:{instance}"), true));
    }
    Err("Sign-in required: open Soulscale from inside Discord.".into())
}

async fn run(socket: WebSocket, app: Arc<AppState>) {
    let (mut sink, mut stream) = socket.split();

    // 1) hello
    let first = tokio::time::timeout(HELLO_TIMEOUT, stream.next()).await;
    let hello = match first {
        Ok(Some(Ok(Message::Text(t)))) => serde_json::from_str::<ClientMsg>(t.as_str()).ok(),
        _ => None,
    };
    let Some(hello) = hello else {
        let _ = sink.send(Message::Text(encode(&ServerMsg::Error { message: "Expected hello.".into() }).into())).await;
        return;
    };
    let (user, instance, dev) = match authenticate(&app, hello).await {
        Ok(v) => v,
        Err(message) => {
            let _ = sink.send(Message::Text(encode(&ServerMsg::Error { message }).into())).await;
            return;
        }
    };
    let welcome = ServerMsg::Welcome {
        user: user.clone(),
        catalog: Catalog::from_db(app.db),
        server: env!("CARGO_PKG_VERSION").into(),
        dev,
    };
    if sink.send(Message::Text(encode(&welcome).into())).await.is_err() {
        return;
    }

    // 2) join room, start writer
    let conn_id = NEXT_CONN.fetch_add(1, Ordering::Relaxed);
    let (tx, mut rx) = mpsc::channel::<String>(OUTBOX);
    let room = app.room(&instance);
    room.lock().expect("room lock").join(conn_id, Conn { user: user.clone(), tx: tx.clone() });
    // No names or Discord IDs in the log: the privacy policy promises technical logs only.
    tracing::info!(conn_id, room = %instance, "connected");

    let writer = tokio::spawn(async move {
        while let Some(text) = rx.recv().await {
            if sink.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    });

    // 3) read loop
    let mut tokens = BUCKET_MAX;
    let mut last = Instant::now();
    while let Some(Ok(msg)) = stream.next().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };
        let now = Instant::now();
        tokens = (tokens + now.duration_since(last).as_secs_f64() * BUCKET_RATE).min(BUCKET_MAX);
        last = now;
        if tokens < 1.0 {
            let _ = tx.try_send(encode(&ServerMsg::Error { message: "Slow down a little.".into() }));
            continue;
        }
        tokens -= 1.0;
        let parsed = match serde_json::from_str::<ClientMsg>(text.as_str()) {
            Ok(m) => m,
            Err(e) => {
                let _ = tx.try_send(encode(&ServerMsg::Error { message: format!("Bad message: {e}") }));
                continue;
            }
        };
        let job = room.lock().expect("room lock").handle(conn_id, parsed);
        if let Some(job) = job {
            app.persist(job);
        }
        if tx.is_closed() {
            break;
        }
    }

    room.lock().expect("room lock").leave(conn_id);
    drop(tx);
    writer.abort();
    tracing::info!(conn_id, "disconnected");
}
