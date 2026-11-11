//! Rooms (one per Discord Activity instance) and the single game table each one hosts.
//!
//! * Everyone in the instance joins the same room and sees the table.
//! * A table is Empty, an Open Duel waiting for an opponent, or a Live match.
//! * Live matches are authoritative here; clients only send intents.
//! * Duel robustness: a disconnected player's seat passes to the AI after a grace period
//!   (and back when they return); an idle player has the AI play that round for them.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use soulscale_core::bot::bot_act;
use soulscale_core::deck::legality;
use soulscale_core::events::GameEvent;
use soulscale_core::protocol::{
    ClientMsg, DeckChoice, NewGame, ParticipantInfo, Role, RoomInfo, ServerMsg, TableInfo, UserInfo,
};
use soulscale_core::rng::PyRandom;
use soulscale_core::session::{Controller, Input, SeatStatus, Session, Vis};
use soulscale_core::{
    AiProfile, CardDb, RulesVersion, Choice, Faction, Game, GameConfig, GameMode, GOAL_DEEP, GOAL_QUICK, GOAL_STANDARD, NamedDeck,
    Seat,
};
use tokio::sync::mpsc;

use crate::clock::{SystemClock, stamp_file, stamp_human};

pub type ConnId = u64;

const DEFAULT_PROFILE: &str = "Balanced / Tactical";
const OPEN_DUEL_TTL: Duration = Duration::from_secs(30 * 60);
pub const OUTBOX: usize = 256;

pub struct Conn {
    pub user: UserInfo,
    pub tx: mpsc::Sender<String>,
}

/// Files to write when a match ends (done off the lock by the caller).
#[derive(Debug)]
pub struct SaveJob {
    pub basename: String,
    pub report: String,
    pub csv: String,
    pub replay: String,
}

#[derive(Clone, Debug)]
struct Setup {
    mode: GameMode,
    p1_faction: Faction,
    p1_deck: NamedDeck,
    p2_deck: NamedDeck,
    goal: i32,
    profile: AiProfile,
    rules: RulesVersion,
}

struct OpenDuel {
    host: UserInfo,
    side: Faction,
    deck: NamedDeck,
    goal: i32,
    profile: AiProfile,
    rules: RulesVersion,
    since: Instant,
}

struct Live {
    session: Session,
    users: [Option<UserInfo>; 2],
    setup: Setup,
    absent_since: [Option<Instant>; 2],
    waiting_since: [Option<Instant>; 2],
    taken_over: [bool; 2],
    saved: bool,
}

// One per room, and the open-duel variant is short-lived: not worth a Box.
#[allow(clippy::large_enum_variant)]
enum Table {
    Empty,
    Open(OpenDuel),
    Live(Box<Live>),
}

pub struct Room {
    pub id: String,
    conns: HashMap<ConnId, Conn>,
    table: Table,
    pub empty_since: Option<Instant>,
    db: &'static CardDb,
    picker: PyRandom,
}

#[derive(Serialize)]
struct ReplayFile<'a> {
    rules_version: RulesVersion,
    seed: u64,
    mode: GameMode,
    p1_faction: Faction,
    p1_deck: DeckOut,
    p2_deck: DeckOut,
    goal: i32,
    ai_profile: &'a str,
    names: &'a [String; 2],
    inputs: &'a [Input],
    final_soul: i32,
    rounds: u32,
    winner: Option<Faction>,
    log: &'a [String],
}

#[derive(Serialize)]
struct DeckOut {
    name: String,
    cards: Vec<String>,
}

pub fn fresh_seed() -> u64 {
    let mut b = [0u8; 8];
    // If the OS RNG is unavailable fall back to the clock; seeds only need to vary.
    if getrandom::fill(&mut b).is_err() {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        return t.as_nanos() as u64;
    }
    u64::from_le_bytes(b)
}

impl Room {
    pub fn new(id: String, db: &'static CardDb) -> Room {
        Room { id, conns: HashMap::new(), table: Table::Empty, empty_since: Some(Instant::now()), db, picker: PyRandom::new(fresh_seed()) }
    }

