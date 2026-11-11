//! Structured events emitted while a round resolves. The client animates these in order
//! (deal, flight, clash, sever, release, soul movement...) and then settles on the
//! post-round snapshot. Every human-readable log line is also an event, interleaved at
//! exactly the point the Python engine logged it.

use serde::{Deserialize, Serialize};

use crate::model::{Chamber, Condition, Faction, Intent};
use crate::state::{DelayedMove, Seat};

/// A play as revealed to everyone once both sides have committed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PlayInfo {
    pub seat: Seat,
    pub card: String,
    pub chamber: Chamber,
    pub intent: Intent,
    pub raw: u8,
    pub intensity: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum GameEvent {
    /// One line of the text log.
    Log { text: String },
    RoundStart { round: u32, will: [i32; 2] },
    /// Delayed / Echo movement landing at round start.
    DelayedResolve { entries: Vec<DelayedMove>, total: i32, from: i32, to: i32 },
    /// Cards dealt to a seat (the session layer hides these from the other seat when secret).
    Reveal { seat: Seat, cards: Vec<String> },
    Reroll { seat: Seat, from: u8, to: u8 },
    /// Both plays revealed together.
    Plays { plays: [PlayInfo; 2] },
    /// A pre-clash or keyword modifier changed a play's force (negative = weakened).
    Modifier { seat: Seat, source: String, delta: i32 },
    TrapSprung { chamber: Chamber, card: String, victim: Seat },
    Clash { chamber: Chamber, power: [i32; 2], winner: Option<Seat> },
    Fizzle { seat: Seat },
    StackSpent { faction: Faction, chamber: Chamber, amount: i32 },
    StackCut { faction: Faction, chamber: Chamber, amount: i32, by: String },
    StackAdded { faction: Faction, chamber: Chamber, card: String, value: i32 },
    EffectRemoved { chamber: Chamber, faction: Faction, card: String, by: String },
    EffectPlaced { chamber: Chamber, faction: Faction, card: String, refreshed: bool },
    ConditionAdded { chamber: Chamber, condition: Condition },
    ConditionRemoved { chamber: Chamber, condition: Condition },
    /// Little Miracle / Whispering Serpent extra d6.
    Wild { seat: Seat, card: String, roll: u8 },
    Scheduled { amount: i32, label: String, echo: bool },
    Contribution { seat: Seat, card: String, amount: i32 },
    Ongoing { chamber: Chamber, card: String, faction: Faction, amount: i32 },
    Locked { faction: Faction, chamber: Chamber },
    WillNext { faction: Faction, delta: i32 },
    SoulMove { from: i32, to: i32, uncapped: i32 },
    Strain { value: i32 },
    Purgatory { roll: u8, name: String, emoji: String },
    GameOver { winner: Faction },
}
