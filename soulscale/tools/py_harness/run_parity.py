#!/usr/bin/env python3
"""Play seeded bot-vs-bot games through the ORIGINAL v1.0.1 Python engine, headless.

Writes one JSON object per line: the game config plus every log line the engine wrote.
`crates/soulscale-core/tests/python_parity.rs` replays the same seeds and configs through
the Rust engine and asserts the logs are identical, line for line.

Usage (repo root; Tkinter not required):

    python -I tools/py_harness/run_parity.py tools/py_harness/reference/soulscale_purgatory_v101.py \
        crates/soulscale-core/tests/fixtures/python_games.jsonl.gz --games 300

(An output name ending in .gz is written gzip-compressed, which is what the Rust test reads.)

What is real and what is stubbed:
  * Real: start_game, new_round, resolve_turn, resolve_hotseat_turn, every rules method,
    choose_ai_play / ai_should_reroll, stats, endgame summary.
  * Stubbed: Tk widgets/dialogs. The human dialogs are replaced by bot policies that use
    the engine's own AI (with a chosen personality) and log exactly what the dialogs log.
  * Clock: fixed at "2026-01-01 00:00:00" so summaries are comparable.
"""
from __future__ import annotations

import argparse
import gzip
import importlib.util
import json
import random
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE / "tkstub"))

FIXED_NOW = "2026-01-01 00:00:00"
MAX_ROUNDS = 400


class FakeVar:
    def __init__(self, value=None):
        self._v = value

    def get(self):
        return self._v

    def set(self, v):
        self._v = v


class FakeRoot:
    def __init__(self):
        self.pending = []

    def after(self, _ms, fn):
        self.pending.append(fn)


class _Now:
    def strftime(self, _fmt):
        return FIXED_NOW


class FixedDatetime:
    @staticmethod
    def now():
        return _Now()


UI_STUBS = [
    "render_player_reveal", "render_ai_reveal", "render_player_reveal_hidden", "refresh_status",
    "refresh_board", "draw_soul_track", "disable_card_buttons", "show_summary_window",
    "rebuild_log_widget", "mask_main_window_for_player2_secret_choice",
    "restore_public_reveals_after_secret_choice", "build_game_screen",
]


def load_module(path: Path):
    spec = importlib.util.spec_from_file_location("soulscale_v101_parity", path)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    mod.datetime = FixedDatetime  # summaries use datetime.now().strftime(...)
    return mod


