//! End-to-end: boot the real router on a random port and play over real WebSockets.

use std::path::PathBuf;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use soulscale_core::protocol::{ClientMsg, DeckChoice, NewGame, ServerMsg, TableInfo};
use soulscale_core::session::SeatStatus;
use soulscale_core::view::{Catalog, View};
use soulscale_core::{Chamber, Faction, GameMode, Intent, Seat};
use soulscale_server::{Config, router, spawn_ticker, state};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("soulscale-e2e-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

async fn boot(tag: &str, grace: u64) -> (String, PathBuf) {
    soulscale_server::install_crypto();
    let data = temp_dir(tag);
    let cfg = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        static_dir: data.join("no-client"),
        data_dir: data.clone(),
        client_id: None,
        client_secret: None,
        bot_token: None,
        dev_login: true,
        turn_timeout: Duration::from_secs(600),
        reconnect_grace: Duration::from_secs(grace),
    };
    let st = state(cfg).unwrap();
    spawn_ticker(st.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router(st)).await.unwrap() });
    (format!("ws://{addr}/ws"), data)
}

struct Client {
    ws: Ws,
    catalog: Option<Catalog>,
    view: Option<View>,
    table: Option<TableInfo>,
    errors: Vec<String>,
    games_seen: usize,
}

impl Client {
    async fn connect(url: &str, name: &str, instance: &str) -> Client {
        let (ws, _) = connect_async(url).await.unwrap();
        let mut c = Client { ws, catalog: None, view: None, table: None, errors: Vec::new(), games_seen: 0 };
        c.send(&ClientMsg::Hello { token: None, dev_name: Some(name.into()), instance: instance.into() }).await;
        c.pump_until(|c| c.catalog.is_some()).await;
        c
    }

    async fn send(&mut self, m: &ClientMsg) {
        self.ws.send(Message::Text(serde_json::to_string(m).unwrap().into())).await.unwrap();
    }

    /// Read messages until `done` holds (10s cap).
    async fn pump_until(&mut self, mut done: impl FnMut(&Client) -> bool) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while !done(self) {
            let msg = tokio::time::timeout_at(deadline, self.ws.next()).await.expect("timed out waiting").unwrap().unwrap();
            let Message::Text(t) = msg else { continue };
            match serde_json::from_str::<ServerMsg>(t.as_str()).unwrap() {
                ServerMsg::Welcome { catalog, .. } => self.catalog = Some(catalog),
                ServerMsg::Room { room } => self.table = Some(room.table),
                ServerMsg::Game { view, .. } => {
                    self.view = Some(view);
                    self.games_seen += 1;
                }
                ServerMsg::Error { message } => self.errors.push(message),
                _ => {}
            }
        }
    }

    /// Drain whatever is already queued (short wait).
    async fn settle(&mut self) {
        while let Ok(Some(Ok(Message::Text(t)))) = tokio::time::timeout(Duration::from_millis(150), self.ws.next()).await {
            match serde_json::from_str::<ServerMsg>(t.as_str()).unwrap() {
                ServerMsg::Room { room } => self.table = Some(room.table),
                ServerMsg::Game { view, .. } => self.view = Some(view),
                ServerMsg::Error { message } => self.errors.push(message),
                _ => {}
            }
        }
    }

    /// Take one legal action for `seat` if the view says it's needed. Returns whether it acted.
    async fn act(&mut self, seat: Seat) -> bool {
        let Some(v) = &self.view else { return false };
        let sv = &v.seats[seat.idx()];
        match sv.status {
            Some(SeatStatus::Choose) => {
                let (card, chamber) = if let Some(f) = &sv.forced {
                    (f.card.clone(), f.chamber)
                } else {
                    let cat = self.catalog.as_ref().unwrap();
                    let hand = sv.hand.clone().expect("own hand visible");
                    let mut pick = None;
                    for k in &hand {
                        let def = cat.cards.iter().find(|c| &c.key == k).unwrap();
                        if let Some(ch) = def.valid_chambers.iter().find(|c| !sv.locked_chambers.contains(c)) {
                            pick = Some((k.clone(), *ch));
                            break;
                        }
                    }
                    pick.unwrap()
                };
                self.send(&ClientMsg::Choose { seat, card, chamber, intent: Intent::Press }).await;
                true
            }
            Some(SeatStatus::Reroll { .. }) => {
                self.send(&ClientMsg::Reroll { seat, reroll: false }).await;
                true
            }
            _ => false,
        }
    }
}

