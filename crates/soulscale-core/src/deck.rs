//! Deck state (draw / reveal / discard), deck legality and custom deck files.

use serde::{Deserialize, Serialize};

use crate::carddb::CardDb;
use crate::model::{
    CardId, DECK_MAX_RARES, DECK_RARITY_BUDGET, DECK_SIZE, Faction, LEGACY_DECK_SIZE, Rarity,
};
use crate::rng::PyRandom;

/// Python `DeckState`.
#[derive(Clone, Debug)]
pub struct DeckState {
    pub original: Vec<CardId>,
    pub draw_pile: Vec<CardId>,
    pub discard_pile: Vec<CardId>,
    pub revealed: Vec<CardId>,
}

impl DeckState {
    /// Builds and shuffles (consumes RNG exactly like the Python constructor).
    pub fn new(cards: &[CardId], rng: &mut PyRandom) -> Self {
        let mut d = DeckState {
            original: cards.to_vec(),
            draw_pile: Vec::new(),
            discard_pile: Vec::new(),
            revealed: Vec::new(),
        };
        d.reset(rng);
        d
    }

    pub fn reset(&mut self, rng: &mut PyRandom) {
        self.draw_pile = self.original.clone();
        self.discard_pile.clear();
        self.revealed.clear();
        rng.shuffle(&mut self.draw_pile);
    }

    fn refill_if_needed(&mut self, rng: &mut PyRandom) {
        if self.draw_pile.is_empty() && !self.discard_pile.is_empty() {
            self.draw_pile = std::mem::take(&mut self.discard_pile);
            rng.shuffle(&mut self.draw_pile);
        }
    }

    /// Discard the old reveal, then reveal `n` fresh glyphs.
    pub fn reveal(&mut self, n: usize, rng: &mut PyRandom) -> Vec<CardId> {
        self.discard_unplayed_reveal();
        self.revealed.clear();
        for _ in 0..n {
            self.refill_if_needed(rng);
            if !self.draw_pile.is_empty() {
                let c = self.draw_pile.remove(0);
                self.revealed.push(c);
            }
        }
        self.revealed.clone()
    }

    pub fn discard_revealed_card(&mut self, id: CardId) {
        if let Some(pos) = self.revealed.iter().position(|&c| c == id) {
            self.revealed.remove(pos);
        }
        self.discard_pile.push(id);
    }

    pub fn discard_unplayed_reveal(&mut self) {
        self.discard_pile.append(&mut self.revealed);
    }
}

/// Deck legality summary (rarity budget, rares, size).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Legality {
    pub size: usize,
    pub cost: u32,
    pub rares: u32,
    pub issues: Vec<String>,
}

impl Legality {
    pub fn is_legal(&self) -> bool {
        self.issues.is_empty()
    }

    /// Python `deck_legality_label`.
    pub fn label(&self) -> String {
        let base = format!(
            "{}/{} glyphs • budget {}/{} • rares {}/{}",
            self.size, DECK_SIZE, self.cost, DECK_RARITY_BUDGET, self.rares, DECK_MAX_RARES
        );
        if self.issues.is_empty() { format!("{base} • legal") } else { format!("{base} • ILLEGAL: {}", self.issues.join("; ")) }
    }
}