    pub fn is_idle(&self) -> bool {
        match &self.table {
            Table::Live(l) => l.session.game.is_over(),
            _ => true,
        }
    }

    // ------------------------------------------------------------- sending

    fn send_to(&self, id: ConnId, msg: &ServerMsg) {
        if let Some(c) = self.conns.get(&id)
            && let Ok(s) = serde_json::to_string(msg)
        {
            // A full outbox means the client stopped reading; the ws task will notice.
            let _ = c.tx.try_send(s);
        }
    }

    fn send_all(&self, msg: &ServerMsg) {
        if let Ok(s) = serde_json::to_string(msg) {
            for c in self.conns.values() {
                let _ = c.tx.try_send(s.clone());
            }
        }
    }

    fn role_of(&self, user: &UserInfo) -> Role {
        match &self.table {
            Table::Live(l) => {
                let is = |s: usize| l.users[s].as_ref().is_some_and(|u| u.id == user.id);
                match (is(0), is(1)) {
                    (true, true) => Role::Both,
                    (true, false) => Role::P1,
                    (false, true) => Role::P2,
                    _ => Role::Spectator,
                }
            }
            Table::Open(o) if o.host.id == user.id => Role::Host,
            _ => Role::Spectator,
        }
    }

    pub fn room_info(&self) -> RoomInfo {
        let mut seen: Vec<ParticipantInfo> = Vec::new();
        for c in self.conns.values() {
            if !seen.iter().any(|p| p.user.id == c.user.id) {
                seen.push(ParticipantInfo { user: c.user.clone(), role: self.role_of(&c.user), connected: true });
            }
        }
        // Seated players who dropped out still show (greyed) so others know who's who.
        if let Table::Live(l) = &self.table {
            for u in l.users.iter().flatten() {
                if !seen.iter().any(|p| p.user.id == u.id) {
                    seen.push(ParticipantInfo { user: u.clone(), role: self.role_of(u), connected: false });
                }
            }
        }
        seen.sort_by(|a, b| a.user.name.cmp(&b.user.name));
        let table = match &self.table {
            Table::Empty => TableInfo::Empty,
            Table::Open(o) => TableInfo::Open { host: o.host.clone(), side: o.side, deck: o.deck.name.clone(), goal: o.goal, rules: o.rules },
            Table::Live(l) => TableInfo::Live {
                mode: l.session.mode(),
                seats: l.users.clone(),
                round: l.session.game.st.round,
                over: l.session.game.is_over(),
                rules: l.setup.rules,
            },
        };
        RoomInfo { participants: seen, table }
    }

    fn broadcast_room(&self) {
        self.send_all(&ServerMsg::Room { room: self.room_info() });
    }

    /// Per-connection filtered events + redacted snapshot.
    fn broadcast_game(&self, events: &[(Vis, GameEvent)]) {
        let Table::Live(l) = &self.table else { return };
        for (&id, c) in &self.conns {
            let viewer = self.role_of(&c.user).viewer();
            let evs: Vec<GameEvent> = events.iter().filter(|(v, _)| viewer.sees(*v)).map(|(_, e)| e.clone()).collect();
            self.send_to(id, &ServerMsg::Game { view: l.session.view(viewer), events: evs });
        }
    }

    fn snapshot_to(&self, id: ConnId) {
        self.send_to(id, &ServerMsg::Room { room: self.room_info() });
        if let (Table::Live(l), Some(c)) = (&self.table, self.conns.get(&id)) {
            let viewer = self.role_of(&c.user).viewer();
            self.send_to(id, &ServerMsg::Game { view: l.session.view(viewer), events: Vec::new() });
        }
    }

    fn error(&self, id: ConnId, message: impl Into<String>) {
        self.send_to(id, &ServerMsg::Error { message: message.into() });
    }

    fn notice(&self, text: impl Into<String>) {
        self.send_all(&ServerMsg::Notice { text: text.into() });
    }

    // ------------------------------------------------------- membership

