#!/usr/bin/env python3
"""Dump CARDS / DECKS / AI_PROFILES from the v1.0.1 Python engine into the Rust core's data file.

Run from the repo root (Python 3.9+; Tkinter NOT required):

    python -I tools/py_harness/dump_card_data.py tools/py_harness/reference/soulscale_purgatory_v101.py crates/soulscale-core/data/cards.json

Reading the data straight out of the live Python module (rather than retyping it) means
emoji variation selectors, help text and the rarity patch are carried across exactly.
"""
from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE / "tkstub"))

FACTION = {"angel": "heaven", "devil": "hell"}


def load_module(path: Path):
    spec = importlib.util.spec_from_file_location("soulscale_v101", path)
    if spec is None or spec.loader is None:
        raise SystemExit(f"cannot import {path}")
    mod = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = mod  # dataclasses look the module up while decorating
    spec.loader.exec_module(mod)
    return mod


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    mod = load_module(Path(sys.argv[1]))
    cards = []
    for key, c in mod.CARDS.items():
        cards.append(
            {
                "key": c.key,
                "faction": FACTION[c.faction],
                "emoji": c.emoji,
                "name": c.name,
                "valid_chambers": list(c.valid_chambers),
                "type_label": c.type_label,
                "aspect": c.aspect,
                "stack_value": c.stack_value,
                "rarity": c.rarity,
                "prices": list(c.prices),
                "movement_type": c.movement_type,
                "base_power": c.base_power,
                "base_move": c.base_move,
                "ongoing_move": c.ongoing_move,
                "ongoing": c.ongoing,
                "trap": c.trap,
                "guard": c.guard,
                "special": c.special,
                "help_text": c.help_text,
            }
        )
    decks = {
        FACTION[f]: [{"name": name, "cards": list(keys)} for name, keys in by_name.items()]
        for f, by_name in mod.DECKS.items()
    }
    profiles = [
        {
            "label": p.label,
            "description": p.description,
            "aggression": p.aggression,
            "planning": p.planning,
            "disruption": p.disruption,
            "release_hunger": p.release_hunger,
            "randomness": p.randomness,
            "reroll_bias": p.reroll_bias,
        }
        for p in mod.AI_PROFILES.values()
    ]
    out = {
        "rules_version": "1.0.1",
        "source": "soulscale_purgatory_v101.py (auto-dumped by tools/py_harness/dump_card_data.py)",
        "cards": cards,
        "decks": decks,
        "ai_profiles": profiles,
    }
    Path(sys.argv[2]).parent.mkdir(parents=True, exist_ok=True)
    Path(sys.argv[2]).write_text(json.dumps(out, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {len(cards)} cards, {sum(len(v) for v in decks.values())} decks, {len(profiles)} AI profiles")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
