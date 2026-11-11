//! The game: owns state, RNG, stats and the log, and runs the round pipeline.
//!
//! Turn order (rulebook §27 / Python `new_round` + `resolve_*`):
//!
//! ```text
//! begin_round: round++, Will, Chamber locks, delayed movement, reveal 4 + 4
//! (orchestrator) AI blind-commits / humans choose glyph + Chamber + Intent
//! (orchestrator) roll d6, optional reroll for 1 Will
//! resolve: strain from 6s -> pending plays -> pre-clash -> clash -> apply plays
//!          -> ongoing -> caps -> move Soul -> stats -> Purgatory events -> victory
//! ```
//!
//! The orchestrator (the session layer, the simulator, or the Python-parity harness)
//! decides *when* each step happens; this type guarantees *what* each step does, with
//! RNG calls and log lines in the same order as v1.0.1.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::carddb::CardDb;
use crate::deck::DeckState;
use crate::events::{GameEvent, PlayInfo};
use crate::model::{AiProfile, BASE_WILL, CardDef, CardId, Chamber, Faction, Intent, REVEAL_SIZE};
use crate::rng::PyRandom;
use crate::rules::intensity::intensity_from_d6;
use crate::state::{ChamberSet, GameState, Seat, SeatState};
use crate::stats::GameStats;

/// How the match is being played. Affects log wording and summaries only;
/// the rules are identical in every mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    /// Human (P1) vs the heuristic AI (P2). The AI commits blind at round start.
    VsComputer,
    /// Two humans, one screen, both hands visible.
    Hotseat,
    /// Two humans, one screen, hands and choices hidden until both commit.
    SecretHotseat,
    /// Two players on their own devices (Discord). Simultaneous secret commits.
    Duel,
}

impl GameMode {
    pub fn is_secret(self) -> bool {
        matches!(self, GameMode::SecretHotseat | GameMode::Duel)
    }
}

/// Wall-clock source for summary timestamps (core does no I/O, so callers inject it).
pub trait Clock: Send + Sync {
    fn now(&self) -> String;
}

/// A clock that always returns the same string (tests, parity harness, unknown time).
#[derive(Clone, Debug)]
pub struct FixedClock(pub String);

impl Clock for FixedClock {
    fn now(&self) -> String {
        self.0.clone()
    }
}

#[derive(Clone, Debug)]
pub struct NamedDeck {
    pub name: String,
    pub cards: Vec<CardId>,
}

#[derive(Clone, Debug)]
pub struct GameConfig {
    pub mode: GameMode,
    pub p1_faction: Faction,
    pub p1_deck: NamedDeck,
    pub p2_deck: NamedDeck,
    pub goal: i32,
    /// Personality used by any AI-controlled seat.
    pub ai_profile: AiProfile,
    /// Actor names used in the log ("You"/"Computer", "Player 1"/"Player 2", Discord names).
    pub names: [String; 2],
    /// Rules revision (v1.0.1 = exact Python behaviour; v1.0.2 = with fixes).
    pub rules: crate::model::RulesVersion,
}

impl GameConfig {
    /// Default actor names for a mode (Python `player_actor_name` / `opponent_actor_name`).
    pub fn default_names(mode: GameMode) -> [String; 2] {
        match mode {
            GameMode::VsComputer => ["You".into(), "Computer".into()],
            _ => ["Player 1".into(), "Player 2".into()],
        }
    }
}

/// One side's locked-in play, with its final (post-reroll) d6.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayCommit {
    pub card: CardId,
    pub chamber: Chamber,
    pub intent: Intent,
    pub raw: u8,
}

impl PlayCommit {
    pub fn intensity(&self) -> i32 {
        intensity_from_d6(self.raw)
    }
}

/// A glyph + Chamber + Intent choice before any dice are rolled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    pub card: CardId,
    pub chamber: Chamber,
    pub intent: Intent,
}

