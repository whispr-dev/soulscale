//! Replay determinism, the rulebook §19 walkthrough, and session invariants.

use std::sync::Arc;

use soulscale_core::game::PlayCommit;
use soulscale_core::rng::PyRandom;
use soulscale_core::session::{Controller, SeatStatus, Session};
use soulscale_core::view::Viewer;
use soulscale_core::{Chamber, Choice, Faction, FixedClock, Game, GameConfig, GameMode, Intent, NamedDeck, RulesVersion, Seat, db};

fn cfg(mode: GameMode, seed_deck: usize) -> GameConfig {
    let d = db();
    let h: Vec<_> = d.decks(Faction::Heaven).collect();
    let l: Vec<_> = d.decks(Faction::Hell).collect();
    let hd = h[seed_deck % h.len()];
    let ld = l[(seed_deck / 2) % l.len()];
    GameConfig {
        mode,
        p1_faction: Faction::Heaven,
        p1_deck: NamedDeck { name: hd.name.clone(), cards: hd.cards.clone() },
        p2_deck: NamedDeck { name: ld.name.clone(), cards: ld.cards.clone() },
        goal: 18,
        ai_profile: d.profiles()[seed_deck % 5].clone(),
        names: GameConfig::default_names(mode),
        rules: RulesVersion::LATEST,
    }
}

fn clock() -> Arc<FixedClock> {
    Arc::new(FixedClock("2026-01-01 00:00:00".into()))
}

/// A "human" that picks random legal plays from its OWN rng (real humans don't touch the
/// game's RNG), so recorded inputs alone must reproduce the match.
fn random_human(s: &mut Session, seat: Seat, r: &mut PyRandom) -> bool {
    match s.status(seat) {
        SeatStatus::Choose => {
            let choice = if let Some(f) = s.game.forced_choice(seat) {
                f
            } else {
                let hand = s.game.revealed(seat).to_vec();
                loop {
                    let card = hand[r.choice_index(hand.len())];
                    let def = s.game.card(card);
                    let chamber = def.valid_chambers[r.choice_index(def.valid_chambers.len())];
                    if s.game.st.is_chamber_locked(def.faction, chamber) {
                        continue;
                    }
                    let own = s.game.st.stack_total(def.faction, chamber);
                    let intent = match r.randint(0, 3) {
                        0 => Intent::Press,
                        1 => Intent::Anchor,
                        2 if own > 0 => Intent::Release { spend: r.randint(1, own) },
                        _ => Intent::Sever,
                    };
                    break Choice { card, chamber, intent };
                }
            };
            s.choose(seat, choice).unwrap();
            true
        }
        SeatStatus::Reroll { .. } => {
            let reroll = r.randint(0, 2) == 0;
            s.decide_reroll(seat, reroll).unwrap();
            true
        }
        _ => false,
    }
}

#[test]
fn recorded_inputs_replay_identically_in_every_mode() {
    for (i, mode) in [GameMode::VsComputer, GameMode::Hotseat, GameMode::SecretHotseat, GameMode::Duel]
        .into_iter()
        .cycle()
        .take(60)
        .enumerate()
    {
        let seed = 77 + i as u64 * 31;
        let controllers =
            if mode == GameMode::VsComputer { [Controller::Human, Controller::Ai] } else { [Controller::Human; 2] };
        let mut live = Session::start(Game::new(db(), cfg(mode, i), seed, clock()), controllers, seed);
        let mut humans = PyRandom::new(seed ^ 0xdead_beef);
        let mut guard = 0;
        while !live.game.is_over() && live.game.st.round < 300 {
            // Duel: the order players finish in varies; simulate that with the human rng.
            let first = if humans.randint(0, 1) == 0 { Seat::P1 } else { Seat::P2 };
            if !random_human(&mut live, first, &mut humans) {
                random_human(&mut live, first.other(), &mut humans);
            }
            // Occasionally hand a Duel seat to the AI and back (disconnect / reclaim).
            if mode == GameMode::Duel && humans.randint(0, 40) == 0 {
                live.set_controller(Seat::P2, Controller::Ai);
                live.set_controller(Seat::P2, Controller::Human);
            }
            guard += 1;
            assert!(guard < 5000, "no progress");
        }
        let mut replay = Session::start(Game::new(db(), cfg(mode, i), seed, clock()), controllers, seed);
        for input in &live.inputs {
            replay.apply_input(input).unwrap();
        }
        assert_eq!(replay.game.log_lines, live.game.log_lines, "mode {mode:?} seed {seed}");
        assert_eq!(replay.game.st.soul, live.game.st.soul);
    }
}

