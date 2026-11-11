//! soulscale-sim: headless balance testing and replay verification.
//!
//! ```text
//! soulscale-sim matrix  --games 200 --goal 18                # every Heaven deck vs every Hell deck
//! soulscale-sim compare --games 200 --goal 18                # v1.0.1 vs v1.0.2 side by side
//! soulscale-sim game    --seed 42 --heaven "Starter Mercy" --hell "Pain Engine"   # print one full log
//! soulscale-sim replay  data/replays/<file>.replay.json     # re-run a saved match, check it matches
//! ```
//!
//! Both seats are driven by the heuristic AI (P1 through the same bot the parity tests
//! use, P2 as the in-game Computer), so results reflect the decks and rules, not a human.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use soulscale_core::bot::play_out;
use soulscale_core::session::{Controller, Input, Session};
use soulscale_core::{AiProfile, CardDb, Faction, FixedClock, Game, GameConfig, GameMode, NamedDeck, RulesVersion, db};

#[derive(Parser)]
#[command(name = "soulscale-sim", version, about = "Soulscale headless simulator")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Args, Clone)]
struct SimOpts {
    /// Games per matchup (half with Heaven as P1, half with Hell as P1).
    #[arg(long, default_value_t = 200)]
    games: u32,
    /// Goal: 12, 18 or 24.
    #[arg(long, default_value_t = 18)]
    goal: i32,
    /// Rules revision: 1.0.1 or 1.0.2.
    #[arg(long, default_value = "1.0.2")]
    rules: String,
    /// AI personality for both sides (see `soulscale-sim profiles`).
    #[arg(long, default_value = "Balanced / Tactical")]
    profile: String,
    /// Base seed; every game's seed derives from it, so runs are reproducible.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Rounds before a game is called a stalemate.
    #[arg(long, default_value_t = 400)]
    max_rounds: u32,
    /// Worker threads (default: all cores).
    #[arg(long)]
    threads: Option<usize>,
    /// Also write per-matchup results as CSV.
    #[arg(long)]
    csv: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Win rates for every built-in Heaven deck against every built-in Hell deck.
    Matrix(SimOpts),
    /// Run the matrix under v1.0.1 and v1.0.2 and print the difference.
    Compare(SimOpts),
    /// Play and print one game.
    Game {
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long, default_value = "Starter Mercy")]
        heaven: String,
        #[arg(long, default_value = "Starter Torment")]
        hell: String,
        #[arg(long, default_value_t = 18)]
        goal: i32,
        #[arg(long, default_value = "1.0.2")]
        rules: String,
        #[arg(long, default_value = "Balanced / Tactical")]
        profile: String,
    },
    /// Re-run a saved match from its replay file and confirm the log matches.
    Replay { file: PathBuf },
    /// List AI personalities and decks.
    Profiles,
}

fn rules(s: &str) -> anyhow::Result<RulesVersion> {
    match s {
        "1.0.1" | "101" => Ok(RulesVersion::V1_0_1),
        "1.0.2" | "102" => Ok(RulesVersion::V1_0_2),
        _ => bail!("--rules must be 1.0.1 or 1.0.2"),
    }
}

fn profile(d: &CardDb, label: &str) -> anyhow::Result<AiProfile> {
    d.profile(label).cloned().with_context(|| format!("unknown profile {label:?}; try `soulscale-sim profiles`"))
}

fn deck(d: &CardDb, f: Faction, name: &str) -> anyhow::Result<NamedDeck> {
    let dl = d.deck(f, name).with_context(|| format!("no {} deck {name:?}", f.short()))?;
    Ok(NamedDeck { name: dl.name.clone(), cards: dl.cards.clone() })
}

struct Outcome {
    winner: Option<Faction>,
    rounds: u32,
    played: Vec<(Faction, String)>,
}

