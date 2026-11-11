//! Card database: loads `data/cards.json` (embedded at compile time) and validates it.
//!
//! The JSON was dumped straight from the v1.0.1 Python module by
//! `tools/py_harness/dump_card_data.py`, so names, emoji and numbers match exactly.
//! Expansion sets can be loaded at runtime with [`CardDb::from_json`].

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::model::{AiProfile, CardDef, CardId, Faction};

const EMBEDDED: &str = include_str!("../data/cards.json");

static DB: LazyLock<CardDb> =
    LazyLock::new(|| CardDb::from_json(EMBEDDED).expect("embedded data/cards.json must be valid"));

/// The built-in v1.0.1 database.
pub fn db() -> &'static CardDb {
    &DB
}

#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("card data JSON is malformed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("duplicate card key {0:?}")]
    DuplicateKey(String),
    #[error("deck {deck:?} references unknown card {card:?}")]
    UnknownDeckCard { deck: String, card: String },
    #[error("deck {deck:?} for {faction:?} contains {card:?} from the other side")]
    WrongFaction { deck: String, faction: Faction, card: String },
    #[error("card {0:?} has no valid chambers")]
    NoChambers(String),
    #[error("more than 65535 cards")]
    TooMany,
}

#[derive(Deserialize)]
struct RawDeck {
    name: String,
    cards: Vec<String>,
}

#[derive(Deserialize)]
struct RawDb {
    rules_version: String,
    cards: Vec<CardDef>,
    decks: HashMap<String, Vec<RawDeck>>,
    ai_profiles: Vec<AiProfile>,
}

/// A named built-in deck.
#[derive(Clone, Debug)]
pub struct DeckList {
    pub name: String,
    pub faction: Faction,
    pub cards: Vec<CardId>,
}

#[derive(Debug)]
pub struct CardDb {
    pub rules_version: String,
    cards: Vec<CardDef>,
    by_key: HashMap<String, CardId>,
    decks: Vec<DeckList>,
    profiles: Vec<AiProfile>,
}

/// Python `normalized_emoji`: trim and drop U+FE0F variation selectors.
pub fn normalize_emoji(s: &str) -> String {
    s.trim().chars().filter(|&c| c != '\u{fe0f}').collect()
}

impl CardDb {
    pub fn from_json(text: &str) -> Result<CardDb, DataError> {
        let raw: RawDb = serde_json::from_str(text)?;
        if raw.cards.len() > u16::MAX as usize {
            return Err(DataError::TooMany);
        }
        let mut by_key = HashMap::new();
        for (i, c) in raw.cards.iter().enumerate() {
            if c.valid_chambers.is_empty() {
                return Err(DataError::NoChambers(c.key.clone()));
            }
            if by_key.insert(c.key.clone(), CardId(i as u16)).is_some() {
                return Err(DataError::DuplicateKey(c.key.clone()));
            }
        }
        let mut decks = Vec::new();
        // Heaven decks first, then Hell, each in file order (Python dict order).
        for faction in Faction::ALL {
            let fkey = match faction {
                Faction::Heaven => "heaven",
                Faction::Hell => "hell",
            };
            for d in raw.decks.get(fkey).map(|v| v.as_slice()).unwrap_or(&[]) {
                let mut ids = Vec::with_capacity(d.cards.len());
                for k in &d.cards {
                    let id = *by_key.get(k).ok_or_else(|| DataError::UnknownDeckCard {
                        deck: d.name.clone(),
                        card: k.clone(),
                    })?;
                    if raw.cards[id.0 as usize].faction != faction {
                        return Err(DataError::WrongFaction { deck: d.name.clone(), faction, card: k.clone() });
                    }
                    ids.push(id);
                }
                decks.push(DeckList { name: d.name.clone(), faction, cards: ids });
            }
        }
        Ok(CardDb { rules_version: raw.rules_version, cards: raw.cards, by_key, decks, profiles: raw.ai_profiles })
    }

    pub fn card(&self, id: CardId) -> &CardDef {
        &self.cards[id.0 as usize]
    }

    pub fn id(&self, key: &str) -> Option<CardId> {
        self.by_key.get(key).copied()
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn all(&self) -> impl Iterator<Item = (CardId, &CardDef)> {
        self.cards.iter().enumerate().map(|(i, c)| (CardId(i as u16), c))
    }

    pub fn faction_cards(&self, f: Faction) -> impl Iterator<Item = (CardId, &CardDef)> {
        self.all().filter(move |(_, c)| c.faction == f)
    }

    pub fn decks(&self, f: Faction) -> impl Iterator<Item = &DeckList> {
        self.decks.iter().filter(move |d| d.faction == f)
    }

    pub fn deck(&self, f: Faction, name: &str) -> Option<&DeckList> {
        self.decks(f).find(|d| d.name == name)
    }

    /// Python `default_deck_name`: the first deck listed for the side.
    pub fn default_deck(&self, f: Faction) -> &DeckList {
        self.decks(f).next().expect("each side has at least one built-in deck")
    }

    pub fn profiles(&self) -> &[AiProfile] {
        &self.profiles
    }

    pub fn profile(&self, label: &str) -> Option<&AiProfile> {
        self.profiles.iter().find(|p| p.label == label)
    }

    /// Glyph → card for one side (exact emoji or variation-selector-normalised).
    /// Both sides use 🧵 (Silver Thread / Tangled Desire), hence per-faction lookup.
    pub fn by_emoji(&self, f: Faction, glyph: &str) -> Option<CardId> {
        let t = glyph.trim();
        let n = normalize_emoji(t);
        self.faction_cards(f)
            .find(|(_, c)| c.emoji == t || normalize_emoji(&c.emoji) == n)
            .map(|(id, _)| id)
    }

    pub fn symbols(&self, ids: &[CardId]) -> String {
        ids.iter().map(|&i| self.card(i).emoji.as_str()).collect::<Vec<_>>().join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_data_loads_and_is_complete() {
        let d = db();
        assert_eq!(d.len(), 40);
        assert_eq!(d.faction_cards(Faction::Heaven).count(), 20);
        assert_eq!(d.faction_cards(Faction::Hell).count(), 20);
        assert_eq!(d.decks(Faction::Heaven).count(), 6);
        assert_eq!(d.decks(Faction::Hell).count(), 6);
        assert_eq!(d.profiles().len(), 5);
        assert_eq!(d.default_deck(Faction::Heaven).name, "Starter Mercy");
        assert_eq!(d.default_deck(Faction::Hell).name, "Starter Torment");
    }

    #[test]
    fn shared_glyph_resolves_per_side() {
        let d = db();
        let heaven = d.by_emoji(Faction::Heaven, "🧵").unwrap();
        let hell = d.by_emoji(Faction::Hell, "🧵").unwrap();
        assert_eq!(d.card(heaven).key, "silver_thread");
        assert_eq!(d.card(hell).key, "tangled_desire");
        // Variation-selector-free spelling still matches.
        assert_eq!(d.card(d.by_emoji(Faction::Heaven, "🕯").unwrap()).key, "candle");
    }

    #[test]
    fn rarity_patch_was_carried_across() {
        let d = db();
        let hope = d.card(d.id("hope").unwrap());
        assert_eq!(hope.rarity, crate::model::Rarity::Uncommon);
        assert_eq!(hope.prices, vec![crate::model::Price::Desperate]);
    }
}
