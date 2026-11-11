//! Pre-clash modifiers (guards, traps, Fragile, Calm Breath, Tangled Desire) and the clash.

use crate::events::GameEvent;
use crate::game::{Game, PendingPlay};
use crate::model::{Price, Special};
use crate::state::Seat;

fn weaken(p: &mut PendingPlay, by: i32) {
    p.power = (p.power - by).max(0);
    p.amount = (p.amount - by).max(0);
}

impl Game {
    /// Python `apply_pre_clash_modifiers`.
    pub(crate) fn apply_pre_clash_modifiers(&mut self, p1: &mut PendingPlay, p2: &mut PendingPlay) {
        for play in [&mut *p1, &mut *p2] {
            let card = self.card(play.card);
            let opposing = card.faction.opponent();
            let ci = play.chamber.idx();
            let snapshot = self.st.board[ci].clone();
            for effect in snapshot {
                let ec = self.card(effect.card);
                if effect.faction == opposing && ec.guard {
                    weaken(play, 2);
                    self.log(format!(
                        "{} guards {}: {} loses 2 power.",
                        ec.short(),
                        play.chamber.name(),
                        card.short()
                    ));
                    self.emit(GameEvent::Modifier { seat: play.seat, source: ec.key.clone(), delta: -2 });
                }
                if effect.faction == opposing && ec.trap {
                    weaken(play, 3);
                    if let Some(pos) = self.st.board[ci].iter().position(|e| *e == effect) {
                        self.st.board[ci].remove(pos);
                    }
                    self.log(format!(
                        "{} springs in {}: {} loses 3 power.",
                        ec.short(),
                        play.chamber.name(),
                        card.short()
                    ));
                    self.emit(GameEvent::TrapSprung { chamber: play.chamber, card: ec.key.clone(), victim: play.seat });
                }
            }
        }

        if p1.chamber != p2.chamber {
            return;
        }
        let c1 = self.card(p1.card);
        let c2 = self.card(p2.card);
        if c1.has_price(Price::Fragile) {
            weaken(p1, 1);
            self.log(format!("{} is Fragile and loses 1 force when contested.", c1.short()));
            self.emit(GameEvent::Modifier { seat: Seat::P1, source: "fragile".into(), delta: -1 });
        }
        if c2.has_price(Price::Fragile) {
            weaken(p2, 1);
            self.log(format!("{} is Fragile and loses 1 force when contested.", c2.short()));
            self.emit(GameEvent::Modifier { seat: Seat::P2, source: "fragile".into(), delta: -1 });
        }
        if c1.special == Special::Calm {
            weaken(p2, 2);
            self.log(format!("Calm Breath softens {} play by 2.", c2.faction.short()));
            self.emit(GameEvent::Modifier { seat: Seat::P2, source: c1.key.clone(), delta: -2 });
        }
        if c2.special == Special::Calm {
            weaken(p1, 2);
            self.log(format!("{} softens {} play by 2.", c2.short(), c1.faction.short()));
            self.emit(GameEvent::Modifier { seat: Seat::P1, source: c2.key.clone(), delta: -2 });
        }
        if c1.special == Special::Tangle {
            weaken(p2, 2);
            self.log(format!("Tangled Desire weakens {} play by 2.", c2.faction.short()));
            self.emit(GameEvent::Modifier { seat: Seat::P2, source: c1.key.clone(), delta: -2 });
        }
        if c2.special == Special::Tangle {
            weaken(p1, 2);
            self.log(format!("Tangled Desire weakens {} play by 2.", c1.faction.short()));
            self.emit(GameEvent::Modifier { seat: Seat::P1, source: c2.key.clone(), delta: -2 });
        }
    }

    /// Same Chamber → compare power; the loser's (or both, on a tie) amount is halved.
    pub(crate) fn resolve_clash(&mut self, p1: &mut PendingPlay, p2: &mut PendingPlay, n1: &str, n2: &str) {
        if p1.chamber != p2.chamber {
            return;
        }
        self.log(format!(
            "Clash in {} {}: {n1} {} vs {n2} {}.",
            p1.chamber.emoji(),
            p1.chamber.name(),
            p1.power,
            p2.power
        ));
        let winner = if p1.power > p2.power {
            p2.half = true;
            self.log(format!("{n1} wins the clash. {n2} effect is halved."));
            Some(Seat::P1)
        } else if p2.power > p1.power {
            p1.half = true;
            self.log(format!("{n2} wins the clash. {n1} effect is halved."));
            Some(Seat::P2)
        } else {
            p1.half = true;
            p2.half = true;
            self.log("The clash is tied. Both effects are halved.");
            None
        };
        self.emit(GameEvent::Clash { chamber: p1.chamber, power: [p1.power, p2.power], winner });
    }
}
