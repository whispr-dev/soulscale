//! Match orchestration: who acts when, who may see what, and replay recording.
//!
//! A [`Session`] wraps a [`Game`] and drives it from player inputs, reproducing the
//! v1.0.1 step order for each mode:
//!
//! * **VsComputer**: AI commits blind at round start → human chooses → human rolls (+
//!   optional reroll) → AI reveals, rolls, maybe rerolls → resolve.
//! * **Hotseat / SecretHotseat**: P1 chooses → rolls → reroll decision → P2 chooses →
//!   rolls → reroll decision → resolve.
//! * **Duel**: both humans act in parallel on their own devices; whoever finishes first
//!   rolls first; resolve when both are locked. AI-controlled seats (disconnect / AFK
//!   takeover) commit blind at round start and roll after the humans, like VsComputer.
//!
//! Every emitted event carries a [`Vis`] so the server can hide hands, choices and rolls
//! until the plays are revealed together.

use serde::{Deserialize, Serialize};

use crate::events::GameEvent;
use crate::game::{Choice, Game, GameMode, PlayCommit, PlayError};
use crate::model::{AiProfile, Chamber};
use crate::rules::intensity::intensity_from_d6;
use crate::state::Seat;

/// Who drives a seat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum Controller {
    Human,
    Ai,
}

/// Visibility of an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vis {
    Public,
    /// Only whoever controls this seat.
    Only(Seat),
    /// Everyone except whoever controls this seat (used to reveal held events later).
    Except(Seat),
}

/// What a seat must do next.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeatStatus {
    /// Pick glyph + Chamber + Intent.
    Choose,
    /// Rolled; decide whether to spend 1 Will to reroll.
    Reroll { raw: u8, intensity: i32 },
    /// Locked in for this round.
    Locked,
    /// Not this seat's turn yet (hot-seat ordering).
    Waiting,
    /// Game over.
    Done,
}

/// Per-seat progress through the current round.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatTurn {
    pub choice: Option<Choice>,
    pub raw: Option<u8>,
    pub locked: bool,
}

/// One recorded input, enough to replay a match deterministically.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Input {
    Choose { seat: Seat, card: String, chamber: Chamber, intent: crate::model::Intent },
    Reroll { seat: Seat, reroll: bool },
    Controller { seat: Seat, controller: Controller },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("{0}")]
    Play(#[from] PlayError),
    #[error("it is not that seat's turn to do that")]
    NotYourTurn,
    #[error("that seat is AI-controlled")]
    AiSeat,
    #[error("unknown card {0:?}")]
    UnknownCard(String),
}

pub struct Session {
    pub game: Game,
    pub controllers: [Controller; 2],
    pub turns: [SeatTurn; 2],
    profile: AiProfile,
    /// Events produced by a seat's private actions, revealed to others at resolution.
    held: Vec<(Seat, GameEvent)>,
    out: Vec<(Vis, GameEvent)>,
    pub inputs: Vec<Input>,
    pub seed: u64,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session").field("game", &self.game).field("turns", &self.turns).finish_non_exhaustive()
    }
}

impl Session {
    /// Starts the match (welcome log + round 1 + any blind AI commits).
    pub fn start(game: Game, controllers: [Controller; 2], seed: u64) -> Session {
        let profile = game.cfg.ai_profile.clone();
        let mut s = Session {
            game,
            controllers,
            turns: Default::default(),
            profile,
            held: Vec::new(),
            out: Vec::new(),
            inputs: Vec::new(),
            seed,
        };
        s.game.start();
        s.after_round_start();
        s
    }

    pub fn mode(&self) -> GameMode {
        self.game.cfg.mode
    }

    /// Take every event (with its visibility) produced since the last call.
    pub fn drain(&mut self) -> Vec<(Vis, GameEvent)> {
        std::mem::take(&mut self.out)
    }

    fn collect(&mut self, private_to: Option<Seat>) {
        let secret = self.mode().is_secret();
        for e in self.game.drain_events() {
            let vis = match (&e, private_to) {
                (GameEvent::Reveal { seat, .. }, _) if secret => Vis::Only(*seat),
                (_, Some(seat)) => {
                    self.held.push((seat, e.clone()));
                    Vis::Only(seat)
                }
                _ => Vis::Public,
            };
            self.out.push((vis, e));
        }
    }