#[allow(clippy::too_many_arguments)]
fn play_one(
    heaven: &NamedDeck,
    hell: &NamedDeck,
    heaven_first: bool,
    goal: i32,
    rules: RulesVersion,
    prof: &AiProfile,
    seed: u64,
    max_rounds: u32,
) -> Outcome {
    let (p1_faction, p1, p2) =
        if heaven_first { (Faction::Heaven, heaven, hell) } else { (Faction::Hell, hell, heaven) };
    let cfg = GameConfig {
        mode: GameMode::VsComputer,
        p1_faction,
        p1_deck: p1.clone(),
        p2_deck: p2.clone(),
        goal,
        ai_profile: prof.clone(),
        names: GameConfig::default_names(GameMode::VsComputer),
        rules,
    };
    let g = Game::new(db(), cfg, seed, Arc::new(FixedClock(String::new())));
    let mut s = Session::start(g, [Controller::Human, Controller::Ai], seed);
    play_out(&mut s, [prof, prof], max_rounds).expect("bots only make legal plays");
    let played = s
        .game
        .stats
        .rounds
        .iter()
        .flat_map(|r| [(p1_faction, r.player_card.clone()), (p1_faction.opponent(), r.opponent_card.clone())])
        .collect();
    Outcome { winner: s.game.st.winner, rounds: s.game.st.round, played }
}

#[derive(Default, Clone)]
struct Cell {
    games: u32,
    heaven: u32,
    hell: u32,
    stalemates: u32,
    rounds: u64,
}

impl Cell {
    fn heaven_pct(&self) -> f64 {
        let decided = self.heaven + self.hell;
        if decided == 0 { 0.0 } else { 100.0 * self.heaven as f64 / decided as f64 }
    }
    fn avg_rounds(&self) -> f64 {
        if self.games == 0 { 0.0 } else { self.rounds as f64 / self.games as f64 }
    }
}

struct MatrixResult {
    heaven_decks: Vec<String>,
    hell_decks: Vec<String>,
    cells: Vec<Vec<Cell>>,
    /// card -> (plays, plays by the eventual winner)
    cards: BTreeMap<String, (u64, u64)>,
}

