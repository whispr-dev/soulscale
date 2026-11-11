//! WebSocket protocol between the client (Discord Activity / browser) and the server.
//! JSON, one message per frame, tagged by `"t"`. TypeScript mirrors are generated into
//! `client/src/protocol/generated/` by `cargo test -p soulscale-core --features ts`.

use serde::{Deserialize, Serialize};

use crate::events::GameEvent;
use crate::game::GameMode;
use crate::model::{Chamber, Faction, Intent};
use crate::state::Seat;
use crate::view::{Catalog, View};

#[cfg(feature = "ts")]
use ts_rs::TS;

/// Which deck to use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeckChoice {
    Builtin { name: String },
    /// A deck from the in-game editor: 10 card keys for the chosen side.
    Custom { name: String, cards: Vec<String> },
    /// Python `choose_ai_deck_name` (personality-preferred deck, else random).
    Random,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct NewGame {
    pub mode: GameMode,
    /// The creator's side (P1).
    pub side: Faction,
    pub deck: DeckChoice,
    /// Opponent deck for vs-Computer / hot-seat. Ignored for Duel (the joiner picks).
    pub opponent_deck: Option<DeckChoice>,
    pub goal: i32,
    pub ai_profile: Option<String>,
    /// Defaults to the latest rules (v1.0.2). "1.0.1" replays the original Python exactly.
    #[serde(default)]
    pub rules: Option<crate::model::RulesVersion>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum ClientMsg {
    /// First message. `token` is the Discord access token (verified server-side);
    /// `dev_name` is only honoured when the server runs with dev logins enabled.
    Hello { token: Option<String>, dev_name: Option<String>, instance: String },
    NewGame { game: NewGame },
    JoinDuel { deck: DeckChoice },
    CancelOpen,
    Choose { seat: Seat, card: String, chamber: Chamber, intent: Intent },
    Reroll { seat: Seat, reroll: bool },
    Resign,
    Rematch,
    Export,
    Ping { n: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Role {
    P1,
    P2,
    /// Hot-seat: one person drives both seats.
    Both,
    /// Waiting for an opponent to accept their Duel.
    Host,
    Spectator,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct ParticipantInfo {
    pub user: UserInfo,
    pub role: Role,
    pub connected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TableInfo {
    Empty,
    Open { host: UserInfo, side: Faction, deck: String, goal: i32, rules: crate::model::RulesVersion },
    Live { mode: GameMode, seats: [Option<UserInfo>; 2], round: u32, over: bool, rules: crate::model::RulesVersion },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct RoomInfo {
    pub participants: Vec<ParticipantInfo>,
    pub table: TableInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(tag = "t", rename_all = "snake_case")]
// Built, serialized and dropped straight away; boxing the big variants buys nothing.
#[allow(clippy::large_enum_variant)]
pub enum ServerMsg {
    Welcome { user: UserInfo, catalog: Catalog, server: String, dev: bool },
    Room { room: RoomInfo },
    /// Snapshot after the events, which the client animates first.
    Game { view: View, events: Vec<GameEvent> },
    Error { message: String },
    Notice { text: String },
    Export { basename: String, report: String, csv: String, replay: String },
    Pong { n: u32 },
}

/// The seat (if any) a role controls, as a [`crate::view::Viewer`].
impl Role {
    pub fn viewer(&self) -> crate::view::Viewer {
        use crate::view::Viewer;
        match self {
            Role::P1 => Viewer::Seat(Seat::P1),
            Role::P2 => Viewer::Seat(Seat::P2),
            Role::Both => Viewer::Both,
            Role::Host | Role::Spectator => Viewer::Spectator,
        }
    }
}
