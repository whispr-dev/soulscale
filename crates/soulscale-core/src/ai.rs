//! Heuristic AI (Python `choose_ai_play`, `score_ai_candidate`, `ai_should_reroll`).
//!
//! Floating-point expressions are evaluated in the same order as the Python so that the
//! noise term and the weighted pick land on identical choices for identical seeds.

use crate::game::{Choice, Game};
use crate::model::{AiProfile, CardDef, CardId, Chamber, Faction, Intent, Price, Special};
use crate::rng::py_sum_floats;
use crate::rules::intent::{press_bonus, release_bonus};
use crate::rules::movement::{movement_sign, signed_card_movement};
use crate::state::Seat;

/// What the AI knows about the other side's play (always `None` in v1.0.1: it commits blind).
#[derive(Clone, Copy, Debug)]
pub struct OtherPlay {
    pub chamber: Chamber,
    pub intent: Option<Intent>,
}

impl Game {
    /// An AI urgency threshold. v1.0.1 used absolute Soul values tuned when the goal was
    /// fixed at ±12 (v0.1–v0.4) and never rescaled; v1.0.2 keeps the same *distance from
    /// the goal* at every game length. At ±12 both give the same number.
    pub fn urgency_threshold(&self, v101_value: i32) -> i32 {
        if self.cfg.rules.fixes() { self.st.goal - (12 - v101_value) } else { v101_value }
    }

    /// Python `ai_modes_for`.
    pub fn ai_modes_for(&self, card: &CardDef, chamber: Chamber) -> Vec<Intent> {
        let own = self.st.stack_total(card.faction, chamber);
        let mut modes = vec![Intent::Press, Intent::Anchor, Intent::Sever];
        if own > 0 {
            let mut spends = vec![1, own.min(3), own.min(6), own];
            spends.sort_unstable();
            spends.dedup();
            modes.extend(spends.into_iter().map(|spend| Intent::Release { spend }));
        }
        modes
    }

    /// Python `can_threaten_win`.
    pub fn can_threaten_win(&self, faction: Faction, amount: i32, card: &CardDef) -> bool {
        let projected = self.st.soul + signed_card_movement(card, amount);
        match faction {
            Faction::Heaven => projected >= self.st.goal,
            Faction::Hell => projected <= -self.st.goal,
        }
    }