/// Python `deck_legality_issues`.
pub fn legality(db: &CardDb, cards: &[CardId]) -> Legality {
    let cost: u32 = cards.iter().map(|&c| db.card(c).rarity.cost()).sum();
    let rares = cards.iter().filter(|&&c| db.card(c).rarity == Rarity::Rare).count() as u32;
    let mut issues = Vec::new();
    if cards.len() != DECK_SIZE {
        issues.push(format!("needs exactly {DECK_SIZE} glyphs; has {}", cards.len()));
    }
    if cost > DECK_RARITY_BUDGET {
        issues.push(format!("rarity budget {cost}/{DECK_RARITY_BUDGET} is too high"));
    }
    if rares > DECK_MAX_RARES {
        issues.push(format!("rare limit {rares}/{DECK_MAX_RARES} is too high"));
    }
    Legality { size: cards.len(), cost, rares, issues }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DeckParseError {
    #[error(
        "Deck must contain exactly {DECK_SIZE} non-empty emoji lines for {faction}. Legacy {LEGACY_DECK_SIZE}-glyph decks are accepted and auto-padded with stable commons. This file contains {found} usable lines. Blank lines and lines starting with # are ignored."
    )]
    WrongCount { faction: &'static str, found: usize },
    #[error("The deck contains glyphs that do not belong to {side}, or are not known cards: {lines}")]
    UnknownGlyphs { side: &'static str, lines: String },
    #[error("The deck glyphs are recognized, but the deck is not legal under v1.0.1 rarity rules: {0}")]
    Illegal(String),
}

/// Python `parse_custom_deck`: one emoji per usable line; legacy 8-glyph decks are padded
/// with the side's Common cards in database order.
pub fn parse_custom_deck(db: &CardDb, faction: Faction, text: &str) -> Result<Vec<CardId>, DeckParseError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let usable: Vec<(usize, &str)> = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'))
        .collect();
    if usable.len() != DECK_SIZE && usable.len() != LEGACY_DECK_SIZE {
        return Err(DeckParseError::WrongCount { faction: faction.label(), found: usable.len() });
    }
    let mut keys = Vec::new();
    let mut invalid = Vec::new();
    for (line_no, t) in usable {
        match db.by_emoji(faction, t) {
            Some(id) => keys.push(id),
            None => invalid.push(format!("line {line_no}: {t:?}")),
        }
    }
    if !invalid.is_empty() {
        return Err(DeckParseError::UnknownGlyphs { side: faction.short(), lines: invalid.join(", ") });
    }
    if keys.len() == LEGACY_DECK_SIZE {
        let pad: Vec<CardId> =
            db.faction_cards(faction).filter(|(_, c)| c.rarity == Rarity::Common).map(|(id, _)| id).collect();
        while keys.len() < DECK_SIZE {
            keys.push(pad[(keys.len() - LEGACY_DECK_SIZE) % pad.len()]);
        }
    }
    let leg = legality(db, &keys);
    if !leg.is_legal() {
        return Err(DeckParseError::Illegal(leg.issues.join("; ")));
    }
    Ok(keys)
}

/// One emoji per line, the format the Python deck editor writes.
pub fn deck_to_text(db: &CardDb, cards: &[CardId]) -> String {
    let mut s = String::new();
    for &c in cards {
        s.push_str(&db.card(c).emoji);
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carddb::db;

    #[test]
    fn all_builtin_decks_are_legal() {
        let d = db();
        for f in Faction::ALL {
            for deck in d.decks(f) {
                let l = legality(d, &deck.cards);
                assert!(l.is_legal(), "{} illegal: {:?}", deck.name, l.issues);
            }
        }
    }

    #[test]
    fn custom_deck_roundtrip_and_errors() {
        let d = db();
        let deck = &d.deck(Faction::Heaven, "Starter Mercy").unwrap().cards;
        let txt = format!("# my deck\n\n{}", deck_to_text(d, deck));
        assert_eq!(parse_custom_deck(d, Faction::Heaven, &txt).unwrap(), *deck);
        // Hell glyphs are rejected for Heaven.
        let bad = "🦷\n".repeat(10);
        assert!(matches!(parse_custom_deck(d, Faction::Heaven, &bad), Err(DeckParseError::UnknownGlyphs { .. })));
        // Too many rares.
        let rares = "🌊\n".repeat(10);
        assert!(matches!(parse_custom_deck(d, Faction::Heaven, &rares), Err(DeckParseError::Illegal(_))));
        // Legacy 8 gets padded to 10.
        let legacy = "🕯️\n".repeat(8);
        assert_eq!(parse_custom_deck(d, Faction::Heaven, &legacy).unwrap().len(), 10);
    }

    #[test]
    fn reveal_refills_from_discard() {
        let d = db();
        let mut rng = PyRandom::new(1);
        let cards = d.deck(Faction::Hell, "Starter Torment").unwrap().cards.clone();
        let mut ds = DeckState::new(&cards, &mut rng);
        for _ in 0..10 {
            let r = ds.reveal(4, &mut rng);
            assert_eq!(r.len(), 4);
            ds.discard_revealed_card(r[0]);
            assert_eq!(ds.draw_pile.len() + ds.discard_pile.len() + ds.revealed.len(), 10);
        }
    }
}
