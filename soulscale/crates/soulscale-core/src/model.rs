//! Core vocabulary: factions, chambers, keywords, intents and card definitions.

use serde::{Deserialize, Serialize};

#[cfg(feature = "ts")]
use ts_rs::TS;

/// Heaven (Angel) pushes the Soul positive; Hell (Devil) pushes it negative.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum Faction {
    Heaven,
    Hell,
}

impl Faction {
    pub const ALL: [Faction; 2] = [Faction::Heaven, Faction::Hell];

    pub fn opponent(self) -> Faction {
        match self {
            Faction::Heaven => Faction::Hell,
            Faction::Hell => Faction::Heaven,
        }
    }

    /// +1 for Heaven, -1 for Hell (Python `FACTION_SIGN`).
    pub fn sign(self) -> i32 {
        match self {
            Faction::Heaven => 1,
            Faction::Hell => -1,
        }
    }

    pub fn idx(self) -> usize {
        match self {
            Faction::Heaven => 0,
            Faction::Hell => 1,
        }
    }

    /// Python `FACTION_LABEL`.
    pub fn label(self) -> &'static str {
        match self {
            Faction::Heaven => "Heaven / Angel",
            Faction::Hell => "Hell / Devil",
        }
    }

    /// Python `FACTION_SHORT`.
    pub fn short(self) -> &'static str {
        match self {
            Faction::Heaven => "Heaven",
            Faction::Hell => "Hell",
        }
    }
}

/// The five Chambers of the Soul, in canonical order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum Chamber {
    Body,
    Mind,
    Heart,
    Time,
    Memory,
}

impl Chamber {
    pub const ALL: [Chamber; 5] = [Chamber::Body, Chamber::Mind, Chamber::Heart, Chamber::Time, Chamber::Memory];

    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Chamber::Body => "Body",
            Chamber::Mind => "Mind",
            Chamber::Heart => "Heart",
            Chamber::Time => "Time",
            Chamber::Memory => "Memory",
        }
    }

    /// Python `CHAMBER_EMOJI` (variation selectors preserved exactly).
    pub fn emoji(self) -> &'static str {
        match self {
            Chamber::Body => "🫀",
            Chamber::Mind => "🧠",
            Chamber::Heart => "❤️",
            Chamber::Time => "🕰️",
            Chamber::Memory => "🪞",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
}

impl Rarity {
    /// Deck-building budget cost.
    pub fn cost(self) -> u32 {
        match self {
            Rarity::Common => 1,
            Rarity::Uncommon => 2,
            Rarity::Rare => 3,
        }
    }

    pub fn badge(self) -> &'static str {
        match self {
            Rarity::Common => "C",
            Rarity::Uncommon => "U",
            Rarity::Rare => "R",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Uncommon => "Uncommon",
            Rarity::Rare => "Rare",
        }
    }
}

/// v1.0 price keywords.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum Price {
    Heavy,
    Fragile,
    Volatile,
    Delayed,
    Binding,
    Hungry,
    Echo,
    Desperate,
    Narrow,
}

impl Price {
    pub fn name(self) -> &'static str {
        match self {
            Price::Heavy => "Heavy",
            Price::Fragile => "Fragile",
            Price::Volatile => "Volatile",
            Price::Delayed => "Delayed",
            Price::Binding => "Binding",
            Price::Hungry => "Hungry",
            Price::Echo => "Echo",
            Price::Desperate => "Desperate",
            Price::Narrow => "Narrow",
        }
    }
}

/// v1.0.1 declared movement direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum MovementType {
    #[default]
    Ownerward,
    Opponentward,
    Heavenward,
    Hellward,
    Neutral,
}

impl MovementType {
    pub fn label(self) -> &'static str {
        match self {
            MovementType::Ownerward => "ownerward",
            MovementType::Opponentward => "opponentward",
            MovementType::Heavenward => "heavenward",
            MovementType::Hellward => "hellward",
            MovementType::Neutral => "neutral",
        }
    }
}

/// Coded special-rule hooks. Several cards share one (e.g. Snare uses `FalseDoor`),
/// exactly as in the Python `special` strings. Adding a card with a new special
/// requires code; the loader rejects unknown specials so a typo cannot slip through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Special {
    // Heaven
    BodyBonus,
    BehindBonus,
    Calm,
    Hymn,
    Rest,
    LovedMemory,
    Guardian,
    Miracle,
    Bell,
    SupportNext,
    Sweetness,
    Lightness,
    Purpose,
    Sanctuary,
    Dawn,
    TeaQuiet,
    Acceptance,
    Grace,
    // Hell
    Ache,
    Screech,
    Fly,
    Thorn,
    Burden,
    FalseDoor,
    WatchingEye,
    EndlessWait,
    Cold,
    Tangle,
    Serpent,
    Weight,
    Fever,
    Obsession,
    Futility,
    Rupture,
    CrawlingSkin,
}

/// Chamber conditions left by cards/events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum Condition {
    Cold,
    Thorn,
}

impl Condition {
    pub fn name(self) -> &'static str {
        match self {
            Condition::Cold => "Cold",
            Condition::Thorn => "Thorn",
        }
    }
}

/// Index into the card database. Stable for the life of a process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CardId(pub u16);

