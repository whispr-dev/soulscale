//! Applying a resolved play (Python `apply_play`): stack spend/cut, card specials,
//! Delayed/Echo scheduling, the movement guard, stack growth, Binding locks and
//! board placement.

use crate::events::GameEvent;
use crate::game::{Game, PendingPlay};
use crate::model::{CardDef, Chamber, Condition, Faction, Price, Special};
use crate::rules::movement::signed_card_movement;
use crate::state::{BoardEffect, DelayedMove};

impl Game {
    /// Returns the signed Soul movement this play contributes this round.
    pub(crate) fn apply_play(&mut self, play: &PendingPlay) -> i32 {
        let card: &'static CardDef = self.card(play.card);
        let f = card.faction;
        let chamber = play.chamber;
        let cn = chamber.name();
        let short = card.short();
        let seat = play.seat;

        if play.power <= 0 {
            self.log(format!("{short} fizzles; its power was reduced to 0."));
            self.emit(GameEvent::Fizzle { seat });
            return 0;
        }

        let amount = if play.half { play.amount / 2 } else { play.amount };

        if play.stack_spend != 0 {
            let spent = self.st.consume_stack(f, chamber, play.stack_spend);
            if spent != 0 {
                self.log(format!("{} releases {spent} stack from {cn}.", f.short()));
                self.emit(GameEvent::StackSpent { faction: f, chamber, amount: spent });
            }
        }

        if play.sever_amount != 0 {
            let cut = self.st.cut_stack(f.opponent(), chamber, play.sever_amount);
            if cut != 0 {
                self.log(format!("{short} severs {cut} enemy stack in {cn}."));
                self.emit(GameEvent::StackCut { faction: f.opponent(), chamber, amount: cut, by: card.key.clone() });
            } else {
                self.log(format!("{short} tries to Sever, but finds no enemy stack in {cn}."));
            }
        }

        let heaven = Faction::Heaven.sign();
        let hell = Faction::Hell.sign();
        let mut mv = 0;
        match card.special {
            Special::Miracle => mv += self.resolve_miracle(play, amount),
            Special::Serpent => mv += self.resolve_serpent(play, amount),
            Special::Bell => {
                if let Some(removed) = self.remove_one_effect(chamber, Faction::Hell, card) {
                    self.log(format!("Bell of Mercy cancels {} in {cn}.", self.card(removed.card).short()));
                } else if amount > 0 {
                    mv += amount.max(1) * heaven;
                    self.log("Bell of Mercy finds no ongoing Devil effect, so it rings as a Heaven push.");
                } else {
                    mv += heaven;
                    self.log("Bell of Mercy finds no ongoing Devil effect, so it nudges the Soul +1.");
                }
            }
            Special::Calm => {
                if let Some(removed) = self.remove_one_effect(chamber, Faction::Hell, card) {
                    self.log(format!("Calm Breath cleanses {} from {cn}.", self.card(removed.card).short()));
                } else if amount > 0 {
                    mv += amount * heaven;
                }
            }
            Special::Rest => {
                if amount > 0 {
                    mv += amount * heaven;
                }
                if let Some(removed) = self.remove_one_effect(chamber, Faction::Hell, card) {
                    self.log(format!("Rest removes {} from {cn}.", self.card(removed.card).short()));
                }
                if self.st.remove_condition(chamber, Condition::Cold) {
                    self.log("Rest thaws Cold.");
                    self.emit(GameEvent::ConditionRemoved { chamber, condition: Condition::Cold });
                }
            }
            Special::LovedMemory => {
                if chamber == Chamber::Memory
                    && let Some(removed) = self.remove_one_effect(chamber, Faction::Hell, card)
                {
                    self.log(format!("Loved Memory dissolves {} in Memory.", self.card(removed.card).short()));
                }
                if amount > 0 {
                    mv += amount * heaven;
                }
            }
            Special::Acceptance => {
                if let Some(removed) = self.remove_one_effect(chamber, Faction::Hell, card) {
                    self.log(format!("Acceptance releases {} from {cn}.", self.card(removed.card).short()));
                }
                if amount > 0 {
                    mv += amount * heaven;
                }
            }
            Special::Burden => {
                if amount > 0 {
                    mv += amount * hell;
                }
                self.st.apply_next_will_penalty(f.opponent(), 1);
                self.emit(GameEvent::WillNext { faction: f.opponent(), delta: -1 });
                self.log("Burden reduces the opponent's Will next round by 1.");
            }
            Special::WatchingEye => {
                mv += amount.max(1) * hell;
                self.st.apply_next_will_bonus(Faction::Hell, 1);
                self.emit(GameEvent::WillNext { faction: Faction::Hell, delta: 1 });
                self.log("Watching Eye gives Hell +1 Will next round.");
            }
            Special::Thorn => {
                if amount > 0 {
                    mv += amount * hell;
                }
                if !self.st.has_condition(chamber, Condition::Thorn) {
                    self.st.conditions[chamber.idx()].push(Condition::Thorn);
                    self.log(format!("Memory Thorn leaves Thorn in {cn}."));
                    self.emit(GameEvent::ConditionAdded { chamber, condition: Condition::Thorn });
                }
            }
            Special::Cold => {
                if amount > 0 {
                    mv += amount * hell;
                }
                if !self.st.has_condition(chamber, Condition::Cold) {
                    self.st.conditions[chamber.idx()].push(Condition::Cold);
                    self.log(format!("Cold Room leaves Cold in {cn}."));
                    self.emit(GameEvent::ConditionAdded { chamber, condition: Condition::Cold });
                }
            }
            Special::EndlessWait => {
                self.st.next_heaven_cap = Some(2);
                if amount > 0 {
                    mv += amount * hell;
                }
                self.log("Endless Wait will cap Heaven movement next round.");
            }
            Special::SupportNext => {
                if amount > 0 {
                    mv += amount * heaven;
                }
                self.st.support_bonus[Faction::Heaven.idx()] += 1;
                if self.cfg.rules.fixes() {
                    self.log(format!("{} gives the next Heaven effect +1.", card.name));
                } else {
                    self.log("Small Comfort gives the next Heaven effect +1.");
                }
            }
            Special::Sweetness => {
                if amount > 0 {
                    mv += amount * heaven;
                }
                if self.st.soul > 0 {
                    self.st.apply_next_will_bonus(Faction::Heaven, 1);
                    self.emit(GameEvent::WillNext { faction: Faction::Heaven, delta: 1 });
                    self.log("Sweetness gives Heaven +1 Will next round.");
                }
            }
            _ => {
                if amount > 0 {
                    mv += amount * f.sign();
                }
            }
        }

        if card.has_price(Price::Delayed) && mv != 0 {
            self.st.next_delayed_movements.push(DelayedMove { amount: mv, label: short.clone() });
            self.log(format!("{short} is Delayed: {mv:+} is scheduled for next round instead of moving now."));
            self.emit(GameEvent::Scheduled { amount: mv, label: short.clone(), echo: false });
            mv = 0;
        }

        if card.has_price(Price::Echo) && mv != 0 {
            let echo = signed_card_movement(card, 1);
            let label = format!("Echo of {short}");
            self.st.next_delayed_movements.push(DelayedMove { amount: echo, label: label.clone() });
            self.log(format!("{short} Echo schedules {echo:+} for next round."));
            self.emit(GameEvent::Scheduled { amount: echo, label, echo: true });
        }

        mv = self.movement_guard(card, mv);

        if play.stack_add != 0 {
            self.st.add_stack(f, chamber, play.card, &card.aspect, play.stack_add);
            self.log(format!(
                "{} adds {} stack to {cn} from {} {}.",
                f.short(),
                play.stack_add,
                card.emoji,
                card.aspect
            ));
            self.emit(GameEvent::StackAdded { faction: f, chamber, card: card.key.clone(), value: play.stack_add });
        }

        if card.has_price(Price::Binding) {
            self.st.lock_chamber_next_round(f, chamber);
            self.log(format!("{short} is Binding: {} cannot use {cn} next round.", f.short()));
        }

        if card.persists() {
            self.place_board_effect(card, play.card, chamber);
        }

        if mv != 0 {
            let side = if mv > 0 { "Heaven" } else { "Hell" };
            let extra = if play.resonance_bonus != 0 {
                format!(" Resonance bonus +{}.", play.resonance_bonus)
            } else {
                String::new()
            };
            self.log(format!("{short} contributes {mv:+} toward {side}.{extra}"));
        } else if card.persists() {
            self.log(format!("{short} takes hold in {} {cn}.", chamber.emoji()));
        } else if play.stack_add != 0 || play.sever_amount != 0 || play.stack_spend != 0 {
            self.log(format!("{short} changes the stack state but has no direct Soul movement."));
        } else {
            self.log(format!("{short} has no direct Soul movement."));
        }
        self.emit(GameEvent::Contribution { seat, card: card.key.clone(), amount: mv });
        mv
    }

