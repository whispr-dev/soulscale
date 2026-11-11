//! # soulscale-core
//!
//! The Soulscale rules engine: a faithful, deterministic Rust port of the v1.0.1
//! *Soulscale: Purgatory* Python engine.
//!
//! * No I/O, no clocks, no threads: the same code runs on the server, in the simulator,
//!   and (later, if wanted) in WebAssembly.
//! * Seeded with a CPython-compatible Mersenne Twister ([`rng::PyRandom`]), so a seed
//!   replays identically here and in the original Python. `tests/python_parity.rs`
//!   holds that to the letter.
//! * Card data lives in `data/cards.json` (dumped from the Python), so new cards and
//!   decks are data, while new *special rules* are code in [`rules::effects`].
//!
//! Layout follows the design manifesto: one module per concern.
//!
//! | module            | concern                                               |
//! |-------------------|-------------------------------------------------------|
//! | [`model`]         | factions, chambers, keywords, intents, card shape     |
//! | [`carddb`]        | loading/validating card + deck data                   |
//! | [`deck`]          | draw/reveal/discard, legality, custom deck files      |
//! | [`state`]         | the mutable board (stacks, effects, conditions, Will) |
//! | [`game`]          | round pipeline and turn order                         |
//! | [`rules`]         | the maths and card effects, one file per stage        |
//! | [`ai`]            | the five heuristic AI personalities                   |
//! | [`stats`]         | ledger, endgame summary, .txt/.csv exports            |
//! | [`events`]        | structured events for animation                       |
//! | [`session`]       | match orchestration for vs-AI / hot-seat / Duel       |
//! | [`view`]          | per-seat redacted snapshots for clients               |

pub mod ai;
pub mod bot;
pub mod carddb;
pub mod deck;
pub mod events;
pub mod game;
pub mod model;
pub mod protocol;
pub mod rng;
pub mod rules;
pub mod session;
pub mod state;
pub mod stats;
pub mod view;

pub use carddb::{CardDb, db};
pub use game::{Choice, Clock, FixedClock, Game, GameConfig, GameMode, NamedDeck, PlayCommit};
pub use model::*;
pub use state::Seat;

/// Rules version implemented by this crate.
pub const RULES_VERSION: &str = "1.0.2";
/// The original Python ruleset reproduced exactly by `RulesVersion::V1_0_1`.
pub const PYTHON_RULES_VERSION: &str = "1.0.1";