/// A card definition, as loaded from `data/cards.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct CardDef {
    pub key: String,
    pub faction: Faction,
    pub emoji: String,
    pub name: String,
    pub valid_chambers: Vec<Chamber>,
    pub type_label: String,
    pub aspect: String,
    pub stack_value: i32,
    pub rarity: Rarity,
    pub prices: Vec<Price>,
    pub movement_type: MovementType,
    pub base_power: i32,
    pub base_move: i32,
    pub ongoing_move: i32,
    pub ongoing: bool,
    pub trap: bool,
    pub guard: bool,
    pub special: Special,
    pub help_text: String,
}

impl CardDef {
    /// Python `Card.short`: "emoji name".
    pub fn short(&self) -> String {
        format!("{} {}", self.emoji, self.name)
    }

    pub fn has_price(&self, p: Price) -> bool {
        self.prices.contains(&p)
    }

    /// Python `price_label`.
    pub fn price_label(&self) -> String {
        if self.prices.is_empty() {
            "—".to_string()
        } else {
            self.prices.iter().map(|p| p.name()).collect::<Vec<_>>().join(", ")
        }
    }

    /// Python `is_disruptive`: type-label substring OR curated special set.
    pub fn is_disruptive(&self) -> bool {
        self.type_label.contains("Disruption")
            || self.type_label.contains("Trap")
            || self.type_label.contains("Control")
            || matches!(
                self.special,
                Special::Calm
                    | Special::Bell
                    | Special::FalseDoor
                    | Special::WatchingEye
                    | Special::Tangle
                    | Special::Serpent
                    | Special::Acceptance
            )
    }

    /// Ongoing, trap or guard cards stay on the board after play.
    pub fn persists(&self) -> bool {
        self.ongoing || self.trap || self.guard
    }

    pub fn chambers_label(&self) -> String {
        self.valid_chambers.iter().map(|c| c.name()).collect::<Vec<_>>().join(", ")
    }
}

/// Which of the four Intents a play uses. Release carries its pre-committed spend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Intent {
    Press,
    Anchor,
    Release { spend: i32 },
    Sever,
}

impl Intent {
    /// Python `ModeChoice.key`.
    pub fn key(self) -> &'static str {
        match self {
            Intent::Press => "press",
            Intent::Anchor => "anchor",
            Intent::Release { .. } => "release",
            Intent::Sever => "sever",
        }
    }

    /// Python `ModeChoice.label` ("Release 3" etc).
    pub fn label(self) -> String {
        match self {
            Intent::Press => "Press".into(),
            Intent::Anchor => "Anchor".into(),
            Intent::Release { spend } => format!("Release {spend}"),
            Intent::Sever => "Sever".into(),
        }
    }

    pub fn spend(self) -> i32 {
        match self {
            Intent::Release { spend } => spend,
            _ => 0,
        }
    }
}

/// AI personality weights (Python `AiProfile`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct AiProfile {
    pub label: String,
    pub description: String,
    pub aggression: f64,
    pub planning: f64,
    pub disruption: f64,
    pub release_hunger: f64,
    pub randomness: f64,
    pub reroll_bias: f64,
}

/// Which rules revision a game uses.
///
/// * `V1_0_1`: exactly the Python v1.0.1 engine, quirks included (pinned by the parity tests).
/// * `V1_0_2`: v1.0.1 plus fixes (see `docs/RULES_CHANGES.md`):
///   1. Screech's +1 applies only while the Soul is below 0, as its card text says.
///   2. AI urgency thresholds keep their original distance from the goal (they were tuned
///      when the goal was fixed at ±12 and never rescaled for ±18 / ±24).
///   3. Void Thought counts in the AI's Hell-defence set (the Python named a special no card has).
///   4. Silver Thread's support line names Silver Thread, not Small Comfort.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum RulesVersion {
    #[serde(rename = "1.0.1")]
    V1_0_1,
    #[default]
    #[serde(rename = "1.0.2")]
    V1_0_2,
}

impl RulesVersion {
    pub const LATEST: RulesVersion = RulesVersion::V1_0_2;

    pub fn label(self) -> &'static str {
        match self {
            RulesVersion::V1_0_1 => "1.0.1",
            RulesVersion::V1_0_2 => "1.0.2",
        }
    }

    /// "101" / "102" for file names.
    pub fn compact(self) -> &'static str {
        match self {
            RulesVersion::V1_0_1 => "101",
            RulesVersion::V1_0_2 => "102",
        }
    }

    pub fn fixes(self) -> bool {
        self != RulesVersion::V1_0_1
    }
}

/// Game length goal (±12 / ±18 / ±24).
pub const GOAL_QUICK: i32 = 12;
pub const GOAL_STANDARD: i32 = 18;
pub const GOAL_DEEP: i32 = 24;

pub const DECK_SIZE: usize = 10;
pub const LEGACY_DECK_SIZE: usize = 8;
pub const REVEAL_SIZE: usize = 4;
pub const DECK_RARITY_BUDGET: u32 = 24;
pub const DECK_MAX_RARES: u32 = 4;
pub const MAX_VISIBLE_STACK_TOKENS: usize = 9;
pub const BASE_WILL: i32 = 3;
