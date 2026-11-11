//! Endgame summary, round ledger and playtest exports (.txt report + .rounds.csv),
//! matching the v1.0.1 formats.

use serde::{Deserialize, Serialize};

use crate::game::{Game, GameMode, PendingPlay};
use crate::model::{Chamber, Faction};
use crate::state::Seat;

/// One row of the per-round ledger (Python `round_results` / CSV columns).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RoundRow {
    pub round: u32,
    pub player_card: String,
    pub player_chamber: String,
    pub player_intent: String,
    pub player_roll: u8,
    pub player_move: i32,
    pub opponent_card: String,
    pub opponent_chamber: String,
    pub opponent_intent: String,
    pub opponent_roll: u8,
    pub opponent_move: i32,
    pub ongoing_move: i32,
    pub uncapped_net: i32,
    pub final_net: i32,
    pub soul_after: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameStats {
    pub started_at: String,
    pub ended_at: String,
    pub winner: Option<Faction>,
    pub rounds: Vec<RoundRow>,
    /// Plays per Chamber, canonical order.
    pub chamber_uses: [u32; 5],
    /// press, anchor, release, sever.
    pub intent_uses: [u32; 4],
    pub total_heaven_pressure: i32,
    pub total_hell_pressure: i32,
    pub total_net_movement: i32,
    pub biggest_single_abs: i32,
    pub biggest_single_label: String,
    pub biggest_round_abs: i32,
    pub biggest_round_label: String,
}

impl GameStats {
    pub fn new(started_at: String) -> Self {
        GameStats {
            started_at,
            ended_at: String::new(),
            winner: None,
            rounds: Vec::new(),
            chamber_uses: [0; 5],
            intent_uses: [0; 4],
            total_heaven_pressure: 0,
            total_hell_pressure: 0,
            total_net_movement: 0,
            biggest_single_abs: 0,
            biggest_single_label: "No scoring play recorded yet.".into(),
            biggest_round_abs: 0,
            biggest_round_label: "No Soul movement recorded yet.".into(),
        }
    }

    /// Highest count; ties go to the earlier Chamber.
    pub fn most_used_chamber(&self) -> (Chamber, u32) {
        let mut best = (Chamber::Body, self.chamber_uses[0]);
        for c in Chamber::ALL {
            if self.chamber_uses[c.idx()] > best.1 {
                best = (c, self.chamber_uses[c.idx()]);
            }
        }
        best
    }

    pub fn intent_summary_line(&self) -> String {
        let u = self.intent_uses;
        format!("Press {}, Anchor {}, Release {}, Sever {}", u[0], u[1], u[2], u[3])
    }
}

fn intent_index(key: &str) -> usize {
    match key {
        "press" => 0,
        "anchor" => 1,
        "release" => 2,
        _ => 3,
    }
}

fn or_unknown(s: &str) -> &str {
    if s.is_empty() { "unknown" } else { s }
}

impl Game {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_round_stats(
        &mut self,
        p1: &PendingPlay,
        p2: &PendingPlay,
        m1: i32,
        m2: i32,
        ongoing: i32,
        uncapped: i32,
        final_net: i32,
    ) {
        for p in [p1, p2] {
            self.stats.chamber_uses[p.chamber.idx()] += 1;
            self.stats.intent_uses[intent_index(p.intent.key())] += 1;
        }
        for m in [m1, m2, ongoing] {
            if m > 0 {
                self.stats.total_heaven_pressure += m;
            } else if m < 0 {
                self.stats.total_hell_pressure += m.abs();
            }
        }
        self.stats.total_net_movement += final_net;

        let n1 = self.name(Seat::P1).to_string();
        let n2 = self.name(Seat::P2).to_string();
        self.consider_biggest_single(&n1, p1, m1);
        self.consider_biggest_single(&n2, p2, m2);
        if ongoing != 0 && ongoing.abs() > self.stats.biggest_single_abs {
            let dir = if ongoing > 0 { "Heaven" } else { "Hell" };
            self.stats.biggest_single_abs = ongoing.abs();
            self.stats.biggest_single_label =
                format!("Round {}: ongoing board effects scored {ongoing:+} toward {dir}.", self.st.round);
        }
        if final_net.abs() > self.stats.biggest_round_abs {
            let dir = if final_net > 0 { "Heaven" } else { "Hell" };
            let cap_note = if final_net == uncapped { String::new() } else { format!("; capped from {uncapped:+}") };
            self.stats.biggest_round_abs = final_net.abs();
            self.stats.biggest_round_label =
                format!("Round {}: net {final_net:+} toward {dir}{cap_note}.", self.st.round);
        }
        let row = RoundRow {
            round: self.st.round,
            player_card: self.card(p1.card).short(),
            player_chamber: p1.chamber.name().into(),
            player_intent: p1.intent.key().into(),
            player_roll: p1.raw,
            player_move: m1,
            opponent_card: self.card(p2.card).short(),
            opponent_chamber: p2.chamber.name().into(),
            opponent_intent: p2.intent.key().into(),
            opponent_roll: p2.raw,
            opponent_move: m2,
            ongoing_move: ongoing,
            uncapped_net: uncapped,
            final_net,
            soul_after: self.st.soul,
        };
        self.stats.rounds.push(row);
    }