    fn flush_held(&mut self) {
        for (seat, e) in std::mem::take(&mut self.held) {
            self.out.push((Vis::Except(seat), e));
        }
    }

    /// After `begin_round`: reset turns and let AI seats commit blind.
    fn after_round_start(&mut self) {
        self.collect(None);
        self.turns = Default::default();
        if self.game.is_over() {
            return;
        }
        for seat in Seat::BOTH {
            if self.controllers[seat.idx()] == Controller::Ai {
                self.ai_commit(seat);
            }
        }
        self.maybe_finish_round();
    }

    fn ai_commit(&mut self, seat: Seat) {
        let profile = self.profile.clone();
        let choice = self.game.ai_choose(seat, &profile, None);
        self.turns[seat.idx()].choice = Some(choice);
        let msg = if self.mode() == GameMode::VsComputer {
            "Computer secretly commits a play now, before your glyph/Chamber/Intent choice.".to_string()
        } else {
            format!("{} (AI) secretly commits a play.", self.game.name(seat))
        };
        self.game.log(msg);
        self.collect(None);
    }

    /// What `seat` must do right now.
    pub fn status(&self, seat: Seat) -> SeatStatus {
        if self.game.is_over() {
            return SeatStatus::Done;
        }
        let t = &self.turns[seat.idx()];
        if t.locked || (self.controllers[seat.idx()] == Controller::Ai && t.choice.is_some()) {
            return SeatStatus::Locked;
        }
        if let (Some(_), Some(raw)) = (t.choice, t.raw) {
            return SeatStatus::Reroll { raw, intensity: intensity_from_d6(raw) };
        }
        let hotseat = matches!(self.mode(), GameMode::Hotseat | GameMode::SecretHotseat);
        if hotseat && seat == Seat::P2 && !self.turns[0].locked {
            return SeatStatus::Waiting;
        }
        SeatStatus::Choose
    }

    fn require_human(&self, seat: Seat) -> Result<(), SessionError> {
        if self.controllers[seat.idx()] == Controller::Ai {
            return Err(SessionError::AiSeat);
        }
        Ok(())
    }

    /// A human picks glyph + Chamber + Intent. The die is rolled immediately; if the seat
    /// has Will the next step is a reroll decision, otherwise the seat locks.
    pub fn choose(&mut self, seat: Seat, choice: Choice) -> Result<(), SessionError> {
        self.require_human(seat)?;
        if self.status(seat) != SeatStatus::Choose {
            return Err(SessionError::NotYourTurn);
        }
        self.game.validate_choice(seat, &choice)?;
        self.inputs.push(Input::Choose {
            seat,
            card: self.game.card(choice.card).key.clone(),
            chamber: choice.chamber,
            intent: choice.intent,
        });
        let raw = self.game.roll();
        let t = &mut self.turns[seat.idx()];
        t.choice = Some(choice);
        t.raw = Some(raw);
        if self.game.st.seat(seat).will <= 0 {
            t.locked = true;
            self.collect(Some(seat));
            self.maybe_finish_round();
        } else {
            self.collect(Some(seat));
        }
        Ok(())
    }

    /// Keep (`false`) or spend 1 Will to reroll (`true`). Either way the seat locks.
    pub fn decide_reroll(&mut self, seat: Seat, reroll: bool) -> Result<(), SessionError> {
        self.require_human(seat)?;
        let SeatStatus::Reroll { raw, .. } = self.status(seat) else {
            return Err(SessionError::NotYourTurn);
        };
        self.inputs.push(Input::Reroll { seat, reroll });
        if reroll {
            let new_raw = self.game.reroll(seat, raw)?;
            self.turns[seat.idx()].raw = Some(new_raw);
        }
        self.turns[seat.idx()].locked = true;
        self.collect(Some(seat));
        self.maybe_finish_round();
        Ok(())
    }

