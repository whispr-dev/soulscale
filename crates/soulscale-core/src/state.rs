//! Mutable game state: Soul track, Will, Resonance stacks, board effects, conditions,
//! delayed movements, locks and one-shot bonuses. Mirrors every field the Python
//! `start_game` resets.

use serde::{Deserialize, Serialize};

use crate::deck::DeckState;
use crate::model::{CardId, Chamber, Condition, Faction, MAX_VISIBLE_STACK_TOKENS};

/// The two seats. P1 is the Python "player"; P2 is the Python "ai"/"opponent".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Seat {
    P1,
    P2,
}

impl Seat {
    pub const BOTH: [Seat; 2] = [Seat::P1, Seat::P2];

    pub fn idx(self) -> usize {
        match self {
            Seat::P1 => 0,
            Seat::P2 => 1,
        }
    }

    pub fn other(self) -> Seat {
        match self {
            Seat::P1 => Seat::P2,
            Seat::P2 => Seat::P1,
        }
    }
}

/// One Resonance token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackToken {
    pub card: CardId,
    pub aspect: String,
    pub value: i32,
}

/// An ongoing / guard / trap card sitting in a Chamber.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardEffect {
    pub faction: Faction,
    pub card: CardId,
}

/// A movement scheduled for the start of the next round (Delayed / Echo).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DelayedMove {
    pub amount: i32,
    pub label: String,
}

/// Per-seat state.
#[derive(Clone, Debug)]
pub struct SeatState {
    pub faction: Faction,
    pub deck_name: String,
    pub deck: DeckState,
    pub will: i32,
    pub next_will_bonus: i32,
    pub next_will_penalty: i32,
}

/// A 5-bit set of chambers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChamberSet(pub u8);

impl ChamberSet {
    pub fn contains(self, c: Chamber) -> bool {
        self.0 & (1 << c.idx()) != 0
    }
    pub fn insert(&mut self, c: Chamber) {
        self.0 |= 1 << c.idx();
    }
    pub fn iter(self) -> impl Iterator<Item = Chamber> {
        Chamber::ALL.into_iter().filter(move |c| self.contains(*c))
    }
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub soul: i32,
    pub goal: i32,
    pub round: u32,
    pub strain: i32,
    pub seats: [SeatState; 2],

    pub next_angel_bonus: i32,
    pub next_devil_bonus: i32,
    pub next_all_bonus: i32,
    pub next_heaven_cap: Option<i32>,
    pub next_soul_cap: Option<i32>,
    /// Indexed by `Faction::idx`.
    pub support_bonus: [i32; 2],
    pub delayed_movements: Vec<DelayedMove>,
    pub next_delayed_movements: Vec<DelayedMove>,
    /// Indexed by `Faction::idx`.
    pub chamber_locks: [ChamberSet; 2],
    pub next_chamber_locks: [ChamberSet; 2],

    pub board: [Vec<BoardEffect>; 5],
    pub conditions: [Vec<Condition>; 5],
    /// `stacks[faction][chamber]`.
    pub stacks: [[Vec<StackToken>; 5]; 2],

    pub game_over: bool,
    pub winner: Option<Faction>,
}

impl GameState {
    pub fn seat_of(&self, f: Faction) -> Seat {
        if self.seats[0].faction == f { Seat::P1 } else { Seat::P2 }
    }

    pub fn seat(&self, s: Seat) -> &SeatState {
        &self.seats[s.idx()]
    }

    pub fn seat_mut(&mut self, s: Seat) -> &mut SeatState {
        &mut self.seats[s.idx()]
    }

    pub fn faction_of(&self, s: Seat) -> Faction {
        self.seats[s.idx()].faction
    }

    pub fn will_of_faction(&self, f: Faction) -> i32 {
        self.seat(self.seat_of(f)).will
    }

    // ---------------------------------------------------------------- stacks

    pub fn stack_total(&self, f: Faction, c: Chamber) -> i32 {
        self.stacks[f.idx()][c.idx()].iter().map(|t| t.value).sum()
    }

    /// Number of friendly tokens with the same aspect (Python `harmony_count`).
    pub fn harmony_count(&self, f: Faction, c: Chamber, aspect: &str) -> i32 {
        self.stacks[f.idx()][c.idx()].iter().filter(|t| t.aspect == aspect).count() as i32
    }

    pub fn add_stack(&mut self, f: Faction, c: Chamber, card: CardId, aspect: &str, value: i32) {
        if value <= 0 {
            return;
        }
        let tokens = &mut self.stacks[f.idx()][c.idx()];
        tokens.push(StackToken { card, aspect: aspect.to_string(), value });
        while tokens.len() > MAX_VISIBLE_STACK_TOKENS {
            // Merge the two oldest tokens to keep the board readable.
            let first = tokens.remove(0);
            tokens[0].value += first.value;
        }
    }

    /// Release / Hungry: newest tokens first.
    pub fn consume_stack(&mut self, f: Faction, c: Chamber, amount: i32) -> i32 {
        self.remove_stack_value(f, c, amount, true)
    }

    /// Sever / Flashback: oldest tokens first.
    pub fn cut_stack(&mut self, f: Faction, c: Chamber, amount: i32) -> i32 {
        self.remove_stack_value(f, c, amount, false)
    }

    fn remove_stack_value(&mut self, f: Faction, c: Chamber, mut amount: i32, newest_first: bool) -> i32 {
        if amount <= 0 {
            return 0;
        }
        let tokens = &mut self.stacks[f.idx()][c.idx()];
        let mut removed = 0;
        while amount > 0 && !tokens.is_empty() {
            let idx = if newest_first { tokens.len() - 1 } else { 0 };
            let take = tokens[idx].value.min(amount);
            tokens[idx].value -= take;
            amount -= take;
            removed += take;
            if tokens[idx].value <= 0 {
                tokens.remove(idx);
            }
        }
        removed
    }

    // ----------------------------------------------------------------- locks

    pub fn is_chamber_locked(&self, f: Faction, c: Chamber) -> bool {
        self.chamber_locks[f.idx()].contains(c)
    }

    pub fn lock_chamber_next_round(&mut self, f: Faction, c: Chamber) {
        self.next_chamber_locks[f.idx()].insert(c);
    }

    // ------------------------------------------------------------------ will

    pub fn apply_next_will_bonus(&mut self, f: Faction, amount: i32) {
        let s = self.seat_of(f);
        self.seat_mut(s).next_will_bonus += amount;
    }

    pub fn apply_next_will_penalty(&mut self, f: Faction, amount: i32) {
        let s = self.seat_of(f);
        self.seat_mut(s).next_will_penalty += amount;
    }

    /// Python `spend_will_for_faction`.
    pub fn spend_will_for_faction(&mut self, f: Faction, amount: i32) -> bool {
        if amount <= 0 {
            return true;
        }
        let s = self.seat_of(f);
        let seat = self.seat_mut(s);
        if seat.will >= amount {
            seat.will -= amount;
            true
        } else {
            false
        }
    }

    pub fn has_condition(&self, c: Chamber, cond: Condition) -> bool {
        self.conditions[c.idx()].contains(&cond)
    }

    pub fn remove_condition(&mut self, c: Chamber, cond: Condition) -> bool {
        let v = &mut self.conditions[c.idx()];
        if let Some(p) = v.iter().position(|&x| x == cond) {
            v.remove(p);
            true
        } else {
            false
        }
    }
}