    fn consider_biggest_single(&mut self, actor: &str, p: &PendingPlay, mv: i32) {
        if mv == 0 || mv.abs() <= self.stats.biggest_single_abs {
            return;
        }
        let dir = if mv > 0 { "Heaven" } else { "Hell" };
        self.stats.biggest_single_abs = mv.abs();
        self.stats.biggest_single_label = format!(
            "Round {}: {actor} used {} in {} {} as {}, scoring {mv:+} toward {dir}.",
            self.st.round,
            self.card(p.card).short(),
            p.chamber.emoji(),
            p.chamber.name(),
            p.intent.label()
        );
    }

    fn mode_line(&self) -> String {
        match self.cfg.mode {
            GameMode::VsComputer => format!("Human vs Computer — {}", self.cfg.ai_profile.label),
            GameMode::Hotseat => "Human vs Human / Open hot-seat".into(),
            GameMode::SecretHotseat => "Human vs Human / Secret Hot-seat".into(),
            GameMode::Duel => "Discord Duel".into(),
        }
    }

    fn side_lines(&self) -> (String, String) {
        let s1 = self.st.seat(Seat::P1);
        let s2 = self.st.seat(Seat::P2);
        (
            format!("{}: {} / {}", self.name(Seat::P1), s1.faction.label(), s1.deck_name),
            format!("{}: {} / {}", self.name(Seat::P2), s2.faction.label(), s2.deck_name),
        )
    }

    /// Python `build_endgame_summary`.
    pub fn build_endgame_summary(&self, winner: Faction) -> String {
        let s = &self.stats;
        let (chamber, count) = s.most_used_chamber();
        let (l1, l2) = self.side_lines();
        let bar = "-".repeat(46);
        let mut lines = vec![
            format!("SOULSCALE: PURGATORY v{} — ENDGAME SUMMARY", self.cfg.rules.label()),
            "=".repeat(46),
            format!("Winner: {}", winner.short()),
            format!("Loser: {}", winner.opponent().short()),
            format!("Final Soul: {:+} / ±{}", self.st.soul, self.st.goal),
            format!("Rounds played: {}", self.st.round),
            format!("Started: {}", or_unknown(&s.started_at)),
            format!("Ended: {}", or_unknown(&s.ended_at)),
            format!("Mode: {}", self.mode_line()),
            l1,
            l2,
            String::new(),
            "PRESSURE TOTALS".into(),
            bar.clone(),
            format!("Total Heaven pressure: +{}", s.total_heaven_pressure),
            format!("Total Hell pressure: -{}", s.total_hell_pressure),
            format!("Raw pressure difference: {:+}", s.total_heaven_pressure - s.total_hell_pressure),
            format!("Actual capped net Soul movement: {:+}", s.total_net_movement),
            String::new(),
            "PLAY PATTERNS".into(),
            bar.clone(),
            format!("Most used Chamber: {} {} ({} plays)", chamber.emoji(), chamber.name(), count),
            format!("Intent use: {}", s.intent_summary_line()),
            format!("Biggest single swing: {}", s.biggest_single_label),
            format!("Biggest net round: {}", s.biggest_round_label),
            String::new(),
            "Last 5 rounds".into(),
            bar,
        ];
        let start = s.rounds.len().saturating_sub(5);
        if s.rounds.is_empty() {
            lines.push("No resolved rounds recorded.".into());
        }
        for r in &s.rounds[start..] {
            lines.push(format!(
                "R{}: {} {:+}; {} {:+}; ongoing {:+}; net {:+}; Soul {:+}",
                r.round, r.player_card, r.player_move, r.opponent_card, r.opponent_move, r.ongoing_move, r.final_net, r.soul_after
            ));
        }
        lines.join("\n")
    }

    /// Python `build_interim_summary` (mid-game export).
    pub fn build_interim_summary(&self, exported_at: &str) -> String {
        let s = &self.stats;
        let (chamber, count) = s.most_used_chamber();
        let (l1, l2) = self.side_lines();
        let bar = "-".repeat(52);
        let pull = if self.st.soul > 0 {
            "Heaven"
        } else if self.st.soul < 0 {
            "Hell"
        } else {
            "Balanced"
        };
        [
            "SOULSCALE: PURGATORY — INTERIM PLAYTEST REPORT".to_string(),
            "=".repeat(52),
            "Status: in progress".into(),
            format!("Current Soul: {:+} / ±{}", self.st.soul, self.st.goal),
            format!("Current pull: {pull}"),
            format!("Round counter: {}", self.st.round),
            format!("Started: {}", or_unknown(&s.started_at)),
            format!("Exported: {exported_at}"),
            format!("Mode: {}", self.mode_line()),
            l1,
            l2,
            String::new(),
            "PRESSURE TOTALS SO FAR".into(),
            bar.clone(),
            format!("Total Heaven pressure: +{}", s.total_heaven_pressure),
            format!("Total Hell pressure: -{}", s.total_hell_pressure),
            format!("Raw pressure difference: {:+}", s.total_heaven_pressure - s.total_hell_pressure),
            format!("Actual capped net Soul movement: {:+}", s.total_net_movement),
            String::new(),
            "PLAY PATTERNS SO FAR".into(),
            bar,
            format!("Most used Chamber: {} {} ({} plays)", chamber.emoji(), chamber.name(), count),
            format!("Intent use: {}", s.intent_summary_line()),
            format!("Biggest single swing: {}", s.biggest_single_label),
            format!("Biggest net round: {}", s.biggest_round_label),
        ]
        .join("\n")
    }