    pub fn join(&mut self, id: ConnId, conn: Conn) {
        self.conns.insert(id, conn);
        self.empty_since = None;
        self.broadcast_room();
        self.snapshot_to(id);
    }

    pub fn leave(&mut self, id: ConnId) {
        self.conns.remove(&id);
        if self.conns.is_empty() {
            self.empty_since = Some(Instant::now());
        }
        self.broadcast_room();
    }

    fn user_of(&self, id: ConnId) -> Option<UserInfo> {
        self.conns.get(&id).map(|c| c.user.clone())
    }

    fn present(&self, user_id: &str) -> bool {
        self.conns.values().any(|c| c.user.id == user_id)
    }

    // ---------------------------------------------------------- decks

    fn resolve_deck(&mut self, choice: &DeckChoice, f: Faction, profile_label: &str) -> Result<NamedDeck, String> {
        let db = self.db;
        match choice {
            DeckChoice::Builtin { name } => db
                .deck(f, name)
                .map(|d| NamedDeck { name: d.name.clone(), cards: d.cards.clone() })
                .ok_or_else(|| format!("No built-in {} deck called {name:?}.", f.short())),
            DeckChoice::Custom { name, cards } => {
                let mut ids = Vec::with_capacity(cards.len());
                for k in cards {
                    let id = db.id(k).ok_or_else(|| format!("Unknown glyph {k:?}."))?;
                    if db.card(id).faction != f {
                        return Err(format!("{} does not belong to {}.", db.card(id).short(), f.short()));
                    }
                    ids.push(id);
                }
                let leg = legality(db, &ids);
                if !leg.is_legal() {
                    return Err(format!("Deck is not legal: {}", leg.issues.join("; ")));
                }
                let clean: String = name.chars().filter(|c| !c.is_control()).take(40).collect();
                let clean = clean.trim();
                Ok(NamedDeck { name: format!("Custom: {}", if clean.is_empty() { "Deck" } else { clean }), cards: ids })
            }
            DeckChoice::Random => {
                // Python `choose_ai_deck_name`.
                let preferred = match (profile_label, f) {
                    ("Architect / Stack Builder", Faction::Heaven) => Some("Sanctuary Control"),
                    ("Architect / Stack Builder", Faction::Hell) => Some("Despair Control"),
                    ("Cruel Accountant / Hard" | "Chaos Imp / Weird", Faction::Heaven) => Some("Hope From Behind"),
                    ("Cruel Accountant / Hard" | "Chaos Imp / Weird", Faction::Hell) => Some("Trickster Hell"),
                    _ => None,
                };
                let d = match preferred.and_then(|n| db.deck(f, n)) {
                    Some(d) => d,
                    None => {
                        let all: Vec<_> = db.decks(f).collect();
                        all[self.picker.choice_index(all.len())]
                    }
                };
                Ok(NamedDeck { name: d.name.clone(), cards: d.cards.clone() })
            }
        }
    }

    fn profile(&self, label: Option<&str>) -> Result<AiProfile, String> {
        let label = label.unwrap_or(DEFAULT_PROFILE);
        self.db.profile(label).cloned().ok_or_else(|| format!("Unknown AI personality {label:?}."))
    }

    // ------------------------------------------------------------ table

    fn start_live(&mut self, setup: Setup, users: [Option<UserInfo>; 2]) {
        let names = match setup.mode {
            GameMode::Duel => [
                users[0].as_ref().map(|u| u.name.clone()).unwrap_or_else(|| "Player 1".into()),
                users[1].as_ref().map(|u| u.name.clone()).unwrap_or_else(|| "Player 2".into()),
            ],
            m => GameConfig::default_names(m),
        };
        let cfg = GameConfig {
            mode: setup.mode,
            p1_faction: setup.p1_faction,
            p1_deck: setup.p1_deck.clone(),
            p2_deck: setup.p2_deck.clone(),
            goal: setup.goal,
            ai_profile: setup.profile.clone(),
            names,
            rules: setup.rules,
        };
        let controllers = match setup.mode {
            GameMode::VsComputer => [Controller::Human, Controller::Ai],
            _ => [Controller::Human, Controller::Human],
        };
        let seed = fresh_seed();
        let game = Game::new(self.db, cfg, seed, Arc::new(SystemClock));
        let mut session = Session::start(game, controllers, seed);
        let events = session.drain();
        tracing::info!(room = %self.id, mode = ?setup.mode, seed, "match started");
        self.table = Table::Live(Box::new(Live {
            session,
            users,
            setup,
            absent_since: [None; 2],
            waiting_since: [None; 2],
            taken_over: [false; 2],
            saved: false,
        }));
        self.broadcast_room();
        self.broadcast_game(&events);
        self.refresh_waiting(Instant::now());
    }