/// Python `PendingPlay`.
#[derive(Clone, Debug)]
pub struct PendingPlay {
    pub seat: Seat,
    pub card: CardId,
    pub chamber: Chamber,
    pub intent: Intent,
    pub raw: u8,
    pub intensity: i32,
    pub power: i32,
    pub amount: i32,
    pub stack_add: i32,
    pub stack_spend: i32,
    pub sever_amount: i32,
    pub resonance_bonus: i32,
    pub half: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PlayError {
    #[error("the game is over")]
    GameOver,
    #[error("that glyph is not in your revealed hand")]
    NotInHand,
    #[error("{card} cannot be played in {chamber}")]
    InvalidChamber { card: String, chamber: &'static str },
    #[error("{chamber} is Binding-locked for your side this round")]
    Locked { chamber: &'static str },
    #[error("Release needs a spend of at least 1 and at most your friendly stack here ({available})")]
    BadRelease { available: i32 },
    #[error("no Will left to reroll")]
    NoWill,
}

pub struct Game {
    pub cfg: GameConfig,
    pub db: &'static CardDb,
    pub rng: PyRandom,
    pub st: GameState,
    pub stats: GameStats,
    pub log_lines: Vec<String>,
    events: Vec<GameEvent>,
    clock: Arc<dyn Clock>,
}

impl std::fmt::Debug for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Game").field("round", &self.st.round).field("soul", &self.st.soul).finish_non_exhaustive()
    }
}

impl Game {
    /// Python `start_game` up to (not including) the welcome log: resets state and
    /// shuffles P1's deck, then P2's.
    pub fn new(db: &'static CardDb, cfg: GameConfig, seed: u64, clock: Arc<dyn Clock>) -> Game {
        let mut rng = PyRandom::new(seed);
        let stats = GameStats::new(clock.now());
        let p1_deck = DeckState::new(&cfg.p1_deck.cards, &mut rng);
        let p2_deck = DeckState::new(&cfg.p2_deck.cards, &mut rng);
        let mk = |faction: Faction, deck_name: &str, deck: DeckState| SeatState {
            faction,
            deck_name: deck_name.to_string(),
            deck,
            will: BASE_WILL,
            next_will_bonus: 0,
            next_will_penalty: 0,
        };
        let st = GameState {
            soul: 0,
            goal: cfg.goal,
            round: 0,
            strain: 0,
            seats: [
                mk(cfg.p1_faction, &cfg.p1_deck.name, p1_deck),
                mk(cfg.p1_faction.opponent(), &cfg.p2_deck.name, p2_deck),
            ],
            next_angel_bonus: 0,
            next_devil_bonus: 0,
            next_all_bonus: 0,
            next_heaven_cap: None,
            next_soul_cap: None,
            support_bonus: [0, 0],
            delayed_movements: Vec::new(),
            next_delayed_movements: Vec::new(),
            chamber_locks: [ChamberSet::default(); 2],
            next_chamber_locks: [ChamberSet::default(); 2],
            board: Default::default(),
            conditions: Default::default(),
            stacks: Default::default(),
            game_over: false,
            winner: None,
        };
        Game { cfg, db, rng, st, stats, log_lines: Vec::new(), events: Vec::new(), clock }
    }

    // ------------------------------------------------------------------ util

    pub fn card(&self, id: CardId) -> &'static CardDef {
        self.db.card(id)
    }

    pub fn name(&self, seat: Seat) -> &str {
        &self.cfg.names[seat.idx()]
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        let text = msg.into();
        self.log_lines.push(text.clone());
        self.events.push(GameEvent::Log { text });
    }

    pub(crate) fn emit(&mut self, e: GameEvent) {
        self.events.push(e);
    }

    /// Take every event emitted since the last drain.
    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn now(&self) -> String {
        self.clock.now()
    }

    pub fn seat_of(&self, f: Faction) -> Seat {
        self.st.seat_of(f)
    }

    // ----------------------------------------------------------------- start