    /// Python `build_playtest_export_text`: summary + full ledger + raw log.
    pub fn playtest_report(&self, exported_at: &str) -> String {
        let summary = match self.st.winner {
            Some(w) if self.st.game_over => self.build_endgame_summary(w),
            _ => self.build_interim_summary(exported_at),
        };
        let (n1, n2) = (self.name(Seat::P1), self.name(Seat::P2));
        let mut lines = vec![summary, String::new(), "FULL ROUND LEDGER".into(), "=".repeat(52)];
        if self.stats.rounds.is_empty() {
            lines.push("No resolved rounds recorded.".into());
        }
        for r in &self.stats.rounds {
            lines.push(format!(
                "R{:>2}: {n1} {} → {} /{} roll {} = {:+}; {n2} {} → {} /{} roll {} = {:+}; ongoing {:+}; uncapped {:+}; final {:+}; Soul {:+}",
                r.round,
                r.player_card,
                r.player_chamber,
                r.player_intent,
                r.player_roll,
                r.player_move,
                r.opponent_card,
                r.opponent_chamber,
                r.opponent_intent,
                r.opponent_roll,
                r.opponent_move,
                r.ongoing_move,
                r.uncapped_net,
                r.final_net,
                r.soul_after
            ));
        }
        let log = self.log_lines.join("\n");
        let log = log.trim_end();
        lines.extend([
            String::new(),
            "RAW GAME LOG".into(),
            "=".repeat(52),
            if log.is_empty() { "No raw log lines recorded.".into() } else { log.to_string() },
            String::new(),
        ]);
        lines.join("\n")
    }

    /// Python `write_round_csv` (UTF-8 with BOM, CRLF line endings like Python's csv module).
    pub fn rounds_csv(&self) -> String {
        let mut out = String::from("\u{feff}");
        out.push_str("round,player_card,player_chamber,player_intent,player_roll,player_move,opponent_card,opponent_chamber,opponent_intent,opponent_roll,opponent_move,ongoing_move,uncapped_net,final_net,soul_after\r\n");
        for r in &self.stats.rounds {
            let cells = [
                r.round.to_string(),
                csv_cell(&r.player_card),
                r.player_chamber.clone(),
                r.player_intent.clone(),
                r.player_roll.to_string(),
                r.player_move.to_string(),
                csv_cell(&r.opponent_card),
                r.opponent_chamber.clone(),
                r.opponent_intent.clone(),
                r.opponent_roll.to_string(),
                r.opponent_move.to_string(),
                r.ongoing_move.to_string(),
                r.uncapped_net.to_string(),
                r.final_net.to_string(),
                r.soul_after.to_string(),
            ];
            out.push_str(&cells.join(","));
            out.push_str("\r\n");
        }
        out
    }

    /// Python `playtest_export_basename` with a caller-supplied `%Y%m%d_%H%M%S` stamp.
    pub fn export_basename(&self, stamp: &str) -> String {
        let mode = match self.cfg.mode {
            GameMode::VsComputer => "ai",
            GameMode::Duel => "duel",
            _ => "hotseat",
        };
        let winner = self.st.winner.map(|w| w.short().to_lowercase()).unwrap_or_else(|| "in_progress".into());
        format!(
            "soulscale_v{}_{stamp}_{mode}_{winner}_r{}_{}_vs_{}",
            self.cfg.rules.compact(),
            self.st.round,
            safe_slug(&self.st.seats[0].deck_name),
            safe_slug(&self.st.seats[1].deck_name)
        )
    }
}

fn csv_cell(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

/// Python `safe_slug`.
pub fn safe_slug(value: &str) -> String {
    let mut out = String::new();
    for ch in value.to_lowercase().chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
        } else if matches!(ch, ' ' | '-' | '_' | '.') {
            out.push('_');
        }
    }
    let mut slug = out.trim_matches('_').to_string();
    while slug.contains("__") {
        slug = slug.replace("__", "_");
    }
    if slug.is_empty() {
        slug = "deck".into();
    }
    slug.chars().take(42).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_matches_python() {
        assert_eq!(safe_slug("Custom: wofls hell deck v101"), "custom_wofls_hell_deck_v101");
        assert_eq!(safe_slug("Starter Mercy"), "starter_mercy");
        assert_eq!(safe_slug("***"), "deck");
    }
}