    fn refresh_waiting(&mut self, now: Instant) {
        if let Table::Live(l) = &mut self.table {
            for s in Seat::BOTH {
                let needs = matches!(l.session.status(s), SeatStatus::Choose | SeatStatus::Reroll { .. })
                    && l.session.controllers[s.idx()] == Controller::Human;
                if !needs {
                    l.waiting_since[s.idx()] = None;
                } else if l.waiting_since[s.idx()].is_none() {
                    l.waiting_since[s.idx()] = Some(now);
                }
            }
        }
    }

    fn take_save_job(&mut self) -> Option<SaveJob> {
        let Table::Live(l) = &mut self.table else { return None };
        if l.saved || !l.session.game.is_over() {
            return None;
        }
        l.saved = true;
        Some(save_job(l))
    }

    /// Handle one client message. Returns files to persist, if a match just ended.
    pub fn handle(&mut self, id: ConnId, msg: ClientMsg) -> Option<SaveJob> {
        let user = self.user_of(id)?;
        match msg {
            ClientMsg::Hello { .. } => self.error(id, "Already connected."),
            ClientMsg::Ping { n } => self.send_to(id, &ServerMsg::Pong { n }),
            ClientMsg::NewGame { game } => {
                if let Err(e) = self.new_game(&user, game) {
                    self.error(id, e);
                }
            }
            ClientMsg::JoinDuel { deck } => {
                if let Err(e) = self.join_duel(&user, &deck) {
                    self.error(id, e);
                }
            }
            ClientMsg::CancelOpen => match &self.table {
                Table::Open(o) if o.host.id == user.id => {
                    self.table = Table::Empty;
                    self.broadcast_room();
                }
                _ => self.error(id, "There is no open Duel of yours to cancel."),
            },
            ClientMsg::Choose { seat, card, chamber, intent } => {
                let res = self.with_seat(&user, seat, |s| {
                    let card = s.card_id(&card)?;
                    s.choose(seat, Choice { card, chamber, intent })
                });
                if let Err(e) = res {
                    self.error(id, e);
                }
            }
            ClientMsg::Reroll { seat, reroll } => {
                if let Err(e) = self.with_seat(&user, seat, |s| s.decide_reroll(seat, reroll)) {
                    self.error(id, e);
                }
            }
            ClientMsg::Resign => return self.resign(id, &user),
            ClientMsg::Rematch => {
                let setup_users = match &self.table {
                    Table::Live(l) if l.session.game.is_over() && l.users.iter().flatten().any(|u| u.id == user.id) => {
                        Some((l.setup.clone(), l.users.clone()))
                    }
                    Table::Live(_) => None,
                    _ => None,
                };
                match setup_users {
                    Some((setup, users)) => self.start_live(setup, users),
                    None => self.error(id, "Rematch is available to the players once a match has ended."),
                }
            }
            ClientMsg::Export => match &self.table {
                Table::Live(l) => {
                    let job = save_job(l);
                    self.send_to(
                        id,
                        &ServerMsg::Export { basename: job.basename, report: job.report, csv: job.csv, replay: job.replay },
                    );
                }
                _ => self.error(id, "No match to export yet."),
            },
        }
        self.refresh_waiting(Instant::now());
        self.take_save_job()
    }