    /// Welcome log + round 1 (Python `start_game` tail).
    pub fn start(&mut self) {
        self.log(format!("Welcome to Soulscale: Purgatory v{}.", self.cfg.rules.label()));
        let db = self.db;
        let sym = |d: &NamedDeck| db.symbols(&d.cards);
        let (p1, p2) = (self.cfg.p1_faction, self.cfg.p1_faction.opponent());
        let (n1, n2) = (self.cfg.names[0].clone(), self.cfg.names[1].clone());
        let (d1, d2) = (self.cfg.p1_deck.clone(), self.cfg.p2_deck.clone());
        match self.cfg.mode {
            GameMode::VsComputer => {
                self.log("Mode: Human vs Computer.");
                self.log(format!("{n1} are {} using {}: {}.", p1.label(), d1.name, sym(&d1)));
                self.log(format!("{n2} is {} using {}: {}.", p2.label(), d2.name, sym(&d2)));
                let p = self.cfg.ai_profile.clone();
                self.log(format!("AI personality: {} — {}", p.label, p.description));
            }
            GameMode::Hotseat | GameMode::SecretHotseat => {
                if self.cfg.mode == GameMode::SecretHotseat {
                    self.log("Mode: Human vs Human Secret Hot-seat. Player 2 hand and Player 1 choice are hidden until reveal.");
                } else {
                    self.log("Mode: Human vs Human open hot-seat. Player 1 chooses first each round; then pass the keyboard to Player 2.");
                }
                self.log(format!("{n1} is {} using {}: {}.", p1.label(), d1.name, sym(&d1)));
                self.log(format!("{n2} is {} using {}: {}.", p2.label(), d2.name, sym(&d2)));
            }
            GameMode::Duel => {
                self.log("Mode: Discord Duel. Both players choose in secret; plays reveal together.");
                self.log(format!("{n1} is {} using {}: {}.", p1.label(), d1.name, sym(&d1)));
                self.log(format!("{n2} is {} using {}: {}.", p2.label(), d2.name, sym(&d2)));
            }
        }
        self.log(format!(
            "Goal length: ±{}. Stack layer: Press, Anchor, Release, or Sever after choosing a Chamber.",
            self.st.goal
        ));
        self.begin_round();
    }

    /// Python `new_round` minus the UI and the AI commit (the orchestrator does that).
    pub fn begin_round(&mut self) {
        if self.st.game_over {
            return;
        }
        self.st.round += 1;
        for s in Seat::BOTH {
            let seat = self.st.seat_mut(s);
            seat.will = (BASE_WILL + seat.next_will_bonus - seat.next_will_penalty).max(0);
            seat.next_will_bonus = 0;
            seat.next_will_penalty = 0;
        }
        self.st.chamber_locks = self.st.next_chamber_locks;
        self.st.next_chamber_locks = [ChamberSet::default(); 2];
        let will = [self.st.seats[0].will, self.st.seats[1].will];
        self.emit(GameEvent::RoundStart { round: self.st.round, will });
        for f in Faction::ALL {
            for c in self.st.chamber_locks[f.idx()].iter() {
                self.emit(GameEvent::Locked { faction: f, chamber: c });
            }
        }

        self.apply_delayed_movements_at_round_start();
        if self.st.game_over {
            return;
        }

        let r1 = self.st.seats[0].deck.reveal(REVEAL_SIZE, &mut self.rng);
        let r2 = self.st.seats[1].deck.reveal(REVEAL_SIZE, &mut self.rng);
        let keys = |v: &[CardId]| v.iter().map(|&c| self.db.card(c).key.clone()).collect::<Vec<_>>();
        let (k1, k2) = (keys(&r1), keys(&r2));
        self.emit(GameEvent::Reveal { seat: Seat::P1, cards: k1 });
        self.emit(GameEvent::Reveal { seat: Seat::P2, cards: k2 });

        self.log("");
        self.log(format!("=== Round {} ===", self.st.round));
        match self.cfg.mode {
            GameMode::SecretHotseat => {
                self.log("Secret hot-seat reveal: both players' dealt glyphs are hidden from each other until played.")
            }
            GameMode::Duel => {
                self.log("Duel reveal: each player's dealt glyphs are hidden from the other until played.")
            }
            _ => {
                let shorts = |v: &[CardId]| v.iter().map(|&c| self.db.card(c).short()).collect::<Vec<_>>().join(", ");
                let (s1, s2) = (shorts(&r1), shorts(&r2));
                let (n1, n2) = (self.cfg.names[0].clone(), self.cfg.names[1].clone());
                self.log(format!("{n1} reveal: {s1}"));
                self.log(format!("{n2} reveal: {s2}"));
            }
        }
    }

    // ------------------------------------------------------------- choices

    pub fn revealed(&self, seat: Seat) -> &[CardId] {
        &self.st.seat(seat).deck.revealed
    }

    /// Validates a human choice against the hand, Chamber list, Binding locks and stack.
    pub fn validate_choice(&self, seat: Seat, ch: &Choice) -> Result<(), PlayError> {
        if self.st.game_over {
            return Err(PlayError::GameOver);
        }
        if !self.revealed(seat).contains(&ch.card) {
            return Err(PlayError::NotInHand);
        }
        let card = self.card(ch.card);
        let forced = self.forced_choice(seat);
        if forced.is_some_and(|f| f.card == ch.card && f.chamber == ch.chamber) {
            // Soft-lock escape hatch: the same fallback the AI uses (see `forced_choice`).
        } else {
            if !card.valid_chambers.contains(&ch.chamber) {
                return Err(PlayError::InvalidChamber { card: card.short(), chamber: ch.chamber.name() });
            }
            if self.st.is_chamber_locked(card.faction, ch.chamber) {
                return Err(PlayError::Locked { chamber: ch.chamber.name() });
            }
        }
        if let Intent::Release { spend } = ch.intent {
            let own = self.st.stack_total(card.faction, ch.chamber);
            if spend < 1 || spend > own {
                return Err(PlayError::BadRelease { available: own });
            }
        }
        Ok(())
    }

