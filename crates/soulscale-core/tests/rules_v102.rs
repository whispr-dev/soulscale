//! v1.0.2 fixes: what changed, and proof that nothing else did.

use std::sync::Arc;

use soulscale_core::bot::play_out;
use soulscale_core::game::PlayCommit;
use soulscale_core::session::{Controller, Session};
use soulscale_core::{Chamber, Faction, FixedClock, Game, GameConfig, GameMode, Intent, NamedDeck, RulesVersion, db};

fn game(rules: RulesVersion, goal: i32, hell_cards: Option<Vec<&str>>, seed: u64) -> Game {
    let d = db();
    let heaven = d.deck(Faction::Heaven, "Starter Mercy").unwrap();
    let hell = match hell_cards {
        Some(keys) => NamedDeck { name: "Test Hell".into(), cards: keys.iter().map(|k| d.id(k).unwrap()).collect() },
        None => {
            let h = d.deck(Faction::Hell, "Starter Torment").unwrap();
            NamedDeck { name: h.name.clone(), cards: h.cards.clone() }
        }
    };
    let cfg = GameConfig {
        mode: GameMode::VsComputer,
        p1_faction: Faction::Heaven,
        p1_deck: NamedDeck { name: heaven.name.clone(), cards: heaven.cards.clone() },
        p2_deck: hell,
        goal,
        ai_profile: d.profile("Balanced / Tactical").unwrap().clone(),
        names: GameConfig::default_names(GameMode::VsComputer),
        rules,
    };
    Game::new(d, cfg, seed, Arc::new(FixedClock("2026-01-01 00:00:00".into())))
}

fn screech_vs_rest(rules: RulesVersion, soul: i32) -> String {
    let d = db();
    let mut g = game(rules, 18, None, 3);
    g.st.soul = soul;
    g.resolve(
        PlayCommit { card: d.id("rest").unwrap(), chamber: Chamber::Time, intent: Intent::Press, raw: 3 },
        PlayCommit { card: d.id("screech").unwrap(), chamber: Chamber::Mind, intent: Intent::Press, raw: 3 },
    );
    g.log_lines.join("\n")
}

#[test]
fn screech_bonus_only_below_zero_in_v102() {
    // Screech base move 1 + intensity 2 = 3; +1 when boosted.
    assert!(screech_vs_rest(RulesVersion::V1_0_1, 5).contains("🔊 Screech contributes -4 toward Hell."));
    assert!(screech_vs_rest(RulesVersion::V1_0_2, 5).contains("🔊 Screech contributes -3 toward Hell."));
    assert!(screech_vs_rest(RulesVersion::V1_0_1, -5).contains("🔊 Screech contributes -4 toward Hell."));
    assert!(screech_vs_rest(RulesVersion::V1_0_2, -5).contains("🔊 Screech contributes -4 toward Hell."));
    assert!(screech_vs_rest(RulesVersion::V1_0_2, 0).contains("🔊 Screech contributes -3 toward Hell."));
}

#[test]
fn silver_thread_names_itself_in_v102() {
    let d = db();
    for (rules, want) in [
        (RulesVersion::V1_0_1, "Small Comfort gives the next Heaven effect +1."),
        (RulesVersion::V1_0_2, "Silver Thread gives the next Heaven effect +1."),
    ] {
        let mut g = game(rules, 18, None, 4);
        g.resolve(
            PlayCommit { card: d.id("silver_thread").unwrap(), chamber: Chamber::Heart, intent: Intent::Press, raw: 3 },
            PlayCommit { card: d.id("ache").unwrap(), chamber: Chamber::Body, intent: Intent::Press, raw: 3 },
        );
        assert!(g.log_lines.iter().any(|l| l == want), "{rules:?}");
    }
}

#[test]
fn urgency_thresholds_keep_distance_from_goal() {
    for (goal, want) in [(12, [8, 7, 6]), (18, [14, 13, 12]), (24, [20, 19, 18])] {
        let g = game(RulesVersion::V1_0_2, goal, None, 1);
        assert_eq!([8, 7, 6].map(|v| g.urgency_threshold(v)), want, "goal {goal}");
        let old = game(RulesVersion::V1_0_1, goal, None, 1);
        assert_eq!([8, 7, 6].map(|v| old.urgency_threshold(v)), [8, 7, 6]);
    }
}

#[test]
fn anchor_penalty_now_scales_with_goal() {
    let d = db();
    let mut profile = d.profile("Architect / Stack Builder").unwrap().clone();
    profile.randomness = 0.0; // uniform(-0, 0) == 0: scores are exact
    let candle = d.card(d.id("candle").unwrap()).clone();
    let score = |rules| {
        let mut g = game(rules, 18, None, 9);
        g.st.soul = 9; // past the old absolute 8, far from the ±18 goal
        g.score_ai_candidate(&profile, &candle, Chamber::Body, Intent::Anchor, None)
    };
    let (v101, v102) = (score(RulesVersion::V1_0_1), score(RulesVersion::V1_0_2));
    assert_eq!(v102 - v101, 2.0, "v1.0.1 wrongly applies the near-goal Anchor penalty at Soul 9 of 18");
}

#[test]
fn at_goal_12_without_changed_cards_v102_plays_exactly_like_v101() {
    // A Hell deck with no Screech or Void Thought, against a Heaven deck with no Silver Thread.
    let hell = vec![
        "ache", "ache", "fly_swarm", "burden", "cold_room", "weight", "crawling_skin", "futility", "memory_thorn", "snare",
    ];
    let d = db();
    let player = d.profile("Chaos Imp / Weird").unwrap().clone();
    let ai = d.profile("Balanced / Tactical").unwrap().clone();
    for seed in 0..60u64 {
        let run = |rules| {
            let g = game(rules, 12, Some(hell.clone()), seed * 101 + 7);
            let mut s = Session::start(g, [Controller::Human, Controller::Ai], seed);
            play_out(&mut s, [&player, &ai], 400).unwrap();
            s.game
                .log_lines
                .iter()
                .filter(|l| !l.starts_with("Welcome to Soulscale") && !l.starts_with("SOULSCALE: PURGATORY v"))
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(run(RulesVersion::V1_0_1), run(RulesVersion::V1_0_2), "seed {seed}");
    }
}