    fn with_seat(
        &mut self,
        user: &UserInfo,
        seat: Seat,
        f: impl FnOnce(&mut Session) -> Result<(), soulscale_core::session::SessionError>,
    ) -> Result<(), String> {
        let Table::Live(l) = &mut self.table else { return Err("No match is running.".into()) };
        if !l.users[seat.idx()].as_ref().is_some_and(|u| u.id == user.id) {
            return Err("That seat isn't yours.".into());
        }
        f(&mut l.session).map_err(|e| e.to_string())?;
        let events = l.session.drain();
        self.broadcast_game(&events);
        if let Table::Live(l) = &self.table
            && l.session.game.is_over()
        {
            self.broadcast_room();
        }
        Ok(())
    }

    fn table_busy_for(&self, user: &UserInfo) -> Option<&'static str> {
        match &self.table {
            Table::Live(l) if !l.session.game.is_over() => {
                if l.users.iter().flatten().any(|u| u.id == user.id) {
                    Some("Finish or resign your current match first.")
                } else {
                    Some("A match is in progress at this table. Spectate, or wait for it to finish.")
                }
            }
            Table::Open(o) if o.host.id != user.id => {
                Some("A Duel is waiting for a challenger. Join it, or wait for the host to cancel.")
            }
            _ => None,
        }
    }

    fn new_game(&mut self, user: &UserInfo, g: NewGame) -> Result<(), String> {
        if let Some(why) = self.table_busy_for(user) {
            return Err(why.into());
        }
        if ![GOAL_QUICK, GOAL_STANDARD, GOAL_DEEP].contains(&g.goal) {
            return Err("Goal must be 12, 18 or 24.".into());
        }
        let profile = self.profile(g.ai_profile.as_deref())?;
        let p1_deck = self.resolve_deck(&g.deck, g.side, &profile.label)?;
        let rules = g.rules.unwrap_or(RulesVersion::LATEST);
        if g.mode == GameMode::Duel {
            self.table = Table::Open(OpenDuel {
                host: user.clone(),
                side: g.side,
                deck: p1_deck,
                goal: g.goal,
                profile,
                rules,
                since: Instant::now(),
            });
            self.broadcast_room();
            self.notice(format!("{} opened a Duel as {}. Anyone can take the other side.", user.name, g.side.short()));
            return Ok(());
        }
        let opp_choice = g.opponent_deck.clone().unwrap_or(DeckChoice::Random);
        let p2_deck = self.resolve_deck(&opp_choice, g.side.opponent(), &profile.label)?;
        let setup = Setup { mode: g.mode, p1_faction: g.side, p1_deck, p2_deck, goal: g.goal, profile, rules };
        let users = match g.mode {
            GameMode::VsComputer => [Some(user.clone()), None],
            _ => [Some(user.clone()), Some(user.clone())],
        };
        self.start_live(setup, users);
        Ok(())
    }

    fn join_duel(&mut self, user: &UserInfo, deck: &DeckChoice) -> Result<(), String> {
        let Table::Open(o) = &self.table else { return Err("There is no open Duel to join.".into()) };
        if o.host.id == user.id {
            return Err("You can't duel yourself. Share the Activity with a friend!".into());
        }
        let (host, side, p1_deck, goal, profile, rules) =
            (o.host.clone(), o.side, o.deck.clone(), o.goal, o.profile.clone(), o.rules);
        let p2_deck = self.resolve_deck(deck, side.opponent(), &profile.label)?;
        let setup = Setup { mode: GameMode::Duel, p1_faction: side, p1_deck, p2_deck, goal, profile, rules };
        self.notice(format!("{} accepted {}'s Duel.", user.name, host.name));
        self.start_live(setup, [Some(host), Some(user.clone())]);
        Ok(())
    }

    fn resign(&mut self, id: ConnId, user: &UserInfo) -> Option<SaveJob> {
        let Table::Live(l) = &mut self.table else {
            self.error(id, "No match to resign from.");
            return None;
        };
        if !l.users.iter().flatten().any(|u| u.id == user.id) {
            self.error(id, "Only the players can resign.");
            return None;
        }
        if l.session.game.is_over() {
            self.table = Table::Empty;
            self.broadcast_room();
            return None;
        }
        l.session.game.log(format!("{} resigned.", user.name));
        let job = if l.saved { None } else { Some(save_job(l)) };
        self.table = Table::Empty;
        self.notice(format!("{} resigned the match.", user.name));
        self.broadcast_room();
        job
    }

    /// Periodic housekeeping: Duel disconnect takeover/reclaim, idle auto-play, stale lobbies.
    pub fn tick(&mut self, now: Instant, grace: Duration, turn_timeout: Duration) -> Option<SaveJob> {
        let mut notices: Vec<String> = Vec::new();
        let mut events: Vec<(Vis, GameEvent)> = Vec::new();
        let mut room_changed = false;
        let present: [bool; 2] = match &self.table {
            Table::Live(l) => Seat::BOTH.map(|s| l.users[s.idx()].as_ref().is_some_and(|u| self.present(&u.id))),
            _ => [false; 2],
        };
        if matches!(&self.table, Table::Open(o) if now.duration_since(o.since) > OPEN_DUEL_TTL) {
            self.table = Table::Empty;
            room_changed = true;
        }
        match &mut self.table {
            Table::Live(l) if l.session.mode() == GameMode::Duel && !l.session.game.is_over() => {
                let profile = l.setup.profile.clone();
                for s in Seat::BOTH {
                    let i = s.idx();
                    let name = l.users[i].as_ref().map(|u| u.name.clone()).unwrap_or_default();
                    if present[i] {
                        l.absent_since[i] = None;
                        if l.taken_over[i] {
                            l.taken_over[i] = false;
                            l.session.set_controller(s, Controller::Human);
                            notices.push(format!("{name} is back and has their seat again."));
                            room_changed = true;
                        }
                    } else {
                        let since = *l.absent_since[i].get_or_insert(now);
                        if !l.taken_over[i] && now.duration_since(since) >= grace {
                            l.taken_over[i] = true;
                            l.session.set_controller(s, Controller::Ai);
                            notices.push(format!("{name} disconnected. The AI is holding their seat until they return."));
                            room_changed = true;
                        }
                    }
                    if l.session.controllers[i] == Controller::Human
                        && let Some(w) = l.waiting_since[i]
                        && now.duration_since(w) >= turn_timeout
                    {
                        // Up to two steps: choose, then the reroll decision.
                        for _ in 0..2 {
                            if !matches!(bot_act(&mut l.session, s, &profile), Ok(true)) {
                                break;
                            }
                        }
                        l.waiting_since[i] = None;
                        notices.push(format!("{name} ran out of time, so the AI played this round for them."));
                    }
                }
                events = l.session.drain();
            }
            _ => {}
        }
        for n in notices {
            self.notice(n);
        }
        if !events.is_empty() {
            self.broadcast_game(&events);
        }
        if room_changed {
            self.broadcast_room();
        }
        self.refresh_waiting(now);
        self.take_save_job()
    }
}

fn save_job(l: &Live) -> SaveJob {
    let g = &l.session.game;
    let db = g.db;
    let keys = |d: &NamedDeck| DeckOut { name: d.name.clone(), cards: d.cards.iter().map(|&c| db.card(c).key.clone()).collect() };
    let basename = g.export_basename(&stamp_file());
    let replay = ReplayFile {
        rules_version: l.setup.rules,
        seed: l.session.seed,
        mode: l.setup.mode,
        p1_faction: l.setup.p1_faction,
        p1_deck: keys(&l.setup.p1_deck),
        p2_deck: keys(&l.setup.p2_deck),
        goal: l.setup.goal,
        ai_profile: &l.setup.profile.label,
        names: &g.cfg.names,
        inputs: &l.session.inputs,
        final_soul: g.st.soul,
        rounds: g.st.round,
        winner: g.st.winner,
        log: &g.log_lines,
    };
    SaveJob {
        basename,
        report: g.playtest_report(&stamp_human()),
        csv: g.rounds_csv(),
        replay: serde_json::to_string_pretty(&replay).unwrap_or_default(),
    }
}