fn builtin(name: &str) -> DeckChoice {
    DeckChoice::Builtin { name: name.into() }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn vs_computer_game_plays_to_the_end_and_saves_files() {
    let (url, data) = boot("ai", 30).await;
    let mut a = Client::connect(&url, "Alice", "inst-ai").await;
    a.send(&ClientMsg::NewGame {
        game: NewGame {
            mode: GameMode::VsComputer,
            side: Faction::Heaven,
            deck: builtin("Starter Mercy"),
            opponent_deck: Some(DeckChoice::Random),
            goal: 12,
            ai_profile: Some("Cruel Accountant / Hard".into()),
            rules: None,
        },
    })
    .await;
    a.pump_until(|c| c.view.is_some()).await;
    let v = a.view.as_ref().unwrap();
    assert!(v.seats[1].hand.is_some(), "vs Computer shows the AI's reveal, as v1.0.1 does");
    let mut steps = 0;
    while !a.view.as_ref().unwrap().over {
        let seen = a.games_seen;
        assert!(a.act(Seat::P1).await, "P1 should always have something to do");
        a.pump_until(|c| c.games_seen > seen || !c.errors.is_empty()).await;
        // The server rate-limits bursts; a real client just tries again.
        if a.errors.iter().all(|e| e.contains("Slow down")) && !a.errors.is_empty() {
            a.errors.clear();
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        steps += 1;
        assert!(steps < 4000, "game did not finish");
    }
    assert!(a.errors.is_empty(), "{:?}", a.errors);
    let v = a.view.as_ref().unwrap();
    assert!(v.summary.as_deref().unwrap().contains("ENDGAME SUMMARY"));
    // Files land asynchronously.
    let dir = data.join("replays");
    for _ in 0..50 {
        if std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0) >= 3 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let names: Vec<String> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    assert!(names.iter().any(|n| n.ends_with(".replay.json")), "{names:?}");
    assert!(names.iter().any(|n| n.ends_with(".rounds.csv")), "{names:?}");
    assert!(names.iter().any(|n| n.ends_with(".txt")), "{names:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duel_keeps_hands_secret_and_plays_out() {
    let (url, _) = boot("duel", 30).await;
    let mut a = Client::connect(&url, "Alice", "inst-duel").await;
    let mut b = Client::connect(&url, "Bob", "inst-duel").await;
    let mut spec = Client::connect(&url, "Carol", "inst-duel").await;

    a.send(&ClientMsg::NewGame {
        game: NewGame {
            mode: GameMode::Duel,
            side: Faction::Hell,
            deck: builtin("Pain Engine"),
            opponent_deck: None,
            goal: 12,
            ai_profile: None,
            rules: None,
        },
    })
    .await;
    b.pump_until(|c| matches!(c.table, Some(TableInfo::Open { .. }))).await;
    // Carol cannot start a second game while the Duel is open.
    spec.send(&ClientMsg::NewGame {
        game: NewGame { mode: GameMode::VsComputer, side: Faction::Heaven, deck: DeckChoice::Random, opponent_deck: None, goal: 12, ai_profile: None, rules: None },
    })
    .await;
    spec.pump_until(|c| !c.errors.is_empty()).await;

    b.send(&ClientMsg::JoinDuel { deck: builtin("Comfort Engine") }).await;
    a.pump_until(|c| c.view.is_some()).await;
    b.pump_until(|c| c.view.is_some()).await;
    spec.pump_until(|c| c.view.is_some()).await;

    // Alice is P1 (Hell), Bob P2 (Heaven).
    let va = a.view.as_ref().unwrap();
    assert_eq!(va.seats[0].faction, Faction::Hell);
    assert!(va.seats[0].hand.is_some() && va.seats[1].hand.is_none());
    let vb = b.view.as_ref().unwrap();
    assert!(vb.seats[1].hand.is_some() && vb.seats[0].hand.is_none());
    let vs = spec.view.as_ref().unwrap();
    assert!(vs.seats[0].hand.is_none() && vs.seats[1].hand.is_none());

    // Bob may not act for Alice's seat.
    b.send(&ClientMsg::Reroll { seat: Seat::P1, reroll: false }).await;
    b.pump_until(|c| c.errors.iter().any(|e| e.contains("isn't yours"))).await;

    let mut guard = 0;
    loop {
        a.settle().await;
        b.settle().await;
        if a.view.as_ref().unwrap().over {
            break;
        }
        let acted_a = a.act(Seat::P1).await;
        let acted_b = b.act(Seat::P2).await;
        if !acted_a && !acted_b {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        guard += 1;
        assert!(guard < 3000, "duel did not finish");
    }
    spec.settle().await;
    assert!(spec.view.as_ref().unwrap().over);
    assert_eq!(a.view.as_ref().unwrap().winner, b.view.as_ref().unwrap().winner);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnected_duelist_is_covered_by_the_ai() {
    let (url, _) = boot("drop", 1).await;
    let mut a = Client::connect(&url, "Alice", "inst-drop").await;
    let mut b = Client::connect(&url, "Bob", "inst-drop").await;
    a.send(&ClientMsg::NewGame {
        game: NewGame { mode: GameMode::Duel, side: Faction::Heaven, deck: builtin("Starter Mercy"), opponent_deck: None, goal: 24, ai_profile: None, rules: None },
    })
    .await;
    b.pump_until(|c| matches!(c.table, Some(TableInfo::Open { .. }))).await;
    b.send(&ClientMsg::JoinDuel { deck: builtin("Starter Torment") }).await;
    a.pump_until(|c| c.view.is_some()).await;
    b.pump_until(|c| c.view.is_some()).await;
    drop(b); // Bob vanishes.

    // Alice keeps playing; within the grace period + a tick, Bob's seat goes to the AI
    // and rounds advance without him.
    let start_round = a.view.as_ref().unwrap().round;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    while a.view.as_ref().unwrap().round < start_round + 3 && !a.view.as_ref().unwrap().over {
        assert!(tokio::time::Instant::now() < deadline, "rounds did not advance after disconnect");
        a.act(Seat::P1).await;
        a.settle().await;
    }
    let v = a.view.as_ref().unwrap();
    assert!(v.over || v.seats[1].controller == soulscale_core::session::Controller::Ai);
    let _ = Chamber::Body;
}
