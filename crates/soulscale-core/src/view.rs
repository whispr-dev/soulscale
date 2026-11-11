//! Serializable snapshots for clients, redacted per viewer, plus the static card catalog.

use serde::{Deserialize, Serialize};

use crate::carddb::CardDb;
use crate::game::GameMode;
use crate::model::{AiProfile, CardDef, Chamber, Condition, Faction, Intent};
use crate::session::{Controller, SeatStatus, Session, Vis};
use crate::state::{DelayedMove, Seat};

/// Who is looking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(tag = "kind", content = "seat", rename_all = "snake_case")]
pub enum Viewer {
    /// A player controlling one seat (Duel, or the human in VsComputer).
    Seat(Seat),
    /// One device controlling both seats (hot-seat).
    Both,
    Spectator,
}

impl Viewer {
    pub fn controls(self, s: Seat) -> bool {
        match self {
            Viewer::Seat(x) => x == s,
            Viewer::Both => true,
            Viewer::Spectator => false,
        }
    }

    /// Whether an event with this visibility reaches this viewer.
    pub fn sees(self, vis: Vis) -> bool {
        match vis {
            Vis::Public => true,
            Vis::Only(s) => self.controls(s),
            Vis::Except(s) => !self.controls(s),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ChoiceView {
    pub card: String,
    pub chamber: Chamber,
    pub intent: Intent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SeatView {
    pub seat: Seat,
    pub name: String,
    pub faction: Faction,
    pub deck_name: String,
    pub controller: Controller,
    pub will: i32,
    /// Pending next-round Will change (bonus minus penalty).
    pub next_will: i32,
    /// "choose" | "reroll" | "locked" | "waiting" | "done" (the reroll roll is private).
    pub public_status: String,
    /// Full status, only for whoever controls the seat.
    pub status: Option<SeatStatus>,
    pub hand: Option<Vec<String>>,
    pub hand_count: usize,
    pub draw_count: usize,
    pub discard_count: usize,
    pub choice: Option<ChoiceView>,
    /// This round's d6 (after any reroll), only for whoever controls the seat.
    pub roll: Option<u8>,
    pub locked_chambers: Vec<Chamber>,
    /// Set when no legal play exists: the only allowed play (AI fallback rule).
    pub forced: Option<ChoiceView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TokenView {
    pub card: String,
    pub aspect: String,
    pub value: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EffectView {
    pub faction: Faction,
    pub card: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ChamberView {
    pub chamber: Chamber,
    pub effects: Vec<EffectView>,
    pub conditions: Vec<Condition>,
    pub heaven: Vec<TokenView>,
    pub hell: Vec<TokenView>,
    pub heaven_total: i32,
    pub hell_total: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct View {
    pub viewer: Viewer,
    pub mode: GameMode,
    pub round: u32,
    pub soul: i32,
    pub goal: i32,
    pub strain: i32,
    pub over: bool,
    pub winner: Option<Faction>,
    pub seats: [SeatView; 2],
    pub chambers: Vec<ChamberView>,
    pub delayed_next: Vec<DelayedMove>,
    pub heaven_cap: Option<i32>,
    pub soul_cap: Option<i32>,
    pub heaven_next_bonus: i32,
    pub hell_next_bonus: i32,
    pub all_next_bonus: i32,
    pub support_bonus: [i32; 2],
    pub summary: Option<String>,
    /// Personality driving any AI seat.
    pub ai_profile: String,
    pub rules: crate::model::RulesVersion,
}

fn status_word(s: SeatStatus) -> &'static str {
    match s {
        SeatStatus::Choose => "choose",
        SeatStatus::Reroll { .. } => "reroll",
        SeatStatus::Locked => "locked",
        SeatStatus::Waiting => "waiting",
        SeatStatus::Done => "done",
    }
}

impl Session {
    /// Whether `viewer` may see `seat`'s hand in this mode.
    pub fn hand_visible(&self, viewer: Viewer, seat: Seat) -> bool {
        match self.mode() {
            GameMode::VsComputer | GameMode::Hotseat => true,
            GameMode::SecretHotseat | GameMode::Duel => viewer.controls(seat),
        }
    }

    pub fn view(&self, viewer: Viewer) -> View {
        let g = &self.game;
        let db = g.db;
        let key = |id: crate::model::CardId| db.card(id).key.clone();
        let seats = Seat::BOTH.map(|s| {
            let st = g.st.seat(s);
            let status = self.status(s);
            let mine = viewer.controls(s) && self.controllers[s.idx()] == Controller::Human;
            let hand = self.hand_visible(viewer, s).then(|| st.deck.revealed.iter().map(|&c| key(c)).collect());
            let choice = if mine {
                self.turns[s.idx()].choice.map(|c| ChoiceView { card: key(c.card), chamber: c.chamber, intent: c.intent })
            } else {
                None
            };
            let forced = if mine {
                g.forced_choice(s).map(|c| ChoiceView { card: key(c.card), chamber: c.chamber, intent: c.intent })
            } else {
                None
            };
            SeatView {
                seat: s,
                name: g.name(s).to_string(),
                faction: st.faction,
                deck_name: st.deck_name.clone(),
                controller: self.controllers[s.idx()],
                will: st.will,
                next_will: st.next_will_bonus - st.next_will_penalty,
                public_status: status_word(status).to_string(),
                status: mine.then_some(status),
                hand,
                hand_count: st.deck.revealed.len(),
                draw_count: st.deck.draw_pile.len(),
                discard_count: st.deck.discard_pile.len(),
                choice,
                roll: if mine { self.turns[s.idx()].raw } else { None },
                locked_chambers: g.st.chamber_locks[st.faction.idx()].iter().collect(),
                forced,
            }
        });
        let tokens = |f: Faction, c: Chamber| -> Vec<TokenView> {
            g.st.stacks[f.idx()][c.idx()]
                .iter()
                .map(|t| TokenView { card: key(t.card), aspect: t.aspect.clone(), value: t.value })
                .collect()
        };
        let chambers = Chamber::ALL
            .iter()
            .map(|&c| ChamberView {
                chamber: c,
                effects: g.st.board[c.idx()].iter().map(|e| EffectView { faction: e.faction, card: key(e.card) }).collect(),
                conditions: g.st.conditions[c.idx()].clone(),
                heaven: tokens(Faction::Heaven, c),
                hell: tokens(Faction::Hell, c),
                heaven_total: g.st.stack_total(Faction::Heaven, c),
                hell_total: g.st.stack_total(Faction::Hell, c),
            })
            .collect();
        View {
            viewer,
            mode: self.mode(),
            round: g.st.round,
            soul: g.st.soul,
            goal: g.st.goal,
            strain: g.st.strain,
            over: g.st.game_over,
            winner: g.st.winner,
            seats,
            chambers,
            delayed_next: g.st.next_delayed_movements.clone(),
            heaven_cap: g.st.next_heaven_cap,
            soul_cap: g.st.next_soul_cap,
            heaven_next_bonus: g.st.next_angel_bonus,
            hell_next_bonus: g.st.next_devil_bonus,
            all_next_bonus: g.st.next_all_bonus,
            support_bonus: g.st.support_bonus,
            summary: g.st.winner.map(|w| g.build_endgame_summary(w)),
            ai_profile: g.cfg.ai_profile.label.clone(),
            rules: g.cfg.rules,
        }
    }
}

/// Static data the client needs once: every card, the built-in decks, the AI personalities.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Catalog {
    pub rules_version: String,
    pub cards: Vec<CardDef>,
    pub decks: Vec<DeckView>,
    pub ai_profiles: Vec<AiProfile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DeckView {
    pub name: String,
    pub faction: Faction,
    pub cards: Vec<String>,
}

impl Catalog {
    pub fn from_db(db: &CardDb) -> Catalog {
        let decks = Faction::ALL
            .iter()
            .flat_map(|&f| db.decks(f))
            .map(|d| DeckView {
                name: d.name.clone(),
                faction: d.faction,
                cards: d.cards.iter().map(|&c| db.card(c).key.clone()).collect(),
            })
            .collect();
        Catalog {
            rules_version: db.rules_version.clone(),
            cards: db.all().map(|(_, c): (_, &CardDef)| c.clone()).collect(),
            decks,
            ai_profiles: db.profiles().to_vec(),
        }
    }
}