    /// If no revealed glyph has an unlocked legal Chamber, the only play is the AI's
    /// fallback (first glyph, its first Chamber, Press), so humans can never soft-lock.
    pub fn forced_choice(&self, seat: Seat) -> Option<Choice> {
        let f = self.st.faction_of(seat);
        let hand = self.revealed(seat);
        let any_legal = hand
            .iter()
            .any(|&c| self.card(c).valid_chambers.iter().any(|&ch| !self.st.is_chamber_locked(f, ch)));
        if any_legal || hand.is_empty() {
            return None;
        }
        let c = hand[0];
        Some(Choice { card: c, chamber: self.card(c).valid_chambers[0], intent: Intent::Press })
    }

    /// One d6 from the shared RNG.
    pub fn roll(&mut self) -> u8 {
        self.rng.d6()
    }

    /// Spend 1 Will and reroll (Python reroll dialog). Logs "<actor> spends 1 Will to reroll a → b."
    pub fn reroll(&mut self, seat: Seat, raw: u8) -> Result<u8, PlayError> {
        if self.st.seat(seat).will <= 0 {
            return Err(PlayError::NoWill);
        }
        let s = self.st.seat_mut(seat);
        s.will = (s.will - 1).max(0);
        let new_raw = self.rng.d6();
        let name = self.name(seat).to_string();
        self.log(format!("{name} spends 1 Will to reroll {raw} → {new_raw}."));
        self.emit(GameEvent::Reroll { seat, from: raw, to: new_raw });
        Ok(new_raw)
    }

    /// Resolve a full round from both locked plays (Python: sixes→Strain, `make_pending_play`
    /// ×2, `resolve_completed_plays`). Does not start the next round.
    pub fn resolve(&mut self, p1: PlayCommit, p2: PlayCommit) {
        if p1.raw == 6 || p2.raw == 6 {
            self.st.strain += i32::from(p1.raw == 6) + i32::from(p2.raw == 6);
        }
        let info = |g: &Game, seat: Seat, p: &PlayCommit| PlayInfo {
            seat,
            card: g.card(p.card).key.clone(),
            chamber: p.chamber,
            intent: p.intent,
            raw: p.raw,
            intensity: p.intensity(),
        };
        let plays = [info(self, Seat::P1, &p1), info(self, Seat::P2, &p2)];
        self.emit(GameEvent::Plays { plays });
        let mut pp1 = self.make_pending_play(Seat::P1, &p1);
        let mut pp2 = self.make_pending_play(Seat::P2, &p2);
        self.resolve_completed_plays(&mut pp1, &mut pp2);
    }

    fn resolve_completed_plays(&mut self, p1: &mut PendingPlay, p2: &mut PendingPlay) {
        let n1 = self.name(Seat::P1).to_string();
        let n2 = self.name(Seat::P2).to_string();
        for (n, p) in [(&n1, &*p1), (&n2, &*p2)] {
            let line = format!(
                "{n} play {} to {} {} as {} (roll {}, intensity {}).",
                self.card(p.card).short(),
                p.chamber.emoji(),
                p.chamber.name(),
                p.intent.label(),
                p.raw,
                p.intensity
            );
            self.log(line);
        }

        self.st.seats[0].deck.discard_revealed_card(p1.card);
        self.st.seats[1].deck.discard_revealed_card(p2.card);

        self.apply_pre_clash_modifiers(p1, p2);
        self.resolve_clash(p1, p2, &n1, &n2);

        let m1 = self.apply_play(p1);
        let m2 = self.apply_play(p2);
        let ongoing = self.apply_ongoing_effects();
        let total = m1 + m2 + ongoing;
        let (f1, f2) = (self.card(p1.card).faction.short(), self.card(p2.card).faction.short());
        self.log(format!(
            "Round movement audit: {n1} ({f1}) {m1:+}; {n2} ({f2}) {m2:+}; ongoing board effects {ongoing:+}; net {total:+}."
        ));

        let uncapped = total;
        let capped = self.apply_movement_caps(total);
        if capped != total {
            self.log(format!("Soul movement capped: {total:+} becomes {capped:+}."));
        }
        let total = capped;

        let mut actual = 0;
        if total != 0 {
            let old = self.st.soul;
            self.st.soul = (self.st.soul + total).clamp(-self.st.goal, self.st.goal);
            actual = self.st.soul - old;
            if actual != total {
                self.log(format!(
                    "Soul moves {actual:+}: {old:+} → {:+} (track edge absorbed {:+}).",
                    self.st.soul,
                    total - actual
                ));
            } else {
                self.log(format!("Soul moves {total:+}: {old:+} → {:+}.", self.st.soul));
            }
            self.emit(GameEvent::SoulMove { from: old, to: self.st.soul, uncapped });
        } else {
            self.log("The Soul does not move this round.");
            self.emit(GameEvent::SoulMove { from: self.st.soul, to: self.st.soul, uncapped });
        }

        self.record_round_stats(p1, p2, m1, m2, ongoing, uncapped, actual);
        self.resolve_strain_events();
        self.check_winner();
    }

