//! Building a pending play (Python `make_pending_play`): power and amount from the card
//! and the intensity roll, then one-shot bonuses, card-specific modifiers, Chamber
//! conditions, price keywords and finally the Intent's stack maths.

use crate::events::GameEvent;
use crate::game::{Game, PendingPlay, PlayCommit};
use crate::model::{Chamber, Condition, Faction, Intent, Price, Special};
use crate::rules::intent;
use crate::state::Seat;

impl Game {
    pub(crate) fn make_pending_play(&mut self, seat: Seat, p: &PlayCommit) -> PendingPlay {
        let card = self.card(p.card);
        let f = card.faction;
        let chamber = p.chamber;
        let intensity = p.intensity();
        let mut power = card.base_power + intensity;
        let mut amount = if card.base_move != 0 { card.base_move + intensity } else { 0 };
        let stack_add: i32;
        let mut stack_spend = 0;
        let mut sever_amount = 0;
        let mut resonance_bonus = 0;
        let short = card.short();

        // Global support/event bonuses before intent math.
        let sb = self.st.support_bonus[f.idx()];
        if sb > 0 && amount > 0 {
            amount += sb;
            power += sb;
            self.log(format!("{} support adds +{sb} to {short}.", f.label()));
            self.st.support_bonus[f.idx()] = 0;
            self.emit(GameEvent::Modifier { seat, source: "support".into(), delta: sb });
        }
        if f == Faction::Heaven && amount > 0 && self.st.next_angel_bonus != 0 {
            let b = self.st.next_angel_bonus;
            amount += b;
            power += b;
            self.log(format!("Reprieve bonus adds +{b} to {short}."));
            self.st.next_angel_bonus = 0;
            self.emit(GameEvent::Modifier { seat, source: "reprieve".into(), delta: b });
        }
        if f == Faction::Hell && amount > 0 && self.st.next_devil_bonus != 0 {
            let b = self.st.next_devil_bonus;
            amount += b;
            power += b;
            self.log(format!("Collapse bonus adds +{b} to {short}."));
            self.st.next_devil_bonus = 0;
            self.emit(GameEvent::Modifier { seat, source: "collapse".into(), delta: b });
        }
        if amount > 0 && self.st.next_all_bonus != 0 {
            let b = self.st.next_all_bonus;
            amount += b;
            power += b;
            self.log(format!("Intensification adds +{b} to {short}."));
            self.st.next_all_bonus = 0;
            self.emit(GameEvent::Modifier { seat, source: "intensification".into(), delta: b });
        }

        // Card-specific modifiers.
        if card.special == Special::BodyBonus && chamber == Chamber::Body {
            amount += 1;
            power += 1;
            self.emit(GameEvent::Modifier { seat, source: card.key.clone(), delta: 1 });
        }
        if matches!(card.special, Special::BehindBonus | Special::Screech) {
            let soul = self.st.soul;
            let boosted = if card.special == Special::Screech && self.cfg.rules.fixes() {
                // v1.0.2: as printed, "stronger if the Soul is already below 0".
                soul < 0
            } else {
                (f == Faction::Heaven && soul < 0)
                    || (f == Faction::Hell && soul > 0)
                    || (card.special == Special::Screech && soul < 0)
            };
            if boosted {
                amount += 1;
                power += 1;
                self.emit(GameEvent::Modifier { seat, source: card.key.clone(), delta: 1 });
            }
        }

        // Conditions.
        if f == Faction::Heaven
            && self.st.has_condition(chamber, Condition::Cold)
            && matches!(card.key.as_str(), "candle" | "small_comfort" | "sweetness")
        {
            power = (power - 1).max(0);
            amount = (amount - 1).max(0);
            self.log(format!("Cold weakens {short} by 1 in {}.", chamber.name()));
            self.emit(GameEvent::Modifier { seat, source: "cold".into(), delta: -1 });
        }
        if f == Faction::Heaven && self.st.has_condition(chamber, Condition::Thorn) && card.key == "loved_memory" {
            power = (power - 2).max(0);
            amount = (amount - 2).max(0);
            self.st.remove_condition(chamber, Condition::Thorn);
            self.log("Memory Thorn weakens Loved Memory by 2, then fades.");
            self.emit(GameEvent::Modifier { seat, source: "thorn".into(), delta: -2 });
            self.emit(GameEvent::ConditionRemoved { chamber, condition: Condition::Thorn });
        }

        // v1.0 price keywords.
        if card.has_price(Price::Heavy) {
            if self.st.spend_will_for_faction(f, 1) {
                self.log(format!("{short} is Heavy: {} spends 1 Will to play it at full force.", f.short()));
            } else {
                power = (power - 1).max(0);
                amount = (amount - 1).max(0);
                self.log(format!("{short} is Heavy, but {} has no Will; it loses 1 force.", f.short()));
                self.emit(GameEvent::Modifier { seat, source: "heavy".into(), delta: -1 });
            }
        }
        if card.has_price(Price::Desperate) {
            let soul = self.st.soul;
            let behind = (f == Faction::Heaven && soul < 0) || (f == Faction::Hell && soul > 0);
            let ahead = (f == Faction::Heaven && soul > 0) || (f == Faction::Hell && soul < 0);
            if behind {
                amount += 1;
                power += 1;
                self.log(format!("{short} is Desperate: +1 while {} is behind.", f.short()));
                self.emit(GameEvent::Modifier { seat, source: "desperate".into(), delta: 1 });
            } else if ahead {
                amount = (amount - 1).max(0);
                power = (power - 1).max(0);
                self.log(format!("{short} is Desperate: -1 while {} is already ahead.", f.short()));
                self.emit(GameEvent::Modifier { seat, source: "desperate".into(), delta: -1 });
            }
        }
        if card.has_price(Price::Volatile) {
            if p.raw <= 2 {
                amount = (amount - 2).max(0);
                power = (power - 2).max(0);
                self.st.strain += 1;
                self.log(format!("{short} is Volatile: weak roll backfires, -2 force and +1 Strain."));
                self.emit(GameEvent::Modifier { seat, source: "volatile".into(), delta: -2 });
            } else if p.raw >= 5 {
                amount += 1;
                power += 1;
                self.log(format!("{short} is Volatile: surge roll adds +1 force."));
                self.emit(GameEvent::Modifier { seat, source: "volatile".into(), delta: 1 });
            }
        }

        // Intent stack math.
        let own = self.st.stack_total(f, chamber);
        let enemy = self.st.stack_total(f.opponent(), chamber);
        let harmony = self.st.harmony_count(f, chamber, &card.aspect);
        match p.intent {
            Intent::Press => {
                resonance_bonus = intent::press_bonus(own, harmony, enemy);
                if resonance_bonus != 0 {
                    amount += resonance_bonus;
                    power += resonance_bonus;
                    self.emit(GameEvent::Modifier { seat, source: "press".into(), delta: resonance_bonus });
                }
                stack_add = card.stack_value;
            }
            Intent::Anchor => {
                // A true investment: still contests a clash, never moves the Soul now.
                amount = 0;
                power = (power - 1).max(0);
                stack_add = intent::anchor_stack(card.stack_value, intensity, harmony);
            }
            Intent::Release { spend } => {
                stack_spend = spend.min(own);
                let rb = intent::release_bonus(stack_spend, intensity);
                amount += rb;
                power += rb;
                resonance_bonus = rb;
                stack_add = 0;
                if rb != 0 {
                    self.emit(GameEvent::Modifier { seat, source: "release".into(), delta: rb });
                }
            }
            Intent::Sever => {
                amount = (amount - 1).max(0);
                power = (power - 1).max(0);
                sever_amount = intent::sever_cut(card.stack_value, intensity, card.is_disruptive());
                stack_add = intent::sever_stack_add(card.stack_value);
                if enemy > 0 {
                    power += 1;
                }
            }
        }

        if card.has_price(Price::Hungry) && !matches!(p.intent, Intent::Release { .. }) {
            if own > 0 {
                stack_spend += 1;
                amount += 1;
                power += 1;
                self.log(format!("{short} is Hungry: consumes 1 friendly stack for +1 force."));
                self.emit(GameEvent::Modifier { seat, source: "hungry".into(), delta: 1 });
            } else {
                amount = (amount - 1).max(0);
                power = (power - 1).max(0);
                self.log(format!("{short} is Hungry but has no friendly stack to feed on; -1 force."));
                self.emit(GameEvent::Modifier { seat, source: "hungry".into(), delta: -1 });
            }
        }

        PendingPlay {
            seat,
            card: p.card,
            chamber,
            intent: p.intent,
            raw: p.raw,
            intensity,
            power,
            amount,
            stack_add,
            stack_spend,
            sever_amount,
            resonance_bonus,
            half: false,
        }
    }
}