def make_app(mod, cfg):
    A = mod.SoulscaleApp
    app = A.__new__(A)
    ANGEL, DEVIL = mod.ANGEL, mod.DEVIL
    p1 = ANGEL if cfg["p1_faction"] == "heaven" else DEVIL
    # Fields SoulscaleApp.__init__ sets (start_game resets the gameplay ones again).
    app.root = FakeRoot()
    app.rng = random.Random(cfg["seed"])
    app.soul = 0
    app.goal = cfg["goal"]
    app.round_no = 0
    app.strain = 0
    app.player_faction = p1
    app.ai_faction = mod.opponent(p1)
    app.player_deck_name = cfg["p1_deck"]
    app.ai_deck_name = cfg["p2_deck"]
    app.opponent_deck_name = cfg["p2_deck"]
    app.ai_profile_name = cfg["ai_profile"]
    app.ai_profile = mod.AI_PROFILES[cfg["ai_profile"]]
    app.game_mode = "ai"
    app.secret_hotseat = False
    app.secret_hotseat_show_p2 = False
    app.custom_decks = {ANGEL: {}, DEVIL: {}}
    app.player_deck = None
    app.ai_deck = None
    app.selected_card = None
    app.selected_card_button = None
    app.in_resolution = False
    app.game_over = False
    app.game_stats = mod.GameStats()
    app.log_lines = []
    app.last_endgame_summary = ""
    app.start_frame = None
    app.game_frame = None
    app.card_buttons = []
    app.chamber_buttons = []
    app.ai_committed_play = None
    for name in ("status_var", "selected_var", "soul_var", "will_var", "strain_var", "ai_reveal_var"):
        setattr(app, name, FakeVar(""))
    app.chamber_text_vars = {}
    # Setup-screen selections start_game reads.
    hotseat = cfg["mode"] in ("hotseat", "secret_hotseat")
    app.game_mode_var = FakeVar("hotseat" if hotseat else "ai")
    app.secret_hotseat_var = FakeVar(cfg["mode"] == "secret_hotseat")
    app.side_var = FakeVar(p1)
    app.deck_var = FakeVar(cfg["p1_deck"])
    app.opponent_deck_var = FakeVar(cfg["p2_deck"])
    app.ai_profile_var = FakeVar(cfg["ai_profile"])
    app.goal_var = FakeVar(cfg["goal"])

    for name in UI_STUBS:
        setattr(app, name, lambda *a, **k: None)
    app.auto_export_playtest_log = lambda summary: None
    app.show_secret_pass_screen = lambda: True

    player_profile = mod.AI_PROFILES[cfg["player_profile"]]

    def with_profile(profile, fn):
        saved = app.ai_profile
        app.ai_profile = profile
        try:
            return fn()
        finally:
            app.ai_profile = saved

    def p1_choose():
        # Drive the engine's own AI for seat 1 by swapping decks and personality.
        app.ai_deck, app.player_deck = app.player_deck, app.ai_deck
        try:
            return with_profile(player_profile, lambda: app.choose_ai_play(None, None, None))
        finally:
            app.ai_deck, app.player_deck = app.player_deck, app.ai_deck

    def ask_player_reroll(card, raw, intensity, mode):
        chamber = app._h_p1_chamber
        other = app.ai_committed_play[1] if app.ai_committed_play else None
        actor = "Player 1" if app.is_hotseat() else "You"
        if app.player_will > 0 and with_profile(
            player_profile, lambda: app.ai_should_reroll(card, chamber, mode, intensity, None, other)
        ):
            app.player_will = max(0, app.player_will - 1)
            new_raw = app.rng.randint(1, 6)
            app.log(f"{actor} spends 1 Will to reroll {raw} → {new_raw}.")
            return new_raw, mod.intensity_from_d6(new_raw)
        return raw, intensity

    def ask_second_human_play():
        card, chamber, mode = app.choose_ai_play(None, None, None)
        raw = app.rng.randint(1, 6)
        intensity = mod.intensity_from_d6(raw)
        if app.ai_will > 0 and app.ai_should_reroll(card, chamber, mode, intensity, None, app._h_p1_chamber):
            app.ai_will = max(0, app.ai_will - 1)
            new_raw = app.rng.randint(1, 6)
            app.log(f"Player 2 spends 1 Will to reroll {raw} → {new_raw}.")
            raw, intensity = new_raw, mod.intensity_from_d6(new_raw)
        return card, chamber, mode, raw, intensity

    app.ask_player_reroll = ask_player_reroll
    app.ask_second_human_play = ask_second_human_play
    app._h_p1_choose = p1_choose
    return app


def play(mod, cfg):
    app = make_app(mod, cfg)
    app.start_game()
    while not app.game_over and app.round_no < MAX_ROUNDS:
        card, chamber, mode = app._h_p1_choose()
        app._h_p1_chamber = chamber
        if app.is_hotseat():
            app.resolve_hotseat_turn(card, chamber, mode)
        else:
            app.resolve_turn(card, chamber, mode)
        while app.root.pending:
            app.root.pending.pop(0)()
    return {
        "cfg": cfg,
        "rounds": app.round_no,
        "soul": app.soul,
        "winner": app.game_stats.winner_faction,
        "log": app.log_lines,
    }


def configs(mod, n):
    heaven = list(mod.DECKS[mod.ANGEL].keys())
    hell = list(mod.DECKS[mod.DEVIL].keys())
    profiles = list(mod.AI_PROFILES.keys())
    modes = ["vs_computer", "vs_computer", "hotseat", "secret_hotseat"]
    goals = [12, 18, 24]
    for i in range(n):
        p1_heaven = i % 2 == 0
        hd, ld = heaven[i % len(heaven)], hell[(i // 2) % len(hell)]
        yield {
            "seed": 1000 + i * 7919,
            "mode": modes[i % len(modes)],
            "p1_faction": "heaven" if p1_heaven else "hell",
            "p1_deck": hd if p1_heaven else ld,
            "p2_deck": ld if p1_heaven else hd,
            "ai_profile": profiles[i % len(profiles)],
            "player_profile": profiles[(i // 5 + 2) % len(profiles)],
            "goal": goals[(i // 3) % len(goals)],
        }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("module")
    ap.add_argument("out")
    ap.add_argument("--games", type=int, default=300)
    args = ap.parse_args()
    mod = load_module(Path(args.module))
    n_lines = 0
    opener = gzip.open if args.out.endswith(".gz") else open
    with opener(args.out, "wt", encoding="utf-8") as fh:
        for cfg in configs(mod, args.games):
            rec = play(mod, cfg)
            n_lines += len(rec["log"])
            fh.write(json.dumps(rec, ensure_ascii=False) + "\n")
    print(f"{args.games} games, {n_lines} log lines -> {args.out} (python {sys.version.split()[0]})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
