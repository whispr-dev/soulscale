//! The port is faithful: 300 seeded games played through the ORIGINAL v1.0.1 Python engine
//! (tools/py_harness/run_parity.py) are replayed through this engine with the same seeds,
//! decks, personalities, modes and bot policies, and every log line must match exactly.
//!
//! Regenerate the fixture after changing the Python reference:
//!   python -I tools/py_harness/run_parity.py tools/py_harness/reference/soulscale_purgatory_v101.py \
//!       crates/soulscale-core/tests/fixtures/python_games.jsonl.gz --games 300

use std::io::Read;
use std::sync::Arc;

use serde::Deserialize;
use soulscale_core::bot::play_out;
use soulscale_core::session::{Controller, Session};
use soulscale_core::{Faction, FixedClock, Game, GameConfig, GameMode, NamedDeck, RulesVersion, db};

#[derive(Deserialize)]
struct Cfg {
    seed: u64,
    mode: String,
    p1_faction: String,
    p1_deck: String,
    p2_deck: String,
    ai_profile: String,
    player_profile: String,
    goal: i32,
}

#[derive(Deserialize)]
struct Rec {
    cfg: Cfg,
    rounds: u32,
    soul: i32,
    log: Vec<String>,
}

const STOP_ROUND: u32 = 400;

fn load() -> Vec<Rec> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/python_games.jsonl.gz");
    let f = std::fs::File::open(path).expect("fixture missing: see header comment to regenerate");
    let mut text = String::new();
    flate2::read::GzDecoder::new(f).read_to_string(&mut text).unwrap();
    text.lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

fn run(rec: &Rec) -> (Vec<String>, u32, i32) {
    let d = db();
    let c = &rec.cfg;
    let mode = match c.mode.as_str() {
        "vs_computer" => GameMode::VsComputer,
        "hotseat" => GameMode::Hotseat,
        "secret_hotseat" => GameMode::SecretHotseat,
        other => panic!("unknown mode {other}"),
    };
    let p1 = if c.p1_faction == "heaven" { Faction::Heaven } else { Faction::Hell };
    let deck = |f: Faction, name: &str| NamedDeck { name: name.into(), cards: d.deck(f, name).unwrap().cards.clone() };
    let ai = d.profile(&c.ai_profile).unwrap().clone();
    let player = d.profile(&c.player_profile).unwrap().clone();
    let cfg = GameConfig {
        mode,
        p1_faction: p1,
        p1_deck: deck(p1, &c.p1_deck),
        p2_deck: deck(p1.opponent(), &c.p2_deck),
        goal: c.goal,
        ai_profile: ai.clone(),
        names: GameConfig::default_names(mode),
        rules: RulesVersion::V1_0_1,
    };
    let game = Game::new(d, cfg, c.seed, Arc::new(FixedClock("2026-01-01 00:00:00".into())));
    let controllers = match mode {
        GameMode::VsComputer => [Controller::Human, Controller::Ai],
        _ => [Controller::Human, Controller::Human],
    };
    let mut s = Session::start(game, controllers, c.seed);
    play_out(&mut s, [&player, &ai], STOP_ROUND).unwrap();
    (s.game.log_lines.clone(), s.game.st.round, s.game.st.soul)
}

#[test]
fn rust_engine_reproduces_python_v101_logs_exactly() {
    let recs = load();
    assert_eq!(recs.len(), 300);
    let mut total_lines = 0usize;
    for (i, rec) in recs.iter().enumerate() {
        let (log, rounds, soul) = run(rec);
        let first_diff = log.iter().zip(&rec.log).position(|(a, b)| a != b);
        if let Some(at) = first_diff.or_else(|| (log.len() != rec.log.len()).then(|| log.len().min(rec.log.len()))) {
            let lo = at.saturating_sub(6);
            let ctx = |v: &[String]| v[lo..(at + 3).min(v.len())].join("\n");
            panic!(
                "game #{i} (seed {}, {}) diverges at log line {at}\n--- python ---\n{}\n--- rust ---\n{}",
                rec.cfg.seed,
                rec.cfg.mode,
                ctx(&rec.log),
                ctx(&log)
            );
        }
        assert_eq!((rounds, soul), (rec.rounds, rec.soul), "game #{i} end state");
        total_lines += log.len();
    }
    eprintln!("parity: 300 games, {total_lines} log lines identical");
}