    /// Swap a seat between human and AI (disconnect / AFK takeover / reclaim).
    /// An AI taking over mid-round commits immediately if the seat hadn't chosen yet.
    pub fn set_controller(&mut self, seat: Seat, c: Controller) {
        if self.controllers[seat.idx()] == c {
            return;
        }
        self.inputs.push(Input::Controller { seat, controller: c });
        self.controllers[seat.idx()] = c;
        if self.game.is_over() {
            return;
        }
        if c == Controller::Ai {
            let t = self.turns[seat.idx()];
            if t.choice.is_none() {
                self.ai_commit(seat);
            } else if t.raw.is_some() && !t.locked {
                // Mid reroll decision: keep the roll.
                self.turns[seat.idx()].locked = true;
            }
            self.maybe_finish_round();
        }
    }

    fn maybe_finish_round(&mut self) {
        if self.game.is_over() {
            return;
        }
        // Every human seat must be locked; AI seats only need their blind choice.
        for s in Seat::BOTH {
            let t = &self.turns[s.idx()];
            let ready = match self.controllers[s.idx()] {
                Controller::Human => t.locked,
                Controller::Ai => t.choice.is_some(),
            };
            if !ready {
                return;
            }
        }
        // AI seats reveal and roll now, after the humans (Python vs-Computer order).
        let profile = self.profile.clone();
        for s in Seat::BOTH {
            if self.controllers[s.idx()] != Controller::Ai || self.turns[s.idx()].locked {
                continue;
            }
            let choice = self.turns[s.idx()].choice.expect("AI seat committed");
            let card = self.game.card(choice.card);
            let line = if self.mode() == GameMode::VsComputer {
                format!(
                    "Computer reveals its blind committed play: {} → {} {} as {}.",
                    card.short(),
                    choice.chamber.emoji(),
                    choice.chamber.name(),
                    choice.intent.label()
                )
            } else {
                format!(
                    "{} (AI) reveals its committed play: {} → {} {} as {}.",
                    self.game.name(s),
                    card.short(),
                    choice.chamber.emoji(),
                    choice.chamber.name(),
                    choice.intent.label()
                )
            };
            self.game.log(line);
            let mut raw = self.game.roll();
            let other_chamber = self.turns[s.other().idx()].choice.map(|c| c.chamber);
            if self.game.st.seat(s).will > 0
                && self.game.ai_should_reroll(&profile, &choice, intensity_from_d6(raw), other_chamber)
            {
                let seat = self.game.st.seat_mut(s);
                seat.will -= 1;
                let old = raw;
                raw = self.game.roll();
                let name = self.game.name(s).to_string();
                self.game.log(format!("{name} spends 1 Will to reroll {old} → {raw}."));
                self.game.emit_reroll(s, old, raw);
            }
            self.turns[s.idx()].raw = Some(raw);
            self.turns[s.idx()].locked = true;
        }
        self.flush_held();
        self.collect(None);

        let commit = |t: &SeatTurn| {
            let c = t.choice.expect("locked seat has a choice");
            PlayCommit { card: c.card, chamber: c.chamber, intent: c.intent, raw: t.raw.expect("locked seat rolled") }
        };
        let (p1, p2) = (commit(&self.turns[0]), commit(&self.turns[1]));
        self.game.resolve(p1, p2);
        self.collect(None);
        if !self.game.is_over() {
            self.game.begin_round();
            self.after_round_start();
        }
    }

    /// Convenience for tests and bots: resolve a card key to an id.
    pub fn card_id(&self, key: &str) -> Result<crate::model::CardId, SessionError> {
        self.game.db.id(key).ok_or_else(|| SessionError::UnknownCard(key.to_string()))
    }

    /// Re-apply recorded inputs (replays / determinism checks).
    pub fn apply_input(&mut self, input: &Input) -> Result<(), SessionError> {
        match input {
            Input::Choose { seat, card, chamber, intent } => {
                let id = self.card_id(card)?;
                self.choose(*seat, Choice { card: id, chamber: *chamber, intent: *intent })
            }
            Input::Reroll { seat, reroll } => self.decide_reroll(*seat, *reroll),
            Input::Controller { seat, controller } => {
                self.set_controller(*seat, *controller);
                Ok(())
            }
        }
    }
}

impl Game {
    pub(crate) fn emit_reroll(&mut self, seat: Seat, from: u8, to: u8) {
        self.emit(GameEvent::Reroll { seat, from, to });
    }
}
