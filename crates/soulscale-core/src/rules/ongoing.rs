//! Ongoing board effects (Python `apply_ongoing_effects`). They fire after both active
//! plays, including on the round they are placed. A Chamber with 6+ friendly stack gives
//! its ongoing engine +1.

use crate::events::GameEvent;
use crate::game::Game;
use crate::model::{Chamber, Special};
use crate::rules::movement::signed_card_movement;

impl Game {
    pub(crate) fn apply_ongoing_effects(&mut self) -> i32 {
        let mut total = 0;
        for chamber in Chamber::ALL {
            let effects = self.st.board[chamber.idx()].clone();
            for effect in effects {
                let card = self.card(effect.card);
                if card.ongoing_move != 0 {
                    let stack = self.st.stack_total(card.faction, chamber);
                    let bonus = i32::from(stack >= 6);
                    let mv = signed_card_movement(card, card.ongoing_move + bonus);
                    total += mv;
                    self.log(format!("Ongoing {} in {} contributes {mv:+}.", card.short(), chamber.name()));
                    self.emit(GameEvent::Ongoing { chamber, card: card.key.clone(), faction: card.faction, amount: mv });
                }
                if card.special == Special::EndlessWait {
                    self.st.next_heaven_cap = Some(2);
                }
            }
        }
        total
    }
}
