//! v1.0.1 movement resolver: cards declare which way they may move the Soul.

use crate::game::Game;
use crate::model::{CardDef, Faction, MovementType};

/// Signed direction a card may move the Soul (Python `movement_sign`).
pub fn movement_sign(card: &CardDef, kind: Option<MovementType>) -> i32 {
    match kind.unwrap_or(card.movement_type) {
        MovementType::Ownerward => card.faction.sign(),
        MovementType::Opponentward => -card.faction.sign(),
        MovementType::Heavenward => Faction::Heaven.sign(),
        MovementType::Hellward => Faction::Hell.sign(),
        MovementType::Neutral => 0,
    }
}

/// Non-negative magnitude → signed movement (Python `signed_card_movement`).
pub fn signed_card_movement(card: &CardDef, amount: i32) -> i32 {
    if amount <= 0 { 0 } else { amount * movement_sign(card, None) }
}

impl Game {
    /// Python `movement_guard`: block neutral movement, correct accidental wrong-way
    /// movement, and log intentional opponentward movement.
    pub(crate) fn movement_guard(&mut self, card: &CardDef, mv: i32) -> i32 {
        if mv == 0 {
            return 0;
        }
        let expected = movement_sign(card, None);
        if expected == 0 {
            self.log(format!(
                "Movement resolver blocked {}: it is declared neutral but produced {mv:+}.",
                card.short()
            ));
            return 0;
        }
        if mv * expected < 0 {
            let corrected = mv.abs() * expected;
            let target = if corrected > 0 { "Heaven" } else { "Hell" };
            self.log(format!(
                "Movement resolver corrected {}: declared {} movement, so {mv:+} becomes {corrected:+} toward {target}.",
                card.short(),
                card.movement_type.label()
            ));
            return corrected;
        }
        if card.movement_type == MovementType::Opponentward {
            let target = if mv > 0 { "Heaven" } else { "Hell" };
            self.log(format!("{} intentionally moves opponentward: {mv:+} toward {target}.", card.short()));
        }
        mv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carddb::db;

    fn with_movement(key: &str, mt: MovementType) -> CardDef {
        let d = db();
        let mut c = d.card(d.id(key).unwrap()).clone();
        c.movement_type = mt;
        c
    }

    #[test]
    fn declared_types_set_direction() {
        assert_eq!(signed_card_movement(&with_movement("candle", MovementType::Ownerward), 3), 3);
        assert_eq!(signed_card_movement(&with_movement("ache", MovementType::Ownerward), 3), -3);
        assert_eq!(signed_card_movement(&with_movement("ache", MovementType::Opponentward), 2), 2);
        assert_eq!(signed_card_movement(&with_movement("candle", MovementType::Hellward), 2), -2);
        assert_eq!(signed_card_movement(&with_movement("candle", MovementType::Neutral), 2), 0);
        assert_eq!(signed_card_movement(&with_movement("candle", MovementType::Ownerward), 0), 0);
    }
}