    fn resolve_miracle(&mut self, play: &PendingPlay, amount: i32) -> i32 {
        let roll = self.rng.d6();
        let bonus = (amount - play.intensity).max(0);
        self.emit(GameEvent::Wild { seat: play.seat, card: self.card(play.card).key.clone(), roll });
        if roll <= 2 {
            self.st.apply_next_will_bonus(Faction::Heaven, 1);
            self.emit(GameEvent::WillNext { faction: Faction::Heaven, delta: 1 });
            self.log(format!("Little Miracle rolls {roll}: Heaven gains +1 Will next round."));
            bonus
        } else if roll <= 4 {
            self.log(format!("Little Miracle rolls {roll}: a clean Heaven push."));
            2 + bonus
        } else {
            self.log(format!("Little Miracle rolls {roll}: blazing Heaven push."));
            4 + bonus
        }
    }

    fn resolve_serpent(&mut self, play: &PendingPlay, amount: i32) -> i32 {
        let roll = self.rng.d6();
        let bonus = (amount - play.intensity).max(0);
        self.emit(GameEvent::Wild { seat: play.seat, card: self.card(play.card).key.clone(), roll });
        if roll <= 2 {
            self.st.apply_next_will_penalty(Faction::Heaven, 1);
            self.emit(GameEvent::WillNext { faction: Faction::Heaven, delta: -1 });
            self.log(format!("Whispering Serpent rolls {roll}: Heaven loses 1 Will next round."));
            -bonus
        } else if roll <= 4 {
            self.log(format!("Whispering Serpent rolls {roll}: a clean Hell push."));
            -(2 + bonus)
        } else {
            self.log(format!("Whispering Serpent rolls {roll}: venomous Hell push."));
            -(4 + bonus)
        }
    }