    /// Python `check_winner`: victory text plus the endgame summary in the log.
    pub(crate) fn check_winner(&mut self) {
        if self.st.game_over {
            return;
        }
        let winner = if self.st.soul >= self.st.goal {
            Faction::Heaven
        } else if self.st.soul <= -self.st.goal {
            Faction::Hell
        } else {
            return;
        };
        self.st.game_over = true;
        self.st.winner = Some(winner);
        self.log("");
        self.log(match winner {
            Faction::Heaven => "HEAVEN WINS — the Soul ascends.",
            Faction::Hell => "HELL WINS — the Soul descends.",
        });
        self.stats.winner = Some(winner);
        self.stats.ended_at = self.now();
        let summary = self.build_endgame_summary(winner);
        self.log("");
        for line in summary.lines() {
            self.log(line.to_string());
        }
        self.emit(GameEvent::GameOver { winner });
    }

    /// Python `apply_delayed_movements_at_round_start`.
    fn apply_delayed_movements_at_round_start(&mut self) {
        if self.st.delayed_movements.is_empty() {
            self.st.delayed_movements = std::mem::take(&mut self.st.next_delayed_movements);
        }
        if self.st.delayed_movements.is_empty() {
            return;
        }
        let total: i32 = self.st.delayed_movements.iter().map(|d| d.amount).sum();
        let labels = self
            .st
            .delayed_movements
            .iter()
            .map(|d| format!("{} {:+}", d.label, d.amount))
            .collect::<Vec<_>>()
            .join("; ");
        self.log(format!("Delayed effects resolve at round start: {labels}; total {total:+}."));
        let old = self.st.soul;
        let capped = self.apply_movement_caps(total);
        self.st.soul = (self.st.soul + capped).clamp(-self.st.goal, self.st.goal);
        let actual = self.st.soul - old;
        self.log(format!("Delayed movement shifts the Soul {actual:+}: {old:+} → {:+}.", self.st.soul));
        let entries = std::mem::take(&mut self.st.delayed_movements);
        self.emit(GameEvent::DelayedResolve { entries, total, from: old, to: self.st.soul });
        self.check_winner();
    }

    /// Python `apply_movement_caps`. Consumes both pending caps.
    pub(crate) fn apply_movement_caps(&mut self, total: i32) -> i32 {
        let mut capped = total;
        if let Some(cap) = self.st.next_heaven_cap
            && capped > cap
        {
            capped = cap;
        }
        if let Some(cap) = self.st.next_soul_cap {
            if capped > cap {
                capped = cap;
            } else if capped < -cap {
                capped = -cap;
            }
        }
        self.st.next_heaven_cap = None;
        self.st.next_soul_cap = None;
        capped
    }

    // ------------------------------------------------------------- queries

    /// Python press-bonus preview shown in the Intent dialog.
    pub fn press_preview(&self, faction: Faction, chamber: Chamber, aspect: &str) -> i32 {
        let own = self.st.stack_total(faction, chamber);
        let enemy = self.st.stack_total(faction.opponent(), chamber);
        let harmony = self.st.harmony_count(faction, chamber, aspect);
        crate::rules::intent::press_bonus(own, harmony, enemy)
    }

    pub fn is_over(&self) -> bool {
        self.st.game_over
    }
}