    /// Python `score_ai_candidate`. Consumes one `uniform` draw.
    pub fn score_ai_candidate(
        &mut self,
        profile: &AiProfile,
        card: &CardDef,
        chamber: Chamber,
        mode: Intent,
        other: Option<OtherPlay>,
    ) -> f64 {
        let soul = self.st.soul;
        let f = card.faction;
        let mut score = 0.0f64;
        let own = self.st.stack_total(f, chamber);
        let enemy = self.st.stack_total(f.opponent(), chamber);
        let harmony = self.st.harmony_count(f, chamber, &card.aspect);
        let expected_intensity = 2;
        let mut ea = if card.base_move != 0 { card.base_move + expected_intensity } else { 0 };

        if card.special == Special::BodyBonus && chamber == Chamber::Body {
            ea += 1;
        }
        if matches!(card.special, Special::BehindBonus | Special::Screech) {
            let boosted = if card.special == Special::Screech && self.cfg.rules.fixes() {
                soul < 0
            } else {
                (f == Faction::Heaven && soul < 0)
                    || (f == Faction::Hell && soul > 0)
                    || (card.special == Special::Screech && soul < 0)
            };
            if boosted {
                ea += 1;
            }
        }

        if card.has_price(Price::Heavy) {
            let available = self.st.will_of_faction(f);
            score -= if available > 0 { 0.8 } else { 2.2 };
            if available <= 0 {
                ea = (ea - 1).max(0);
            }
        }
        if card.has_price(Price::Desperate) {
            let behind = (f == Faction::Heaven && soul < 0) || (f == Faction::Hell && soul > 0);
            let ahead = (f == Faction::Heaven && soul > 0) || (f == Faction::Hell && soul < 0);
            if behind {
                ea += 1;
                score += 1.0;
            } else if ahead {
                ea = (ea - 1).max(0);
                score -= 0.8;
            }
        }
        if card.has_price(Price::Volatile) {
            score += if profile.randomness >= 1.0 { 0.35 } else { -0.35 };
        }
        if card.has_price(Price::Delayed) {
            score += profile.planning * 0.6;
            if soul.abs() >= self.st.goal - 4 {
                score -= 1.5;
            }
        }
        if card.has_price(Price::Binding) {
            score -= 0.7;
        }
        if card.has_price(Price::Echo) {
            score += profile.planning * 0.35;
        }
        if card.has_price(Price::Hungry) && own <= 0 {
            ea = (ea - 1).max(0);
            score -= 1.2;
        }

        match mode {
            Intent::Press => {
                ea += press_bonus(own, harmony, enemy);
                score += 0.8;
            }
            Intent::Anchor => {
                ea = 0;
                score += profile.planning * (1.2 + card.stack_value as f64 + 0.4 * harmony as f64);
                if soul.abs() >= self.urgency_threshold(8) {
                    score -= 2.0;
                }
            }
            Intent::Release { spend } => {
                let rb = release_bonus(spend, expected_intensity);
                ea += rb;
                let threat = if self.can_threaten_win(f, ea, card) { 1.5 } else { 0.0 };
                score += profile.release_hunger * (rb as f64 + threat);
                if spend >= own && own >= 5 && soul.abs() < self.urgency_threshold(7) {
                    score -= 1.2;
                }
            }
            Intent::Sever => {
                ea = (ea - 1).max(0);
                let cut = card.stack_value + expected_intensity + i32::from(card.is_disruptive());
                score += profile.disruption * enemy.min(cut) as f64;
                if enemy <= 0 {
                    score -= 2.2;
                }
            }
        }

        let sign = movement_sign(card, None);
        let ownerward = if sign == f.sign() {
            ea
        } else if sign == -f.sign() {
            -ea
        } else {
            0
        };
        score += profile.aggression * ownerward as f64;

        if card.persists() {
            score += profile.planning * 1.4;
        }
        if matches!(card.special, Special::Serpent | Special::Miracle) {
            score += 1.0;
        }
        if card.is_disruptive() {
            score += profile.disruption * 0.8;
        }

        if let Some(o) = other
            && o.chamber == chamber
        {
            score += 1.0;
            if matches!(mode, Intent::Sever) && matches!(o.intent, Some(Intent::Anchor | Intent::Press)) {
                score += profile.disruption * 1.6;
            }
            if matches!(card.special, Special::FalseDoor | Special::Tangle | Special::Calm) {
                score += profile.disruption * 2.0;
            }
        }

        if self.st.board[chamber.idx()].iter().any(|e| e.faction == f.opponent()) {
            if matches!(card.special, Special::Calm | Special::Bell | Special::Rest | Special::LovedMemory) {
                score += profile.disruption * 2.5;
            }
            if card.trap || card.guard {
                score += profile.planning;
            }
        }

        // Win/defense urgency. The Python set {"void_thought", "serpent", "false_door"} names
        // a special no card has (Void Thought's special is behind_bonus), so in v1.0.1 Void
        // Thought never qualifies; v1.0.2 includes it as intended.
        let close_out = self.urgency_threshold(8);
        let danger = self.urgency_threshold(6);
        match f {
            Faction::Hell => {
                if soul <= -close_out && ownerward > 0 {
                    score += 2.0;
                }
                let defender = matches!(card.special, Special::Serpent | Special::FalseDoor)
                    || (self.cfg.rules.fixes() && card.key == "void_thought");
                if soul >= danger && defender {
                    score += 1.5;
                }
            }
            Faction::Heaven => {
                if soul >= close_out && ownerward > 0 {
                    score += 2.0;
                }
                if soul <= -danger
                    && matches!(card.special, Special::BehindBonus | Special::Miracle | Special::Bell | Special::Calm)
                {
                    score += 1.5;
                }
            }
        }

        // Thematic chamber nudges.
        let k = card.key.as_str();
        if matches!(k, "ache" | "candle" | "rest" | "cold_room") && chamber == Chamber::Body {
            score += 0.5;
        }
        if matches!(k, "screech" | "calm_breath" | "hymn" | "void_thought") && chamber == Chamber::Mind {
            score += 0.5;
        }
        if matches!(k, "loved_memory" | "memory_thorn" | "watching_eye") && chamber == Chamber::Memory {
            score += 0.5;
        }
        if matches!(k, "hope" | "endless_wait" | "burden" | "dawn" | "futility" | "purpose" | "weight")
            && chamber == Chamber::Time
        {
            score += 0.5;
        }
        if matches!(k, "shame_mirror" | "obsession" | "acceptance" | "grace" | "silver_thread")
            && chamber == Chamber::Memory
        {
            score += 0.5;
        }

        score += self.rng.uniform(-profile.randomness, profile.randomness);
        score
    }