    /// Ongoing / guard / trap placement; the same card from the same side refreshes.
    pub(crate) fn place_board_effect(&mut self, card: &CardDef, id: crate::model::CardId, chamber: Chamber) {
        let exists = self.st.board[chamber.idx()].iter().any(|e| e.faction == card.faction && e.card == id);
        if exists {
            self.log(format!("{} refreshes its presence in {}.", card.short(), chamber.name()));
            self.emit(GameEvent::EffectPlaced { chamber, faction: card.faction, card: card.key.clone(), refreshed: true });
            return;
        }
        self.st.board[chamber.idx()].push(BoardEffect { faction: card.faction, card: id });
        self.emit(GameEvent::EffectPlaced { chamber, faction: card.faction, card: card.key.clone(), refreshed: false });
    }

    /// Python `remove_one_effect`: removes the first board effect of `faction` here. If none
    /// and `faction` is Hell, cleanses Thorn (else Cold) instead and still returns `None`.
    pub(crate) fn remove_one_effect(&mut self, chamber: Chamber, faction: Faction, by: &CardDef) -> Option<BoardEffect> {
        let ci = chamber.idx();
        if let Some(pos) = self.st.board[ci].iter().position(|e| e.faction == faction) {
            let e = self.st.board[ci].remove(pos);
            self.emit(GameEvent::EffectRemoved {
                chamber,
                faction,
                card: self.card(e.card).key.clone(),
                by: by.key.clone(),
            });
            return Some(e);
        }
        if faction == Faction::Hell {
            for cond in [Condition::Thorn, Condition::Cold] {
                if self.st.remove_condition(chamber, cond) {
                    self.log(format!("{} is cleansed from {}.", cond.name(), chamber.name()));
                    self.emit(GameEvent::ConditionRemoved { chamber, condition: cond });
                    return None;
                }
            }
        }
        None
    }
}