fn run_matrix(o: &SimOpts, rv: RulesVersion) -> anyhow::Result<MatrixResult> {
    let d = db();
    if ![12, 18, 24].contains(&o.goal) {
        bail!("--goal must be 12, 18 or 24");
    }
    let prof = profile(d, &o.profile)?;
    let hv: Vec<NamedDeck> = d.decks(Faction::Heaven).map(|x| NamedDeck { name: x.name.clone(), cards: x.cards.clone() }).collect();
    let hl: Vec<NamedDeck> = d.decks(Faction::Hell).map(|x| NamedDeck { name: x.name.clone(), cards: x.cards.clone() }).collect();
    let mut jobs = Vec::new();
    for i in 0..hv.len() {
        for j in 0..hl.len() {
            for k in 0..o.games {
                jobs.push((i, j, k));
            }
        }
    }
    let threads = o.threads.unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2)).max(1);
    let chunk = jobs.len().div_ceil(threads);
    let results: Vec<(usize, usize, Outcome)> = std::thread::scope(|sc| {
        let handles: Vec<_> = jobs
            .chunks(chunk.max(1))
            .map(|part| {
                let (hv, hl, prof) = (&hv, &hl, &prof);
                sc.spawn(move || {
                    part.iter()
                        .map(|&(i, j, k)| {
                            let seed = o.seed.wrapping_mul(1_000_003).wrapping_add((i * 1000 + j) as u64 * 100_000 + k as u64);
                            (i, j, play_one(&hv[i], &hl[j], k % 2 == 0, o.goal, rv, prof, seed, o.max_rounds))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().expect("worker panicked")).collect()
    });
    let mut cells = vec![vec![Cell::default(); hl.len()]; hv.len()];
    let mut cards: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for (i, j, out) in results {
        let c = &mut cells[i][j];
        c.games += 1;
        c.rounds += out.rounds as u64;
        match out.winner {
            Some(Faction::Heaven) => c.heaven += 1,
            Some(Faction::Hell) => c.hell += 1,
            None => c.stalemates += 1,
        }
        for (f, card) in out.played {
            let e = cards.entry(card).or_default();
            e.0 += 1;
            if out.winner == Some(f) {
                e.1 += 1;
            }
        }
    }
    Ok(MatrixResult {
        heaven_decks: hv.into_iter().map(|d| d.name).collect(),
        hell_decks: hl.into_iter().map(|d| d.name).collect(),
        cells,
        cards,
    })
}

fn print_matrix(m: &MatrixResult, title: &str) {
    println!("\n{title}");
    println!("Heaven win % (decided games), rows = Heaven deck, columns = Hell deck\n");
    print!("{:<20}", "");
    for h in &m.hell_decks {
        print!("{:>17}", trunc(h, 16));
    }
    println!("{:>10}", "row avg");
    let mut total = Cell::default();
    for (i, row) in m.cells.iter().enumerate() {
        print!("{:<20}", trunc(&m.heaven_decks[i], 19));
        let mut acc = Cell::default();
        for c in row {
            print!("{:>16.1}%", c.heaven_pct());
            acc.games += c.games;
            acc.heaven += c.heaven;
            acc.hell += c.hell;
            acc.stalemates += c.stalemates;
            acc.rounds += c.rounds;
        }
        println!("{:>9.1}%", acc.heaven_pct());
        total.games += acc.games;
        total.heaven += acc.heaven;
        total.hell += acc.hell;
        total.stalemates += acc.stalemates;
        total.rounds += acc.rounds;
    }
    println!(
        "\nOverall: {} games, Heaven {:.1}%, avg {:.1} rounds, {} stalemates",
        total.games,
        total.heaven_pct(),
        total.avg_rounds(),
        total.stalemates
    );
}

fn print_cards(m: &MatrixResult) {
    println!("\nCard win-rate when played (plays ≥ 50):");
    let mut v: Vec<_> = m.cards.iter().filter(|(_, (n, _))| *n >= 50).collect();
    v.sort_by(|a, b| {
        let ra = a.1.1 as f64 / a.1.0 as f64;
        let rb = b.1.1 as f64 / b.1.0 as f64;
        rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
    });
    for (card, (n, w)) in v {
        println!("  {:<26} {:>7} plays  {:>5.1}% won", card, n, 100.0 * *w as f64 / *n as f64);
    }
}

fn write_csv(m: &MatrixResult, path: &PathBuf) -> anyhow::Result<()> {
    let mut s = String::from("heaven_deck,hell_deck,games,heaven_wins,hell_wins,stalemates,heaven_pct,avg_rounds\n");
    for (i, row) in m.cells.iter().enumerate() {
        for (j, c) in row.iter().enumerate() {
            s.push_str(&format!(
                "{},{},{},{},{},{},{:.2},{:.2}\n",
                m.heaven_decks[i],
                m.hell_decks[j],
                c.games,
                c.heaven,
                c.hell,
                c.stalemates,
                c.heaven_pct(),
                c.avg_rounds()
            ));
        }
    }
    std::fs::write(path, s).with_context(|| format!("writing {}", path.display()))
}

fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { s.chars().take(n - 1).collect::<String>() + "…" }
}

#[derive(Deserialize)]
struct ReplayDeck {
    name: String,
    cards: Vec<String>,
}

#[derive(Deserialize)]
struct ReplayFile {
    rules_version: RulesVersion,
    seed: u64,
    mode: GameMode,
    p1_faction: Faction,
    p1_deck: ReplayDeck,
    p2_deck: ReplayDeck,
    goal: i32,
    ai_profile: String,
    names: [String; 2],
    inputs: Vec<Input>,
    log: Vec<String>,
}

fn replay(path: &PathBuf) -> anyhow::Result<()> {
    let d = db();
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let r: ReplayFile = serde_json::from_str(&text).context("not a Soulscale replay file")?;
    let to_deck = |rd: &ReplayDeck| -> anyhow::Result<NamedDeck> {
        let cards = rd.cards.iter().map(|k| d.id(k).with_context(|| format!("unknown card {k}"))).collect::<anyhow::Result<_>>()?;
        Ok(NamedDeck { name: rd.name.clone(), cards })
    };
    let cfg = GameConfig {
        mode: r.mode,
        p1_faction: r.p1_faction,
        p1_deck: to_deck(&r.p1_deck)?,
        p2_deck: to_deck(&r.p2_deck)?,
        goal: r.goal,
        ai_profile: profile(d, &r.ai_profile)?,
        names: r.names.clone(),
        rules: r.rules_version,
    };
    // Timestamps live in the summary; compare everything else.
    let g = Game::new(d, cfg, r.seed, Arc::new(FixedClock(String::new())));
    let controllers = if r.mode == GameMode::VsComputer { [Controller::Human, Controller::Ai] } else { [Controller::Human; 2] };
    let mut s = Session::start(g, controllers, r.seed);
    for (i, input) in r.inputs.iter().enumerate() {
        s.apply_input(input).with_context(|| format!("input #{i} rejected: {input:?}"))?;
    }
    let skip = |l: &&String| !l.starts_with("Started:") && !l.starts_with("Ended:") && !l.ends_with(" resigned.");
    let got: Vec<&String> = s.game.log_lines.iter().filter(skip).collect();
    let want: Vec<&String> = r.log.iter().filter(skip).collect();
    if let Some(i) = got.iter().zip(&want).position(|(a, b)| a != b) {
        bail!("replay diverges at log line {i}:\n  saved:  {}\n  replay: {}", want[i], got[i]);
    }
    if got.len() != want.len() && !r.log.iter().any(|l| l.ends_with(" resigned.")) {
        bail!("replay has {} log lines, saved file has {}", got.len(), want.len());
    }
    println!("OK: {} inputs replayed, {} log lines identical, final Soul {:+}", r.inputs.len(), got.len(), s.game.st.soul);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let d = db();
    match cli.cmd {
        Cmd::Profiles => {
            println!("AI personalities:");
            for p in d.profiles() {
                println!("  {:<28} {}", p.label, p.description);
            }
            for f in Faction::ALL {
                println!("\n{} decks:", f.short());
                for dl in d.decks(f) {
                    println!("  {:<20} {}", dl.name, d.symbols(&dl.cards));
                }
            }
        }
        Cmd::Matrix(o) => {
            let rv = rules(&o.rules)?;
            let m = run_matrix(&o, rv)?;
            print_matrix(&m, &format!("Rules v{} · goal ±{} · {} · {} games per matchup", rv.label(), o.goal, o.profile, o.games));
            print_cards(&m);
            if let Some(p) = &o.csv {
                write_csv(&m, p)?;
                println!("\nCSV written to {}", p.display());
            }
        }
        Cmd::Compare(o) => {
            let a = run_matrix(&o, RulesVersion::V1_0_1)?;
            let b = run_matrix(&o, RulesVersion::V1_0_2)?;
            print_matrix(&a, &format!("v1.0.1 · goal ±{} · {} · {} games per matchup", o.goal, o.profile, o.games));
            print_matrix(&b, &format!("v1.0.2 · goal ±{} · {} · {} games per matchup", o.goal, o.profile, o.games));
            println!("\nChange in Heaven win % (v1.0.2 − v1.0.1):");
            for (i, row) in b.cells.iter().enumerate() {
                print!("{:<20}", trunc(&b.heaven_decks[i], 19));
                for (j, c) in row.iter().enumerate() {
                    print!("{:>+16.1} ", c.heaven_pct() - a.cells[i][j].heaven_pct());
                }
                println!();
            }
            if let Some(p) = &o.csv {
                write_csv(&b, p)?;
            }
        }
        Cmd::Game { seed, heaven, hell, goal, rules: r, profile: pl } => {
            let prof = profile(d, &pl)?;
            let cfg = GameConfig {
                mode: GameMode::VsComputer,
                p1_faction: Faction::Heaven,
                p1_deck: deck(d, Faction::Heaven, &heaven)?,
                p2_deck: deck(d, Faction::Hell, &hell)?,
                goal,
                ai_profile: prof.clone(),
                names: GameConfig::default_names(GameMode::VsComputer),
                rules: rules(&r)?,
            };
            let g = Game::new(d, cfg, seed, Arc::new(FixedClock("(sim)".into())));
            let mut s = Session::start(g, [Controller::Human, Controller::Ai], seed);
            play_out(&mut s, [&prof, &prof], 400).expect("bots only make legal plays");
            for l in &s.game.log_lines {
                println!("{l}");
            }
        }
        Cmd::Replay { file } => replay(&file)?,
    }
    Ok(())
}