    /// Python `choose_ai_play` for any seat (v1.0.1 only ever passes `other = None`).
    pub fn ai_choose(&mut self, seat: Seat, profile: &AiProfile, other: Option<OtherPlay>) -> Choice {
        let hand: Vec<CardId> = self.st.seat(seat).deck.revealed.clone();
        let mut candidates: Vec<(f64, Choice)> = Vec::new();
        for &id in &hand {
            let card: &'static CardDef = self.card(id);
            for &chamber in &card.valid_chambers {
                if self.st.is_chamber_locked(card.faction, chamber) {
                    continue;
                }
                for mode in self.ai_modes_for(card, chamber) {
                    let s = self.score_ai_candidate(profile, card, chamber, mode, other);
                    candidates.push((s, Choice { card: id, chamber, intent: mode }));
                }
            }
        }
        if candidates.is_empty() {
            let c = hand[0];
            return Choice { card: c, chamber: self.card(c).valid_chambers[0], intent: Intent::Press };
        }
        // Stable sort, descending (Python `sort(key=..., reverse=True)` keeps tie order).
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        if profile.randomness <= 0.35 {
            return candidates[0].1;
        }
        let top_count = if profile.randomness < 1.0 {
            2
        } else if profile.randomness < 2.0 {
            4
        } else {
            7
        };
        let top = &candidates[..top_count.min(candidates.len())];
        let weights: Vec<f64> = top.iter().map(|(s, _)| (s + 8.0).max(0.1)).collect();
        let total = py_sum_floats(&weights);
        let r = self.rng.uniform(0.0, total);
        let mut upto = 0.0;
        for (item, w) in top.iter().zip(&weights) {
            upto += w;
            if upto >= r {
                return item.1;
            }
        }
        top[top.len() - 1].1
    }

    /// Python `ai_should_reroll`. Consumes one `random()` only when a reroll is considered.
    pub fn ai_should_reroll(
        &mut self,
        profile: &AiProfile,
        choice: &Choice,
        intensity: i32,
        other_chamber: Option<Chamber>,
    ) -> bool {
        if intensity >= 3 {
            return false;
        }
        let is_release = matches!(choice.intent, Intent::Release { .. });
        if intensity == 2 && !is_release {
            return false;
        }
        let card = self.card(choice.card);
        let mut desire = profile.reroll_bias;
        if other_chamber == Some(choice.chamber) {
            desire += 0.20;
        }
        if is_release {
            desire += 0.30;
        }
        if card.base_move > 0 && self.st.soul.abs() >= self.urgency_threshold(7) {
            desire += 0.25;
        }
        if matches!(card.special, Special::Serpent | Special::Miracle) {
            desire += 0.15;
        }
        self.rng.random() < desire.min(0.95)
    }
}