#[test]
fn rulebook_section_19_walkthrough() {
    let d = db();
    let mut g = Game::new(d, cfg(GameMode::VsComputer, 0), 1, clock());
    let candle = d.id("candle").unwrap();
    let ache = d.id("ache").unwrap();
    g.resolve(
        PlayCommit { card: candle, chamber: Chamber::Body, intent: Intent::Press, raw: 4 },
        PlayCommit { card: ache, chamber: Chamber::Body, intent: Intent::Press, raw: 3 },
    );
    let log = g.log_lines.join("\n");
    assert!(log.contains("Clash in 🫀 Body: You 4 vs Computer 3."), "{log}");
    assert!(log.contains("You wins the clash. Computer effect is halved."), "{log}");
    assert!(log.contains("🕯️ Candle of Warmth contributes +4 toward Heaven."), "{log}");
    assert!(log.contains("Heaven adds 2 stack to Body from 🕯️ Warmth."), "{log}");
    assert!(log.contains("🦷 Ache contributes -1 toward Hell."), "{log}");
    assert!(log.contains("Hell adds 2 stack to Body from 🦷 Pain."), "{log}");
    assert!(log.contains("Ongoing 🦷 Ache in Body contributes -1."), "{log}");
    assert!(log.contains("net +2."), "{log}");
    assert_eq!(g.st.soul, 2);
}

#[test]
fn duel_hides_hands_and_choices_until_resolution() {
    let mut s = Session::start(Game::new(db(), cfg(GameMode::Duel, 3), 5, clock()), [Controller::Human; 2], 5);
    let v1 = s.view(Viewer::Seat(Seat::P1));
    assert!(v1.seats[0].hand.is_some());
    assert!(v1.seats[1].hand.is_none(), "opponent hand must be hidden in a Duel");
    let spectator = s.view(Viewer::Spectator);
    assert!(spectator.seats[0].hand.is_none() && spectator.seats[1].hand.is_none());
    // P1 commits; P2 must not learn the choice or roll.
    let hand = s.game.revealed(Seat::P1).to_vec();
    let card = hand[0];
    let def = s.game.card(card);
    let chamber = *def.valid_chambers.iter().find(|c| !s.game.st.is_chamber_locked(def.faction, **c)).unwrap();
    s.choose(Seat::P1, Choice { card, chamber, intent: Intent::Press }).unwrap();
    s.drain();
    let v2 = s.view(Viewer::Seat(Seat::P2));
    assert!(v2.seats[0].choice.is_none() && v2.seats[0].status.is_none());
    assert_eq!(v2.seats[0].public_status, "reroll");
    s.decide_reroll(Seat::P1, true).unwrap();
    let ev = s.drain();
    // The reroll log is private to P1 now...
    assert!(ev.iter().all(|(vis, _)| !Viewer::Seat(Seat::P2).sees(*vis)));
    // ...and revealed to P2 when the round resolves.
    let hand2 = s.game.revealed(Seat::P2).to_vec();
    let c2 = hand2[0];
    let d2 = s.game.card(c2);
    let ch2 = *d2.valid_chambers.iter().find(|c| !s.game.st.is_chamber_locked(d2.faction, **c)).unwrap();
    s.choose(Seat::P2, Choice { card: c2, chamber: ch2, intent: Intent::Anchor }).unwrap();
    if matches!(s.status(Seat::P2), SeatStatus::Reroll { .. }) {
        s.decide_reroll(Seat::P2, false).unwrap();
    }
    let ev = s.drain();
    let p2_sees: Vec<_> = ev.iter().filter(|(v, _)| Viewer::Seat(Seat::P2).sees(*v)).collect();
    let text = format!("{p2_sees:?}");
    assert!(text.contains("spends 1 Will to reroll"), "held reroll should reach P2 at resolution");
    assert_eq!(s.game.st.round, 2, "round resolved and the next began");
}

#[test]
fn illegal_choices_are_rejected() {
    let mut s = Session::start(Game::new(db(), cfg(GameMode::VsComputer, 0), 9, clock()), [Controller::Human, Controller::Ai], 9);
    let d = db();
    let not_in_hand = d.faction_cards(Faction::Heaven).map(|(id, _)| id).find(|id| !s.game.revealed(Seat::P1).contains(id)).unwrap();
    let err = s.choose(Seat::P1, Choice { card: not_in_hand, chamber: Chamber::Body, intent: Intent::Press });
    assert!(err.is_err());
    let card = s.game.revealed(Seat::P1)[0];
    let def = s.game.card(card);
    let bad = Chamber::ALL.into_iter().find(|c| !def.valid_chambers.contains(c)).unwrap();
    assert!(s.choose(Seat::P1, Choice { card, chamber: bad, intent: Intent::Press }).is_err());
    let good = def.valid_chambers[0];
    assert!(s.choose(Seat::P1, Choice { card, chamber: good, intent: Intent::Release { spend: 1 } }).is_err(), "no stack to release");
    assert!(s.choose(Seat::P2, Choice { card, chamber: good, intent: Intent::Press }).is_err(), "AI seat");
}
