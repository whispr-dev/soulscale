//! Strain and Purgatory Events (rulebook §18). Every 3 Strain spends 3 and rolls a d6.

use crate::events::GameEvent;
use crate::game::Game;
use crate::model::{Chamber, Faction};

/// (emoji, name) for each d6 face.
pub const PURGATORY_EVENTS: [(&str, &str); 6] = [
    ("🕳️", "Collapse"),
    ("🌤️", "Reprieve"),
    ("🪞", "Flashback"),
    ("🕰️", "Stasis"),
    ("🔥", "Intensification"),
    ("⚖️", "Judgement"),
];

impl Game {
    pub(crate) fn resolve_strain_events(&mut self) {
        self.emit(GameEvent::Strain { value: self.st.strain });
        while self.st.strain >= 3 {
            self.st.strain -= 3;
            let roll = self.rng.d6();
            let (emoji, name) = PURGATORY_EVENTS[(roll - 1) as usize];
            self.emit(GameEvent::Purgatory { roll, name: name.into(), emoji: emoji.into() });
            match roll {
                1 => {
                    self.st.next_devil_bonus += 2;
                    self.log("Purgatory Event — 🕳️ Collapse: the next Hell push gets +2.");
                }
                2 => {
                    self.st.next_angel_bonus += 2;
                    self.log("Purgatory Event — 🌤️ Reprieve: the next Heaven push gets +2.");
                }
                3 => {
                    self.st.conditions[Chamber::Memory.idx()].clear();
                    self.st.cut_stack(Faction::Heaven, Chamber::Memory, 2);
                    self.st.cut_stack(Faction::Hell, Chamber::Memory, 2);
                    self.log("Purgatory Event — 🪞 Flashback: Memory conditions clear and both Memory stacks lose 2.");
                }
                4 => {
                    self.st.next_soul_cap = Some(2);
                    self.log("Purgatory Event — 🕰️ Stasis: next round's Soul movement is capped at ±2.");
                }
                5 => {
                    self.st.next_all_bonus += 1;
                    self.log("Purgatory Event — 🔥 Intensification: the next active effect gets +1.");
                }
                _ => {
                    if self.st.soul > 0 {
                        self.st.apply_next_will_bonus(Faction::Hell, 2);
                        self.log("Purgatory Event — ⚖️ Judgement: Hell is behind and gets +2 Will next round.");
                    } else if self.st.soul < 0 {
                        self.st.apply_next_will_bonus(Faction::Heaven, 2);
                        self.log("Purgatory Event — ⚖️ Judgement: Heaven is behind and gets +2 Will next round.");
                    } else {
                        self.st.apply_next_will_bonus(Faction::Heaven, 1);
                        self.st.apply_next_will_bonus(Faction::Hell, 1);
                        self.log("Purgatory Event — ⚖️ Judgement: both sides get +1 Will next round.");
                    }
                }
            }
            self.emit(GameEvent::Strain { value: self.st.strain });
        }
    }
}
