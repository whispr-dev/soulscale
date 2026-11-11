#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Soulscale: Purgatory v1.0.1 — rules-aware movement resolver.

Run:
    python soulscale_purgatory_v101.py

Requires:
    Python 3.9+ with Tkinter.

What v0.3 added over v0.2:
    - optional Human vs Human hot-seat mode for playtesting with friends
    - separate opponent deck selection on the setup screen
    - Human vs AI mode remains intact, with AI personalities/difficulty profiles
    - user-created deck loading from .txt files, one emoji per line, exactly 10 glyphs
    - Chamber stacking via Resonance tokens and player intent choices

What v0.4 fixed/added:
    - clearer per-round movement audit so Hell/Heaven contributions are visible separately
    - initial polarity guard so accidental wrong-way scoring was blocked

What v0.5 changed:
    - AI secretly commits its play before the human chooses, removing omniscient counterplay
    - Release is capped and softened: min(5, ceil(spent stack × intensity / 4))
    - Anchor is now a true investment: no immediate Soul movement, stronger stack growth
    - setup screen supports Quick / Standard / Deep goal lengths: ±12 / ±18 / ±24
    - game-over locks input more aggressively

What v0.6 added:
    - end-of-game summary with rounds played, winner, final Soul, biggest move,
      most-used Chamber, intent distribution, and total Heaven/Hell pressure
    - summary is logged and shown in a copyable window for playtest notes

What v0.7 adds:
    - playtest log export as a full .txt report plus a per-round .csv ledger
    - automatic export at game end to ./soulscale_playtest_logs/
    - manual Export Log button during or after a game

What v0.7.1 tried to fix:
    - attempted to make the endgame Save full log button more visible

What v0.7.4 fixes:
    - removes the misleading smoke test
    - adds a real forced completed-game endgame/export test path from the start screen
    - replaces the popup with a defensive export window that falls back instead of going blank
    - updates title/version strings so stale builds are immediately obvious

What v0.8 added:
    - built-in visual deck editor on the start screen
    - click glyph cards into a 10-card custom deck
    - validate deck legality live
    - save one-emoji-per-line .txt decks
    - automatically load saved editor decks into Player 1 or opponent deck slots



What v0.9 adds:
    - optional Secret Hot-seat mode for two-human playtesting
    - Player 2 hand is hidden from Player 1 during Player 1 choice
    - Player 1 choice/hand/log details are masked before Player 2 chooses
    - Player 2 chooses from a modal secret panel, then both plays are revealed together

What v1.0 adds:
    - 16 new glyphs: 8 Heaven and 8 Hell
    - rarity metadata: Common, Uncommon, Rare
    - price keywords: Heavy, Fragile, Volatile, Delayed, Binding, Hungry, Echo, Desperate, Narrow
    - deck size increased to 10; reveal size increased to 4
    - deck editor shows rarity/budget/price legality
    - existing fair v0.9 stack/intent/dice core preserved

What v1.0.1 fixes:
    - replaces the blunt polarity guard with a rules-aware movement resolver
    - cards now declare movement_type: ownerward, opponentward, heavenward, hellward, or neutral
    - normal cards remain protected from wrong-way scoring bugs
    - future cost/bait/sacrifice cards can intentionally move toward the opponent without being auto-corrected

The design stays intentionally compact: a small board, emoji-readable pieces, and a
simple Heaven/Hell Soul track, but now the best play is not always the largest
immediate push. Good players build, protect, cut, and release stacks.
"""

from __future__ import annotations

import csv
import math
import random
import sys
import tkinter as tk
from dataclasses import dataclass, field, replace
from datetime import datetime
from pathlib import Path
from tkinter import filedialog, messagebox
from typing import Dict, Iterable, List, Optional, Sequence, Tuple


# -----------------------------------------------------------------------------
# Core constants
# -----------------------------------------------------------------------------

CHAMBERS = ["Body", "Mind", "Heart", "Time", "Memory"]
CHAMBER_EMOJI = {
    "Body": "🫀",
    "Mind": "🧠",
    "Heart": "❤️",
    "Time": "🕰️",
    "Memory": "🪞",
}

ANGEL = "angel"
DEVIL = "devil"

FACTION_LABEL = {
    ANGEL: "Heaven / Angel",
    DEVIL: "Hell / Devil",
}

FACTION_SHORT = {
    ANGEL: "Heaven",
    DEVIL: "Hell",
}

FACTION_SIGN = {
    ANGEL: 1,
    DEVIL: -1,
}

MOVEMENT_OWNERWARD = "ownerward"
MOVEMENT_OPPONENTWARD = "opponentward"
MOVEMENT_HEAVENWARD = "heavenward"
MOVEMENT_HELLWARD = "hellward"
MOVEMENT_NEUTRAL = "neutral"
MOVEMENT_TYPES = {
    MOVEMENT_OWNERWARD,
    MOVEMENT_OPPONENTWARD,
    MOVEMENT_HEAVENWARD,
    MOVEMENT_HELLWARD,
    MOVEMENT_NEUTRAL,
}
MOVEMENT_TYPE_LABEL = {
    MOVEMENT_OWNERWARD: "ownerward",
    MOVEMENT_OPPONENTWARD: "opponentward",
    MOVEMENT_HEAVENWARD: "heavenward",
    MOVEMENT_HELLWARD: "hellward",
    MOVEMENT_NEUTRAL: "neutral",
}

DECK_SIZE = 10
LEGACY_DECK_SIZE = 8
REVEAL_SIZE = 4
RARITY_COST = {"Common": 1, "Uncommon": 2, "Rare": 3}
DECK_RARITY_BUDGET = 24
DECK_MAX_RARES = 4
MAX_VISIBLE_STACK_TOKENS = 9


# -----------------------------------------------------------------------------
# Data model
# -----------------------------------------------------------------------------

@dataclass(frozen=True)
class Card:
    key: str
    faction: str
    emoji: str
    name: str
    valid_chambers: Tuple[str, ...]
    type_label: str
    aspect: str
    stack_value: int
    rarity: str = "Common"
    prices: Tuple[str, ...] = ()
    movement_type: str = MOVEMENT_OWNERWARD
    base_power: int = 1
    base_move: int = 0
    ongoing_move: int = 0
    ongoing: bool = False
    trap: bool = False
    guard: bool = False
    special: str = ""
    help_text: str = ""

    @property
    def short(self) -> str:
        return f"{self.emoji} {self.name}"


@dataclass
class BoardEffect:
    faction: str
    card_key: str
    chamber: str
    remaining: int = -1

    @property
    def card(self) -> Card:
        return CARDS[self.card_key]


@dataclass
class StackToken:
    faction: str
    card_key: str
    aspect: str
    value: int

    @property
    def card(self) -> Card:
        return CARDS[self.card_key]


@dataclass(frozen=True)
class ModeChoice:
    key: str
    label: str
    spend: int = 0


@dataclass
class PendingPlay:
    card: Card
    chamber: str
    mode: ModeChoice
    raw_roll: int
    intensity: int
    power: int
    amount: int
    stack_add: int = 0
    stack_spend: int = 0
    sever_amount: int = 0
    resonance_bonus: int = 0
    blocked: bool = False
    half: bool = False


@dataclass(frozen=True)
class AiProfile:
    label: str
    description: str
    aggression: float
    planning: float
    disruption: float
    release_hunger: float
    randomness: float
    reroll_bias: float


@dataclass
class GameStats:
    started_at: str = ""
    ended_at: str = ""
    winner_faction: str = ""
    last_export_path: str = ""
    round_results: List[Dict[str, object]] = field(default_factory=list)
    chamber_uses: Dict[str, int] = field(default_factory=dict)
    intent_uses: Dict[str, int] = field(default_factory=dict)
    total_heaven_pressure: int = 0
    total_hell_pressure: int = 0
    total_net_movement: int = 0
    biggest_single_abs: int = 0
    biggest_single_label: str = "No scoring play recorded yet."
    biggest_round_abs: int = 0
    biggest_round_label: str = "No Soul movement recorded yet."


AI_PROFILES: Dict[str, AiProfile] = {
    "Novice / Forgiving": AiProfile(
        label="Novice / Forgiving",
        description="Loose and a bit wasteful. Good for learning the board.",
        aggression=0.75,
        planning=0.45,
        disruption=0.45,
        release_hunger=0.35,
        randomness=2.2,
        reroll_bias=0.25,
    ),
    "Balanced / Tactical": AiProfile(
        label="Balanced / Tactical",
        description="Reasonable tactical play with some personality noise.",
        aggression=1.0,
        planning=1.0,
        disruption=0.9,
        release_hunger=0.85,
        randomness=0.9,
        reroll_bias=0.65,
    ),
    "Architect / Stack Builder": AiProfile(
        label="Architect / Stack Builder",
        description="Prefers Anchor, protects strong Chambers, then releases hard.",
        aggression=0.9,
        planning=1.75,
        disruption=0.8,
        release_hunger=1.1,
        randomness=0.65,
        reroll_bias=0.7,
    ),
    "Cruel Accountant / Hard": AiProfile(
        label="Cruel Accountant / Hard",
        description="Calculating, low-noise, contests leverage and rerolls important plays.",
        aggression=1.25,
        planning=1.25,
        disruption=1.35,
        release_hunger=1.3,
        randomness=0.25,
        reroll_bias=1.0,
    ),
    "Chaos Imp / Weird": AiProfile(
        label="Chaos Imp / Weird",
        description="High-variance gremlin. Sometimes brilliant, sometimes soup.",
        aggression=1.05,
        planning=0.65,
        disruption=1.1,
        release_hunger=1.45,
        randomness=2.8,
        reroll_bias=0.8,
    ),
}


# -----------------------------------------------------------------------------
# Cards and built-in decks
# -----------------------------------------------------------------------------

CARDS: Dict[str, Card] = {
    # Heaven / Angel
    "candle": Card(
        key="candle",
        faction=ANGEL,
        emoji="🕯️",
        name="Candle of Warmth",
        valid_chambers=("Body", "Heart"),
        type_label="Direct / Guard",
        aspect="Warmth",
        stack_value=2,
        base_power=1,
        base_move=1,
        special="body_bonus",
        help_text="A reliable comfort push. +1 extra in Body.",
    ),
    "open_hand": Card(
        key="open_hand",
        faction=ANGEL,
        emoji="🤲",
        name="Open Hand",
        valid_chambers=("Heart", "Memory"),
        type_label="Direct",
        aspect="Mercy",
        stack_value=2,
        base_power=1,
        base_move=1,
        special="behind_bonus",
        help_text="A mercy play. Stronger if the Soul is below 0.",
    ),
    "calm_breath": Card(
        key="calm_breath",
        faction=ANGEL,
        emoji="🌊",
        name="Calm Breath",
        valid_chambers=("Mind", "Body", "Heart"),
        type_label="Disruption / Cleanse",
        aspect="Calm",
        stack_value=1,
        base_power=2,
        base_move=0,
        special="calm",
        help_text="Weakens opposing plays in the same Chamber, or cleanses Devil effects.",
    ),
    "hymn": Card(
        key="hymn",
        faction=ANGEL,
        emoji="🎵",
        name="Hymn",
        valid_chambers=("Mind", "Heart"),
        type_label="Ongoing",
        aspect="Harmony",
        stack_value=2,
        base_power=1,
        base_move=0,
        ongoing_move=1,
        ongoing=True,
        special="hymn",
        help_text="Ongoing: pushes +1 each round.",
    ),
    "rest": Card(
        key="rest",
        faction=ANGEL,
        emoji="🛏️",
        name="Rest",
        valid_chambers=("Body", "Time"),
        type_label="Direct / Cleanse",
        aspect="Rest",
        stack_value=2,
        base_power=1,
        base_move=1,
        special="rest",
        help_text="Pushes Heaven and removes one Devil effect in this Chamber.",
    ),
    "loved_memory": Card(
        key="loved_memory",
        faction=ANGEL,
        emoji="💌",
        name="Loved Memory",
        valid_chambers=("Memory", "Heart"),
        type_label="Direct / Cleanse",
        aspect="Memory",
        stack_value=3,
        base_power=2,
        base_move=2,
        special="loved_memory",
        help_text="Strong Heaven push. In Memory, dissolves one Devil effect before scoring.",
    ),
    "guardian": Card(
        key="guardian",
        faction=ANGEL,
        emoji="🪽",
        name="Guardian",
        valid_chambers=("Body", "Mind", "Heart", "Time", "Memory"),
        type_label="Ongoing / Guard",
        aspect="Sanctuary",
        stack_value=2,
        base_power=2,
        base_move=0,
        ongoing=True,
        guard=True,
        special="guardian",
        help_text="Ongoing guard: Devil plays in this Chamber lose 2 power.",
    ),
    "little_miracle": Card(
        key="little_miracle",
        faction=ANGEL,
        emoji="✨",
        name="Little Miracle",
        valid_chambers=("Body", "Mind", "Heart", "Time", "Memory"),
        type_label="Wild",
        aspect="Wonder",
        stack_value=1,
        base_power=1,
        base_move=0,
        special="miracle",
        help_text="Wild: rolls for Will or a burst Heaven push.",
    ),
    "hope": Card(
        key="hope",
        faction=ANGEL,
        emoji="🌤️",
        name="Hope",
        valid_chambers=("Heart", "Time"),
        type_label="Scaling",
        aspect="Hope",
        stack_value=2,
        base_power=1,
        base_move=1,
        special="behind_bonus",
        help_text="Pushes Heaven. Stronger when Heaven is losing.",
    ),
    "bell": Card(
        key="bell",
        faction=ANGEL,
        emoji="🔔",
        name="Bell of Mercy",
        valid_chambers=("Body", "Mind", "Heart", "Time", "Memory"),
        type_label="Disruption",
        aspect="Mercy",
        stack_value=1,
        base_power=2,
        base_move=0,
        special="bell",
        help_text="Cancels one Devil ongoing/trap in the chosen Chamber.",
    ),
    "small_comfort": Card(
        key="small_comfort",
        faction=ANGEL,
        emoji="🧸",
        name="Small Comfort",
        valid_chambers=("Body", "Heart"),
        type_label="Support",
        aspect="Comfort",
        stack_value=2,
        base_power=1,
        base_move=1,
        special="support_next",
        help_text="Small push. Your next Heaven effect gets +1.",
    ),
    "sweetness": Card(
        key="sweetness",
        faction=ANGEL,
        emoji="🍯",
        name="Sweetness",
        valid_chambers=("Body", "Heart"),
        type_label="Direct / Will",
        aspect="Pleasure",
        stack_value=2,
        base_power=1,
        base_move=1,
        special="sweetness",
        help_text="Pushes Heaven. If Heaven is ahead, gain +1 Will next round.",
    ),


    "lightness": Card(
        key="lightness",
        faction=ANGEL,
        emoji="🪶",
        name="Lightness",
        valid_chambers=("Body", "Mind", "Heart"),
        type_label="Direct / Echo",
        aspect="Ease",
        stack_value=1,
        rarity="Common",
        prices=("Echo",),
        base_power=1,
        base_move=1,
        special="lightness",
        help_text="Gentle push. Echo repeats as a small +1 Heaven nudge next round.",
    ),
    "purpose": Card(
        key="purpose",
        faction=ANGEL,
        emoji="🧭",
        name="Purpose",
        valid_chambers=("Mind", "Time"),
        type_label="Direct / Comeback",
        aspect="Purpose",
        stack_value=2,
        rarity="Uncommon",
        prices=("Desperate",),
        base_power=1,
        base_move=1,
        special="purpose",
        help_text="Desperate: stronger while Heaven is behind, weaker while Heaven is already ahead.",
    ),
    "sanctuary": Card(
        key="sanctuary",
        faction=ANGEL,
        emoji="🏰",
        name="Sanctuary",
        valid_chambers=("Body", "Heart"),
        type_label="Ongoing / Guard",
        aspect="Sanctuary",
        stack_value=3,
        rarity="Rare",
        prices=("Heavy", "Binding"),
        base_power=3,
        base_move=0,
        ongoing=True,
        guard=True,
        special="sanctuary",
        help_text="Heavy Binding guard. Very strong defense, but costs Will and locks that Chamber next round.",
    ),
    "dawn": Card(
        key="dawn",
        faction=ANGEL,
        emoji="☀️",
        name="Dawn",
        valid_chambers=("Time", "Heart"),
        type_label="Direct / Delayed",
        aspect="Renewal",
        stack_value=2,
        rarity="Rare",
        prices=("Delayed",),
        base_power=2,
        base_move=2,
        special="dawn",
        help_text="Delayed: schedules its Heaven movement for the next round instead of scoring immediately.",
    ),
    "tea_quiet": Card(
        key="tea_quiet",
        faction=ANGEL,
        emoji="🫖",
        name="Tea & Quiet",
        valid_chambers=("Mind",),
        type_label="Direct / Narrow",
        aspect="Calm",
        stack_value=1,
        rarity="Common",
        prices=("Narrow",),
        base_power=2,
        base_move=1,
        special="tea_quiet",
        help_text="Efficient Mind-only calm. Narrow but reliable.",
    ),
    "acceptance": Card(
        key="acceptance",
        faction=ANGEL,
        emoji="🪷",
        name="Acceptance",
        valid_chambers=("Mind", "Memory"),
        type_label="Disruption / Cleanse",
        aspect="Acceptance",
        stack_value=2,
        rarity="Uncommon",
        prices=("Hungry",),
        base_power=2,
        base_move=1,
        special="acceptance",
        help_text="Hungry cleanse. Consumes friendly stack if possible; removes a Devil effect/condition here.",
    ),
    "silver_thread": Card(
        key="silver_thread",
        faction=ANGEL,
        emoji="🧵",
        name="Silver Thread",
        valid_chambers=("Heart", "Memory"),
        type_label="Support / Fragile",
        aspect="Connection",
        stack_value=3,
        rarity="Rare",
        prices=("Fragile",),
        base_power=1,
        base_move=1,
        special="support_next",
        help_text="Fragile support. Builds a powerful connection stack and boosts the next Heaven effect.",
    ),
    "grace": Card(
        key="grace",
        faction=ANGEL,
        emoji="🕊️",
        name="Grace",
        valid_chambers=("Heart", "Memory"),
        type_label="Direct / Rescue",
        aspect="Grace",
        stack_value=2,
        rarity="Rare",
        prices=("Heavy", "Desperate"),
        base_power=2,
        base_move=2,
        special="grace",
        help_text="Heavy rescue. Stronger when Heaven is behind; expensive when you are trying to turn the tide.",
    ),

    # Hell / Devil
    "ache": Card(
        key="ache",
        faction=DEVIL,
        emoji="🦷",
        name="Ache",
        valid_chambers=("Body",),
        type_label="Direct / Ongoing",
        aspect="Pain",
        stack_value=2,
        base_power=1,
        base_move=1,
        ongoing_move=1,
        ongoing=True,
        special="ache",
        help_text="Pushes Hell and leaves ongoing Body pain.",
    ),
    "screech": Card(
        key="screech",
        faction=DEVIL,
        emoji="🔊",
        name="Screech",
        valid_chambers=("Mind",),
        type_label="Direct",
        aspect="Noise",
        stack_value=2,
        base_power=1,
        base_move=1,
        special="screech",
        help_text="Pushes Hell. Stronger if the Soul is already below 0.",
    ),
    "fly_swarm": Card(
        key="fly_swarm",
        faction=DEVIL,
        emoji="🪰",
        name="Fly Swarm",
        valid_chambers=("Body", "Mind"),
        type_label="Ongoing",
        aspect="Irritation",
        stack_value=2,
        base_power=1,
        base_move=0,
        ongoing_move=1,
        ongoing=True,
        special="fly",
        help_text="Ongoing: pushes -1 each round.",
    ),
    "void_thought": Card(
        key="void_thought",
        faction=DEVIL,
        emoji="🕳️",
        name="Void Thought",
        valid_chambers=("Mind", "Time"),
        type_label="Scaling",
        aspect="Despair",
        stack_value=2,
        base_power=1,
        base_move=1,
        special="behind_bonus",
        help_text="Pushes Hell. Stronger when Hell is losing.",
    ),
    "memory_thorn": Card(
        key="memory_thorn",
        faction=DEVIL,
        emoji="🥀",
        name="Memory Thorn",
        valid_chambers=("Memory", "Heart"),
        type_label="Direct / Condition",
        aspect="Regret",
        stack_value=3,
        base_power=2,
        base_move=1,
        special="thorn",
        help_text="Pushes Hell and leaves Thorn to weaken Loved Memory.",
    ),
    "burden": Card(
        key="burden",
        faction=DEVIL,
        emoji="⛓️",
        name="Burden",
        valid_chambers=("Body", "Time"),
        type_label="Direct / Delay",
        aspect="Weight",
        stack_value=2,
        base_power=1,
        base_move=1,
        special="burden",
        help_text="Pushes Hell and reduces opponent Will next round.",
    ),
    "false_door": Card(
        key="false_door",
        faction=DEVIL,
        emoji="🪤",
        name="False Door",
        valid_chambers=("Body", "Mind", "Heart", "Time", "Memory"),
        type_label="Trap",
        aspect="Trap",
        stack_value=1,
        base_power=2,
        base_move=0,
        trap=True,
        special="false_door",
        help_text="Trap: next Angel play in this Chamber loses 3 power, then vanishes.",
    ),
    "watching_eye": Card(
        key="watching_eye",
        faction=DEVIL,
        emoji="👁️",
        name="Watching Eye",
        valid_chambers=("Mind", "Memory"),
        type_label="Control",
        aspect="Dread",
        stack_value=1,
        base_power=2,
        base_move=0,
        special="watching_eye",
        help_text="Gives Hell +1 Will next round and a small Hell push.",
    ),
    "endless_wait": Card(
        key="endless_wait",
        faction=DEVIL,
        emoji="⏳",
        name="Endless Wait",
        valid_chambers=("Time",),
        type_label="Ongoing / Cap",
        aspect="Stasis",
        stack_value=2,
        base_power=2,
        base_move=0,
        ongoing_move=1,
        ongoing=True,
        special="endless_wait",
        help_text="Ongoing Hell pressure. Caps Heaven movement next round.",
    ),
    "cold_room": Card(
        key="cold_room",
        faction=DEVIL,
        emoji="🧊",
        name="Cold Room",
        valid_chambers=("Body", "Heart"),
        type_label="Direct / Condition",
        aspect="Cold",
        stack_value=2,
        base_power=1,
        base_move=1,
        special="cold",
        help_text="Pushes Hell and leaves Cold, weakening warmth/comfort here.",
    ),
    "tangled_desire": Card(
        key="tangled_desire",
        faction=DEVIL,
        emoji="🧵",
        name="Tangled Desire",
        valid_chambers=("Heart",),
        type_label="Disruption",
        aspect="Frustration",
        stack_value=1,
        base_power=2,
        base_move=0,
        special="tangle",
        help_text="Disrupts Heart. Weakens opposing Heart plays and can Sever stack well.",
    ),
    "serpent": Card(
        key="serpent",
        faction=DEVIL,
        emoji="🐍",
        name="Whispering Serpent",
        valid_chambers=("Body", "Mind", "Heart", "Time", "Memory"),
        type_label="Wild",
        aspect="Temptation",
        stack_value=1,
        base_power=1,
        base_move=0,
        special="serpent",
        help_text="Wild: rolls for Will drain or a burst Hell push.",
    ),

    "weight": Card(
        key="weight",
        faction=DEVIL,
        emoji="🪨",
        name="Weight",
        valid_chambers=("Body", "Time"),
        type_label="Direct / Heavy",
        aspect="Weight",
        stack_value=2,
        rarity="Common",
        prices=("Heavy",),
        base_power=2,
        base_move=2,
        special="weight",
        help_text="Heavy reliable Hell pressure. Strong for a common, but costs Will or loses force.",
    ),
    "snare": Card(
        key="snare",
        faction=DEVIL,
        emoji="🕸️",
        name="Snare",
        valid_chambers=("Body", "Time"),
        type_label="Trap / Fragile",
        aspect="Trap",
        stack_value=2,
        rarity="Uncommon",
        prices=("Fragile",),
        base_power=2,
        base_move=0,
        trap=True,
        special="false_door",
        help_text="Fragile trap. Strong tempo catch if uncontested, poor if Angel fights the same Chamber.",
    ),
    "fever": Card(
        key="fever",
        faction=DEVIL,
        emoji="🔥",
        name="Fever",
        valid_chambers=("Body", "Mind"),
        type_label="Direct / Volatile",
        aspect="Fever",
        stack_value=3,
        rarity="Rare",
        prices=("Volatile",),
        base_power=2,
        base_move=2,
        special="fever",
        help_text="Volatile burst. Weak rolls lose force; surge rolls bite harder.",
    ),
    "obsession": Card(
        key="obsession",
        faction=DEVIL,
        emoji="🧿",
        name="Obsession",
        valid_chambers=("Heart", "Memory"),
        type_label="Ongoing / Binding",
        aspect="Obsession",
        stack_value=3,
        rarity="Rare",
        prices=("Binding",),
        base_power=2,
        base_move=1,
        ongoing_move=1,
        ongoing=True,
        special="obsession",
        help_text="Binding ongoing lock. Good pressure, but you cannot use that Chamber next round.",
    ),
    "futility": Card(
        key="futility",
        faction=DEVIL,
        emoji="🪦",
        name="Futility",
        valid_chambers=("Mind", "Time"),
        type_label="Direct / Delayed",
        aspect="Futility",
        stack_value=2,
        rarity="Uncommon",
        prices=("Delayed",),
        base_power=2,
        base_move=2,
        special="futility",
        help_text="Delayed despair pulse. Telegraphs pressure instead of scoring immediately.",
    ),
    "rupture": Card(
        key="rupture",
        faction=DEVIL,
        emoji="🧨",
        name="Rupture",
        valid_chambers=("Body", "Memory"),
        type_label="Direct / Hungry",
        aspect="Rupture",
        stack_value=3,
        rarity="Rare",
        prices=("Hungry",),
        base_power=2,
        base_move=2,
        special="rupture",
        help_text="Hungry burst. Feeds on Hell stack; painful if prepared, blunter if played naked.",
    ),
    "crawling_skin": Card(
        key="crawling_skin",
        faction=DEVIL,
        emoji="🐜",
        name="Crawling Skin",
        valid_chambers=("Body", "Mind"),
        type_label="Direct / Echo",
        aspect="Irritation",
        stack_value=1,
        rarity="Common",
        prices=("Echo",),
        base_power=1,
        base_move=1,
        special="crawling_skin",
        help_text="Low irritation. Echo repeats as a small -1 Hell nudge next round.",
    ),
    "shame_mirror": Card(
        key="shame_mirror",
        faction=DEVIL,
        emoji="🪞",
        name="Shame Mirror",
        valid_chambers=("Memory",),
        type_label="Direct / Narrow",
        aspect="Shame",
        stack_value=3,
        rarity="Rare",
        prices=("Narrow", "Fragile"),
        base_power=2,
        base_move=2,
        special="thorn",
        help_text="Memory-only rare. Brutal if protected, fragile if Angel contests Memory.",
    ),
}

# Rarity pass for the original v0.x glyphs. Rarity means tactical specificity,
# not simple superiority: some Stack 1 cards are Rare because they control timing,
# information, or enemy stacks better than their printed number suggests.
_ORIGINAL_RARITY_PATCH: Dict[str, Tuple[str, Tuple[str, ...]]] = {
    "candle": ("Common", ()),
    "open_hand": ("Uncommon", ()),
    "calm_breath": ("Rare", ()),
    "hymn": ("Uncommon", ()),
    "rest": ("Common", ()),
    "loved_memory": ("Rare", ()),
    "guardian": ("Rare", ()),
    "little_miracle": ("Rare", ("Volatile",)),
    "hope": ("Uncommon", ("Desperate",)),
    "bell": ("Rare", ()),
    "small_comfort": ("Common", ()),
    "sweetness": ("Uncommon", ()),
    "ache": ("Common", ()),
    "screech": ("Common", ()),
    "fly_swarm": ("Uncommon", ()),
    "void_thought": ("Uncommon", ("Desperate",)),
    "memory_thorn": ("Rare", ()),
    "burden": ("Uncommon", ()),
    "false_door": ("Rare", ()),
    "watching_eye": ("Rare", ()),
    "endless_wait": ("Rare", ()),
    "cold_room": ("Uncommon", ()),
    "tangled_desire": ("Rare", ()),
    "serpent": ("Rare", ("Volatile",)),
}
for _key, (_rarity, _prices) in _ORIGINAL_RARITY_PATCH.items():
    CARDS[_key] = replace(CARDS[_key], rarity=_rarity, prices=_prices)


DECKS: Dict[str, Dict[str, List[str]]] = {
    ANGEL: {
        "Starter Mercy": [
            "candle", "open_hand", "rest", "hymn", "lightness",
            "tea_quiet", "hope", "small_comfort", "sweetness", "loved_memory",
        ],
        "Comfort Engine": [
            "candle", "candle", "rest", "small_comfort", "sweetness",
            "lightness", "tea_quiet", "open_hand", "hymn", "guardian",
        ],
        "Sanctuary Control": [
            "guardian", "sanctuary", "calm_breath", "bell", "hymn",
            "rest", "acceptance", "tea_quiet", "hope", "lightness",
        ],
        "Hope From Behind": [
            "hope", "purpose", "grace", "open_hand", "loved_memory",
            "rest", "calm_breath", "little_miracle", "candle", "lightness",
        ],
        "Memory Loom": [
            "loved_memory", "silver_thread", "acceptance", "open_hand", "grace",
            "lightness", "hope", "small_comfort", "rest", "tea_quiet",
        ],
        "Delayed Dawn": [
            "dawn", "purpose", "hymn", "guardian", "candle",
            "rest", "lightness", "tea_quiet", "small_comfort", "sweetness",
        ],
    },
    DEVIL: {
        "Starter Torment": [
            "ache", "screech", "fly_swarm", "void_thought", "burden",
            "cold_room", "weight", "crawling_skin", "futility", "memory_thorn",
        ],
        "Pain Engine": [
            "ache", "ache", "screech", "fly_swarm", "cold_room",
            "burden", "weight", "crawling_skin", "void_thought", "serpent",
        ],
        "Despair Control": [
            "void_thought", "memory_thorn", "endless_wait", "watching_eye", "false_door",
            "snare", "futility", "burden", "crawling_skin", "screech",
        ],
        "Trickster Hell": [
            "false_door", "watching_eye", "tangled_desire", "serpent", "screech",
            "snare", "crawling_skin", "futility", "burden", "void_thought",
        ],
        "Fever Break": [
            "fever", "rupture", "weight", "ache", "crawling_skin",
            "burden", "cold_room", "fly_swarm", "void_thought", "screech",
        ],
        "Memory Shame": [
            "shame_mirror", "memory_thorn", "obsession", "watching_eye", "snare",
            "futility", "void_thought", "crawling_skin", "burden", "screech",
        ],
    },
}


# -----------------------------------------------------------------------------
# Utility functions
# -----------------------------------------------------------------------------

def opponent(faction: str) -> str:
    return DEVIL if faction == ANGEL else ANGEL


def intensity_from_d6(raw: int) -> int:
    if raw <= 2:
        return 1
    if raw <= 4:
        return 2
    return 3


def normalized_emoji(value: str) -> str:
    return value.strip().replace("\ufe0f", "")


def is_disruptive(card: Card) -> bool:
    return (
        "Disruption" in card.type_label
        or "Trap" in card.type_label
        or "Control" in card.type_label
        or card.special in {"calm", "bell", "false_door", "watching_eye", "tangle", "serpent", "acceptance", "snare"}
    )


def has_price(card: Card, price: str) -> bool:
    return price in card.prices


def price_label(card: Card) -> str:
    return ", ".join(card.prices) if card.prices else "—"


def rarity_badge(card: Card) -> str:
    return {"Common": "C", "Uncommon": "U", "Rare": "R"}.get(card.rarity, card.rarity[:1].upper())


def movement_label(card: Card) -> str:
    kind = card.movement_type if card.movement_type in MOVEMENT_TYPES else MOVEMENT_OWNERWARD
    return MOVEMENT_TYPE_LABEL[kind]


def movement_note(card: Card) -> str:
    if card.movement_type == MOVEMENT_OWNERWARD:
        return "ownerward"
    return movement_label(card)


def deck_rarity_cost(keys: Sequence[str]) -> int:
    return sum(RARITY_COST.get(CARDS[key].rarity, 1) for key in keys)


def deck_rare_count(keys: Sequence[str]) -> int:
    return sum(1 for key in keys if CARDS[key].rarity == "Rare")


def deck_legality_issues(keys: Sequence[str]) -> List[str]:
    issues: List[str] = []
    if len(keys) != DECK_SIZE:
        issues.append(f"needs exactly {DECK_SIZE} glyphs; has {len(keys)}")
    cost = deck_rarity_cost(keys)
    if cost > DECK_RARITY_BUDGET:
        issues.append(f"rarity budget {cost}/{DECK_RARITY_BUDGET} is too high")
    rares = deck_rare_count(keys)
    if rares > DECK_MAX_RARES:
        issues.append(f"rare limit {rares}/{DECK_MAX_RARES} is too high")
    return issues


def deck_legality_label(keys: Sequence[str]) -> str:
    issues = deck_legality_issues(keys)
    cost = deck_rarity_cost(keys)
    rares = deck_rare_count(keys)
    base = f"{len(keys)}/{DECK_SIZE} glyphs • budget {cost}/{DECK_RARITY_BUDGET} • rares {rares}/{DECK_MAX_RARES}"
    if issues:
        return base + " • ILLEGAL: " + "; ".join(issues)
    return base + " • legal"


# -----------------------------------------------------------------------------
# Deck state
# -----------------------------------------------------------------------------

class DeckState:
    def __init__(self, card_keys: Sequence[str], rng: random.Random):
        self.original = list(card_keys)
        self.rng = rng
        self.draw_pile: List[str] = []
        self.discard_pile: List[str] = []
        self.revealed: List[str] = []
        self.reset()

    def reset(self) -> None:
        self.draw_pile = list(self.original)
        self.discard_pile = []
        self.revealed = []
        self.rng.shuffle(self.draw_pile)

    def _refill_if_needed(self) -> None:
        if not self.draw_pile and self.discard_pile:
            self.draw_pile = list(self.discard_pile)
            self.discard_pile = []
            self.rng.shuffle(self.draw_pile)

    def reveal(self, n: int = REVEAL_SIZE) -> List[Card]:
        self.discard_unplayed_reveal()
        self.revealed = []
        for _ in range(n):
            self._refill_if_needed()
            if self.draw_pile:
                self.revealed.append(self.draw_pile.pop(0))
        return [CARDS[k] for k in self.revealed]

    def discard_revealed_card(self, key: str) -> None:
        if key in self.revealed:
            self.revealed.remove(key)
        self.discard_pile.append(key)

    def discard_unplayed_reveal(self) -> None:
        while self.revealed:
            self.discard_pile.append(self.revealed.pop(0))


# -----------------------------------------------------------------------------
# Main app
# -----------------------------------------------------------------------------

class SoulscaleApp:
    def __init__(self, root: tk.Tk):
        self.root = root
        self.root.title("Soulscale: Purgatory v1.0.1 — movement resolver")
        self.root.geometry("1280x820")
        self.root.minsize(1120, 720)

        self.rng = random.Random()
        self.soul = 0
        self.goal = 18
        self.round_no = 0
        self.strain = 0

        self.player_faction = ANGEL
        self.ai_faction = DEVIL
        self.player_deck_name = "Starter Mercy"
        self.ai_deck_name = "Starter Torment"
        self.ai_profile_name = "Balanced / Tactical"
        self.ai_profile = AI_PROFILES[self.ai_profile_name]
        self.game_mode = "ai"  # "ai" or "hotseat"
        self.secret_hotseat = False
        self.secret_hotseat_show_p2 = False
        self.opponent_deck_name = "Starter Torment"

        self.custom_decks: Dict[str, Dict[str, List[str]]] = {ANGEL: {}, DEVIL: {}}
        self.player_deck: Optional[DeckState] = None
        self.ai_deck: Optional[DeckState] = None

        self.player_will = 3
        self.ai_will = 3
        self.player_next_will_bonus = 0
        self.ai_next_will_bonus = 0
        self.player_next_will_penalty = 0
        self.ai_next_will_penalty = 0

        self.next_angel_bonus = 0
        self.next_devil_bonus = 0
        self.next_all_bonus = 0
        self.next_heaven_cap: Optional[int] = None
        self.next_soul_cap: Optional[int] = None
        self.support_bonus = {ANGEL: 0, DEVIL: 0}
        self.delayed_movements: List[Tuple[int, str]] = []
        self.next_delayed_movements: List[Tuple[int, str]] = []
        self.chamber_locks: Dict[str, set[str]] = {ANGEL: set(), DEVIL: set()}
        self.next_chamber_locks: Dict[str, set[str]] = {ANGEL: set(), DEVIL: set()}

        self.board: Dict[str, List[BoardEffect]] = {c: [] for c in CHAMBERS}
        self.conditions: Dict[str, List[str]] = {c: [] for c in CHAMBERS}
        self.stacks: Dict[str, Dict[str, List[StackToken]]] = {
            ANGEL: {c: [] for c in CHAMBERS},
            DEVIL: {c: [] for c in CHAMBERS},
        }

        self.selected_card: Optional[Card] = None
        self.selected_card_button: Optional[tk.Button] = None
        self.in_resolution = False
        self.game_over = False
        self.game_stats = GameStats()
        self.log_lines: List[str] = []
        self.last_endgame_summary = ""

        self.font_normal = ("Segoe UI", 10)
        self.font_big = ("Segoe UI Emoji", 18)
        self.font_card = ("Segoe UI Emoji", 12)
        self.font_title = ("Segoe UI", 18, "bold")
        self.font_mono = ("Consolas", 10)

        self.start_frame: Optional[tk.Frame] = None
        self.game_frame: Optional[tk.Frame] = None

        self.status_var = tk.StringVar()
        self.selected_var = tk.StringVar(value="Select a glyph, then click a Chamber.")
        self.soul_var = tk.StringVar()
        self.will_var = tk.StringVar()
        self.strain_var = tk.StringVar()
        self.ai_reveal_var = tk.StringVar()
        self.chamber_text_vars: Dict[str, tk.StringVar] = {}
        self.card_buttons: List[tk.Button] = []
        self.chamber_buttons: List[tk.Button] = []
        self.ai_committed_play: Optional[Tuple[Card, str, ModeChoice]] = None

        self.build_start_screen()

    # ------------------------------------------------------------------
    # Setup / deck loading
    # ------------------------------------------------------------------

    def build_start_screen(self) -> None:
        if self.game_frame is not None:
            self.game_frame.destroy()
            self.game_frame = None
        if self.start_frame is not None:
            self.start_frame.destroy()
            self.start_frame = None

        # v0.9 secret hot-seat:
        # The setup screen can be taller than a laptop/desktop viewport once the deck
        # editor controls and deck preview are present. The action buttons now live in
        # a pinned footer, while the setup body scrolls vertically. This keeps Start
        # Game / Deck Editor / Export Test reachable even when Windows taskbar or DPI
        # scaling steals vertical space.
        self.start_frame = tk.Frame(self.root)
        self.start_frame.pack(fill="both", expand=True)

        footer = tk.Frame(self.start_frame, bd=1, relief="groove", padx=12, pady=8)
        footer.pack(side="bottom", fill="x")

        scroll_host = tk.Frame(self.start_frame)
        scroll_host.pack(side="top", fill="both", expand=True)

        canvas = tk.Canvas(scroll_host, borderwidth=0, highlightthickness=0)
        scrollbar = tk.Scrollbar(scroll_host, orient="vertical", command=canvas.yview)
        canvas.configure(yscrollcommand=scrollbar.set)
        canvas.pack(side="left", fill="both", expand=True)
        scrollbar.pack(side="right", fill="y")

        content = tk.Frame(canvas, padx=24, pady=18)
        content_window = canvas.create_window((0, 0), window=content, anchor="nw")

        def sync_scroll_region(_event=None) -> None:
            canvas.configure(scrollregion=canvas.bbox("all"))

        def sync_canvas_width(event) -> None:
            # Stretch the content frame to the current viewport width so text widgets
            # and grouped panels use the available space instead of hiding controls
            # below the visible area.
            canvas.itemconfigure(content_window, width=event.width)
            sync_scroll_region()

        def mousewheel(event) -> None:
            try:
                if getattr(event, "num", None) == 4:
                    canvas.yview_scroll(-3, "units")
                elif getattr(event, "num", None) == 5:
                    canvas.yview_scroll(3, "units")
                else:
                    delta = int(-1 * (event.delta / 120)) if event.delta else 0
                    if delta:
                        canvas.yview_scroll(delta * 3, "units")
            except tk.TclError:
                # The canvas may have been destroyed while changing screens.
                pass

        def bind_wheel(_event=None) -> None:
            self.root.bind_all("<MouseWheel>", mousewheel)
            self.root.bind_all("<Button-4>", mousewheel)
            self.root.bind_all("<Button-5>", mousewheel)

        def unbind_wheel(_event=None) -> None:
            self.root.unbind_all("<MouseWheel>")
            self.root.unbind_all("<Button-4>")
            self.root.unbind_all("<Button-5>")

        content.bind("<Configure>", sync_scroll_region)
        canvas.bind("<Configure>", sync_canvas_width)
        canvas.bind("<Enter>", bind_wheel)
        content.bind("<Enter>", bind_wheel)
        canvas.bind("<Leave>", unbind_wheel)
        content.bind("<Leave>", unbind_wheel)

        start_buttons = tk.Frame(footer)
        start_buttons.pack(anchor="center")

        tk.Button(
            start_buttons,
            text="Start Game",
            command=self.start_game,
            font=("Segoe UI", 14, "bold"),
            padx=26,
            pady=8,
        ).pack(side="left", padx=8)

        tk.Button(
            start_buttons,
            text="Build/Edit Player 1 Deck",
            command=lambda: self.open_deck_editor(self.side_var.get(), target="player"),
            font=("Segoe UI", 11, "bold"),
            padx=18,
            pady=8,
        ).pack(side="left", padx=8)

        tk.Button(
            start_buttons,
            text="Test Real Endgame Export Window",
            command=self.force_endgame_window_test,
            font=("Segoe UI", 11, "bold"),
            padx=18,
            pady=8,
        ).pack(side="left", padx=8)

        tk.Label(
            footer,
            text="v1.0.1: rarity/price cards plus rules-aware movement resolver for future reverse-cost mechanics.",
            font=("Segoe UI", 9, "bold"),
            fg="#7a2e00",
        ).pack(pady=(6, 0))

        tk.Label(
            content,
            text="Soulscale: Purgatory",
            font=("Segoe UI", 28, "bold"),
        ).pack(pady=(6, 2))

        tk.Label(
            content,
            text="A compact Heaven-vs-Hell loadout battler with Resonance stacks, AI opponents, and hot-seat playtesting.",
            font=("Segoe UI", 13),
        ).pack(pady=(0, 16))

        self.game_mode_var = tk.StringVar(value=self.game_mode)
        self.secret_hotseat_var = tk.BooleanVar(value=self.secret_hotseat)
        self.side_var = tk.StringVar(value=self.player_faction)
        self.deck_var = tk.StringVar(value=self.player_deck_name if self.player_deck_name in self.available_decks(self.player_faction) else self.default_deck_name(self.player_faction))
        opp_faction = opponent(self.player_faction)
        self.opponent_deck_var = tk.StringVar(value=self.opponent_deck_name if self.opponent_deck_name in self.available_decks(opp_faction) else self.default_deck_name(opp_faction))
        self.ai_profile_var = tk.StringVar(value=self.ai_profile_name)
        self.goal_var = tk.IntVar(value=self.goal)

        panel = tk.Frame(content)
        panel.pack(pady=8, fill="x")

        mode_box = tk.LabelFrame(panel, text="1) Pick play mode", padx=14, pady=10)
        mode_box.grid(row=0, column=0, padx=8, sticky="n")
        tk.Radiobutton(
            mode_box,
            text="Human vs Computer",
            value="ai",
            variable=self.game_mode_var,
            command=self.refresh_deck_options,
            font=self.font_normal,
        ).pack(anchor="w", pady=4)
        tk.Radiobutton(
            mode_box,
            text="Human vs Human / hot-seat",
            value="hotseat",
            variable=self.game_mode_var,
            command=self.refresh_deck_options,
            font=self.font_normal,
        ).pack(anchor="w", pady=4)
        self.secret_hotseat_check = tk.Checkbutton(
            mode_box,
            text="Secret hot-seat choices",
            variable=self.secret_hotseat_var,
            command=self.refresh_deck_options,
            font=("Segoe UI", 9, "bold"),
        )
        self.secret_hotseat_check.pack(anchor="w", pady=(6, 0))
        self.mode_note_label = tk.Label(mode_box, justify="left", wraplength=230, font=("Segoe UI", 9))
        self.mode_note_label.pack(anchor="w", pady=(10, 0))

        side_box = tk.LabelFrame(panel, text="2) Pick Player 1 side", padx=14, pady=10)
        side_box.grid(row=0, column=1, padx=8, sticky="n")

        tk.Radiobutton(
            side_box,
            text="Heaven / Angel  🪽",
            value=ANGEL,
            variable=self.side_var,
            command=self.refresh_deck_options,
            font=self.font_normal,
        ).pack(anchor="w", pady=4)
        tk.Radiobutton(
            side_box,
            text="Hell / Devil  🐍",
            value=DEVIL,
            variable=self.side_var,
            command=self.refresh_deck_options,
            font=self.font_normal,
        ).pack(anchor="w", pady=4)

        tk.Button(
            side_box,
            text="Load Player 1 custom .txt deck",
            command=lambda: self.load_custom_deck(self.side_var.get()),
            font=self.font_normal,
        ).pack(anchor="w", pady=(14, 4), fill="x")

        tk.Button(
            side_box,
            text="Build/Edit Player 1 deck",
            command=lambda: self.open_deck_editor(self.side_var.get(), target="player"),
            font=self.font_normal,
        ).pack(anchor="w", pady=(2, 4), fill="x")

        tk.Label(
            side_box,
            text=f"Custom deck format:\none emoji per line, exactly {DECK_SIZE} lines.",
            justify="left",
            font=("Segoe UI", 9),
        ).pack(anchor="w", pady=(2, 0))

        deck_box = tk.LabelFrame(panel, text="3) Player 1 deck", padx=14, pady=10)
        deck_box.grid(row=0, column=2, padx=8, sticky="n")
        self.deck_menu = tk.OptionMenu(deck_box, self.deck_var, *self.available_decks(self.side_var.get()).keys())
        self.deck_menu.config(width=25, font=self.font_normal)
        self.deck_menu.pack(anchor="w", pady=4)

        self.opponent_deck_box = tk.LabelFrame(panel, text="4) Opponent deck", padx=14, pady=10)
        self.opponent_deck_box.grid(row=0, column=3, padx=8, sticky="n")
        self.opponent_deck_menu = tk.OptionMenu(
            self.opponent_deck_box,
            self.opponent_deck_var,
            *self.available_decks(opponent(self.side_var.get())).keys(),
        )
        self.opponent_deck_menu.config(width=25, font=self.font_normal)
        self.opponent_deck_menu.pack(anchor="w", pady=4)
        tk.Button(
            self.opponent_deck_box,
            text="Load opponent custom .txt deck",
            command=lambda: self.load_custom_deck(opponent(self.side_var.get())),
            font=self.font_normal,
        ).pack(anchor="w", pady=(10, 4), fill="x")

        tk.Button(
            self.opponent_deck_box,
            text="Build/Edit opponent deck",
            command=lambda: self.open_deck_editor(opponent(self.side_var.get()), target="opponent"),
            font=self.font_normal,
        ).pack(anchor="w", pady=(2, 4), fill="x")

        self.ai_box = tk.LabelFrame(panel, text="5) Computer personality", padx=14, pady=10)
        self.ai_box.grid(row=0, column=4, padx=8, sticky="n")
        self.ai_profile_menu = tk.OptionMenu(self.ai_box, self.ai_profile_var, *AI_PROFILES.keys())
        self.ai_profile_menu.config(width=25, font=self.font_normal)
        self.ai_profile_menu.pack(anchor="w", pady=4)

        self.ai_profile_preview = tk.Label(self.ai_box, justify="left", wraplength=245, font=self.font_normal)
        self.ai_profile_preview.pack(anchor="w", pady=(10, 0))

        goal_box = tk.LabelFrame(panel, text="6) Game length", padx=14, pady=10)
        goal_box.grid(row=1, column=0, columnspan=5, padx=8, pady=(12, 0), sticky="ew")
        for label, value in (("Quick ±12", 12), ("Standard ±18", 18), ("Deep ±24", 24)):
            tk.Radiobutton(
                goal_box,
                text=label,
                value=value,
                variable=self.goal_var,
                font=self.font_normal,
            ).pack(side="left", padx=12)
        tk.Label(
            goal_box,
            text="Standard is recommended for Resonance stacking; Quick is swingy, Deep is more tactical.",
            font=("Segoe UI", 9),
        ).pack(side="left", padx=16)

        self.deck_preview = tk.Text(
            content,
            width=118,
            height=12,
            wrap="word",
            font=("Segoe UI Emoji", 10),
        )
        self.deck_preview.pack(pady=(14, 8), padx=20, fill="both", expand=False)
        self.deck_preview.config(state="disabled")

        self.deck_var.trace_add("write", lambda *_: self.update_deck_preview())
        self.opponent_deck_var.trace_add("write", lambda *_: self.update_deck_preview())
        self.ai_profile_var.trace_add("write", lambda *_: self.update_ai_profile_preview())
        self.game_mode_var.trace_add("write", lambda *_: self.update_ai_profile_preview())
        self.secret_hotseat_var.trace_add("write", lambda *_: self.refresh_deck_options())
        self.update_deck_preview()
        self.update_ai_profile_preview()
        self.refresh_deck_options()

        rules = tk.LabelFrame(content, text="v1.0.1 rules bite", padx=14, pady=10)
        rules.pack(fill="x", padx=24, pady=10)

        rules_text = (
            "Each round both sides reveal 4 glyphs. In solo mode, the AI secretly commits before Player 1 chooses. "
            "In hot-seat mode, Player 1 chooses first, then pass the keyboard to Player 2.\n"
            "Press = score from stack. Anchor = no immediate Soul movement but stronger stack growth. "
            "Release = spend stack for a capped dice-amplified burst. Sever = cut enemy stack. "
            "Choose Quick ±12, Standard ±18, or Deep ±24."
        )
        tk.Label(rules, text=rules_text, justify="left", font=self.font_normal, wraplength=1180).pack(anchor="w")

    def available_decks(self, faction: str) -> Dict[str, List[str]]:
        decks = dict(DECKS[faction])
        decks.update(self.custom_decks[faction])
        return decks

    def default_deck_name(self, faction: str) -> str:
        return next(iter(DECKS[faction].keys()))

    def refresh_deck_options(self) -> None:
        if not hasattr(self, "side_var"):
            return
        p1_faction = self.side_var.get()
        p2_faction = opponent(p1_faction)
        p1_decks = self.available_decks(p1_faction)
        p2_decks = self.available_decks(p2_faction)

        if self.deck_var.get() not in p1_decks:
            self.deck_var.set(next(iter(p1_decks.keys())))
        if self.opponent_deck_var.get() not in p2_decks:
            self.opponent_deck_var.set(next(iter(p2_decks.keys())))

        self.update_option_menu(self.deck_menu, self.deck_var, p1_decks.keys())
        self.update_option_menu(self.opponent_deck_menu, self.opponent_deck_var, p2_decks.keys())

        mode = self.game_mode_var.get() if hasattr(self, "game_mode_var") else self.game_mode
        if hasattr(self, "opponent_deck_box"):
            who = "Player 2" if mode == "hotseat" else "Computer"
            self.opponent_deck_box.config(text=f"4) {who} deck ({FACTION_SHORT[p2_faction]})")
        if hasattr(self, "secret_hotseat_check"):
            self.secret_hotseat_check.config(state=("normal" if mode == "hotseat" else "disabled"))
        if hasattr(self, "mode_note_label"):
            if mode == "hotseat":
                if getattr(self, "secret_hotseat_var", None) is not None and self.secret_hotseat_var.get():
                    self.mode_note_label.config(
                        text="Secret hot-seat: Player 2's hand is hidden from Player 1; Player 1's choice is masked before Player 2 chooses."
                    )
                else:
                    self.mode_note_label.config(text="Open hot-seat: two humans share the same window; both dealt hands remain visible.")
            else:
                self.mode_note_label.config(text="Solo play against the selected computer personality.")
        self.update_deck_preview()
        self.update_ai_profile_preview()

    def update_option_menu(self, menu_widget: tk.OptionMenu, variable: tk.StringVar, names: Iterable[str]) -> None:
        menu = menu_widget["menu"]
        menu.delete(0, "end")
        for name in names:
            menu.add_command(label=name, command=lambda n=name: variable.set(n))

    def update_ai_profile_preview(self) -> None:
        if not hasattr(self, "ai_profile_preview"):
            return
        mode = self.game_mode_var.get() if hasattr(self, "game_mode_var") else self.game_mode
        if mode == "hotseat":
            secret = getattr(self, "secret_hotseat_var", None) is not None and self.secret_hotseat_var.get()
            text = "Ignored in hot-seat mode. Player 2 makes the opponent choices."
            if secret:
                text += "\n\nSecret mode hides hands/choices until both players have committed."
            if hasattr(self, "ai_profile_menu"):
                self.ai_profile_menu.config(state="disabled")
            self.ai_profile_preview.config(text=text)
            return
        if hasattr(self, "ai_profile_menu"):
            self.ai_profile_menu.config(state="normal")
        profile = AI_PROFILES[self.ai_profile_var.get()]
        text = (
            f"{profile.description}\n\n"
            f"Aggression: {profile.aggression:.2f}\n"
            f"Planning: {profile.planning:.2f}\n"
            f"Disruption: {profile.disruption:.2f}\n"
            f"Randomness: {profile.randomness:.2f}"
        )
        self.ai_profile_preview.config(text=text)

    def update_deck_preview(self) -> None:
        if not hasattr(self, "deck_preview"):
            return
        p1_faction = self.side_var.get()
        p2_faction = opponent(p1_faction)
        p1_name = self.deck_var.get()
        p2_name = self.opponent_deck_var.get() if hasattr(self, "opponent_deck_var") else self.default_deck_name(p2_faction)
        sections = [
            ("PLAYER 1", p1_faction, p1_name, self.available_decks(p1_faction).get(p1_name, [])),
            ("OPPONENT", p2_faction, p2_name, self.available_decks(p2_faction).get(p2_name, [])),
        ]
        lines: List[str] = []
        for title, faction, deck_name, keys in sections:
            lines.append(f"{title}: {FACTION_LABEL[faction]} — {deck_name}")
            lines.append(deck_legality_label(keys))
            lines.append("-" * 78)
            for key in keys:
                c = CARDS[key]
                chambers = "/".join(f"{CHAMBER_EMOJI[ch]} {ch}" for ch in c.valid_chambers)
                lines.append(
                    f"{c.emoji} {c.name} — {c.rarity} [{rarity_badge(c)}] — {c.type_label} — {c.aspect} — Stack {c.stack_value}\n"
                    f"   Price: {price_label(c)}\n"
                    f"   {chambers}\n"
                    f"   {c.help_text}\n"
                )
            lines.append("")
        self.deck_preview.config(state="normal")
        self.deck_preview.delete("1.0", "end")
        self.deck_preview.insert("1.0", "\n".join(lines))
        self.deck_preview.config(state="disabled")

    def load_custom_deck(self, faction: Optional[str] = None) -> None:
        faction = faction or self.side_var.get()
        path = filedialog.askopenfilename(
            title=f"Load {FACTION_SHORT[faction]} custom deck",
            filetypes=[("Text decks", "*.txt"), ("All files", "*.*")],
        )
        if not path:
            return

        try:
            keys = self.parse_custom_deck(Path(path), faction)
        except ValueError as exc:
            messagebox.showerror("Invalid custom deck", str(exc))
            return
        except OSError as exc:
            messagebox.showerror("Could not read deck", f"Could not read file:\n{exc}")
            return

        base_name = Path(path).stem.strip() or "Custom Deck"
        deck_name = f"Custom: {base_name}"
        existing = self.available_decks(faction)
        if deck_name in existing:
            suffix = 2
            while f"{deck_name} ({suffix})" in existing:
                suffix += 1
            deck_name = f"{deck_name} ({suffix})"

        self.custom_decks[faction][deck_name] = keys
        self.refresh_deck_options()
        if faction == self.side_var.get():
            self.deck_var.set(deck_name)
        else:
            self.opponent_deck_var.set(deck_name)
        messagebox.showinfo(
            "Custom deck loaded",
            f"Loaded {deck_name}\n\n{self.deck_symbols(keys)}",
        )

    def parse_custom_deck(self, path: Path, faction: str) -> List[str]:
        emoji_to_key: Dict[str, str] = {}
        for key, card in CARDS.items():
            if card.faction == faction:
                emoji_to_key[card.emoji] = key
                emoji_to_key[normalized_emoji(card.emoji)] = key

        raw_lines = path.read_text(encoding="utf-8-sig").splitlines()
        usable: List[Tuple[int, str]] = []
        for idx, raw in enumerate(raw_lines, start=1):
            stripped = raw.strip()
            if not stripped or stripped.startswith("#"):
                continue
            usable.append((idx, stripped))

        if len(usable) not in {DECK_SIZE, LEGACY_DECK_SIZE}:
            raise ValueError(
                f"Deck must contain exactly {DECK_SIZE} non-empty emoji lines for {FACTION_LABEL[faction]}.\n"
                f"Legacy {LEGACY_DECK_SIZE}-glyph decks are accepted and auto-padded with stable commons.\n"
                f"This file contains {len(usable)} usable lines.\n\n"
                f"Blank lines and lines starting with # are ignored."
            )

        keys: List[str] = []
        invalid: List[str] = []
        for line_no, text in usable:
            key = emoji_to_key.get(text) or emoji_to_key.get(normalized_emoji(text))
            if key is None:
                invalid.append(f"line {line_no}: {text!r}")
            else:
                keys.append(key)

        if invalid:
            allowed = " ".join(card.emoji for card in CARDS.values() if card.faction == faction)
            raise ValueError(
                "The deck contains glyphs that do not belong to the selected side, "
                "or are not known cards.\n\n"
                + "\n".join(invalid)
                + f"\n\nAllowed {FACTION_SHORT[faction]} glyphs:\n{allowed}"
            )

        if len(keys) == LEGACY_DECK_SIZE:
            pad_pool = [k for k, c in CARDS.items() if c.faction == faction and c.rarity == "Common"]
            while len(keys) < DECK_SIZE:
                keys.append(pad_pool[(len(keys) - LEGACY_DECK_SIZE) % len(pad_pool)])

        issues = deck_legality_issues(keys)
        if issues:
            raise ValueError(
                "The deck glyphs are recognized, but the deck is not legal under v1.0.1 rarity rules.\n\n"
                + "\n".join(f"- {issue}" for issue in issues)
                + f"\n\nRule: {DECK_SIZE} glyphs, rarity budget ≤ {DECK_RARITY_BUDGET}, rares ≤ {DECK_MAX_RARES}."
            )
        return keys

    def deck_symbols(self, keys: Sequence[str]) -> str:
        return " ".join(CARDS[k].emoji for k in keys)

    def register_editor_deck(self, faction: str, deck_name: str, keys: Sequence[str], target: str) -> None:
        """Register a deck created in the visual editor and select it in the setup UI."""
        self.custom_decks[faction][deck_name] = list(keys)
        self.refresh_deck_options()
        if target == "player":
            self.deck_var.set(deck_name)
        else:
            self.opponent_deck_var.set(deck_name)
        self.update_deck_preview()

    def open_deck_editor(self, faction: Optional[str] = None, target: str = "player") -> None:
        """Open the visual click-to-build deck editor.

        The editor writes the same simple custom deck format already accepted by the
        loader: exactly ten non-empty emoji lines, one glyph per line.
        """
        if not hasattr(self, "side_var"):
            messagebox.showinfo("Deck editor", "Deck editing is available from the setup screen.")
            return

        faction = faction or (self.side_var.get() if target == "player" else opponent(self.side_var.get()))
        target_label = "Player 1" if target == "player" else ("Player 2" if self.game_mode_var.get() == "hotseat" else "Opponent")
        faction_label = FACTION_LABEL[faction]
        current_var = self.deck_var if target == "player" else self.opponent_deck_var
        current_name = current_var.get()
        current_keys = list(self.available_decks(faction).get(current_name, []))
        deck_keys: List[str] = list(current_keys[:DECK_SIZE])

        win = tk.Toplevel(self.root)
        win.title(f"Deck Editor — {target_label} — {faction_label} — v1.0.1")
        win.geometry("1180x800")
        win.minsize(1040, 660)
        win.transient(self.root)

        header = tk.Frame(win, padx=14, pady=10)
        header.pack(fill="x")
        tk.Label(
            header,
            text=f"Deck Editor: {target_label} — {faction_label}",
            font=("Segoe UI", 18, "bold"),
        ).pack(anchor="w")
        tk.Label(
            header,
            text=f"Click glyphs to add them. Build exactly {DECK_SIZE}; budget ≤ {DECK_RARITY_BUDGET}; rares ≤ {DECK_MAX_RARES}. Double-click a deck slot to remove.",
            font=self.font_normal,
        ).pack(anchor="w", pady=(4, 0))

        body = tk.Frame(win, padx=12, pady=8)
        body.pack(fill="both", expand=True)

        library_box = tk.LabelFrame(body, text=f"Available {FACTION_SHORT[faction]} glyphs — click to add", padx=10, pady=10)
        library_box.pack(side="left", fill="both", expand=True, padx=(0, 8))

        deck_box = tk.LabelFrame(body, text="Current editor deck", padx=10, pady=10)
        deck_box.pack(side="right", fill="both", expand=False, padx=(8, 0))

        count_var = tk.StringVar()
        status_var = tk.StringVar()
        composition_var = tk.StringVar()

        card_keys = [key for key, card in CARDS.items() if card.faction == faction]
        cards_grid = tk.Frame(library_box)
        cards_grid.pack(fill="both", expand=True)

        def add_card(key: str) -> None:
            if len(deck_keys) >= DECK_SIZE:
                status_var.set(f"Deck already has {DECK_SIZE}/{DECK_SIZE} cards. Remove one before adding another.")
                return
            deck_keys.append(key)
            refresh_editor()

        for idx, key in enumerate(card_keys):
            card = CARDS[key]
            chambers = " / ".join(CHAMBER_EMOJI[ch] for ch in card.valid_chambers)
            label = (
                f"{card.emoji}  {card.name}   [{rarity_badge(card)} {card.rarity}]\n"
                f"{card.type_label} • {card.aspect} • stack {card.stack_value}\n"
                f"Price: {price_label(card)}\n"
                f"{chambers}\n"
                f"{card.help_text}"
            )
            btn = tk.Button(
                cards_grid,
                text=label,
                justify="left",
                anchor="w",
                wraplength=310,
                font=("Segoe UI Emoji", 10),
                command=lambda k=key: add_card(k),
                padx=8,
                pady=6,
            )
            btn.grid(row=idx // 2, column=idx % 2, sticky="nsew", padx=5, pady=5)
        cards_grid.columnconfigure(0, weight=1)
        cards_grid.columnconfigure(1, weight=1)
        for row in range((len(card_keys) + 1) // 2):
            cards_grid.rowconfigure(row, weight=1)

        tk.Label(deck_box, textvariable=count_var, font=("Segoe UI", 12, "bold")).pack(anchor="w")
        tk.Label(deck_box, textvariable=status_var, font=("Segoe UI", 9), wraplength=330, justify="left").pack(anchor="w", pady=(2, 8))

        list_frame = tk.Frame(deck_box)
        list_frame.pack(fill="both", expand=True)
        deck_list = tk.Listbox(
            list_frame,
            width=42,
            height=14,
            font=("Segoe UI Emoji", 11),
            activestyle="dotbox",
            exportselection=False,
        )
        deck_scroll = tk.Scrollbar(list_frame, orient="vertical", command=deck_list.yview)
        deck_list.config(yscrollcommand=deck_scroll.set)
        deck_list.pack(side="left", fill="both", expand=True)
        deck_scroll.pack(side="right", fill="y")

        def selected_index() -> Optional[int]:
            selection = deck_list.curselection()
            if not selection:
                return None
            return int(selection[0])

        def remove_selected(_event: object = None) -> None:
            idx = selected_index()
            if idx is None:
                status_var.set("Select a deck slot to remove, or double-click a slot.")
                return
            removed = deck_keys.pop(idx)
            refresh_editor()
            if deck_keys:
                deck_list.selection_set(min(idx, len(deck_keys) - 1))
            status_var.set(f"Removed {CARDS[removed].short}.")

        def move_selected(delta: int) -> None:
            idx = selected_index()
            if idx is None:
                status_var.set("Select a deck slot to move.")
                return
            new_idx = idx + delta
            if new_idx < 0 or new_idx >= len(deck_keys):
                return
            deck_keys[idx], deck_keys[new_idx] = deck_keys[new_idx], deck_keys[idx]
            refresh_editor()
            deck_list.selection_set(new_idx)

        def clear_deck() -> None:
            if not deck_keys:
                return
            if messagebox.askyesno("Clear editor deck", "Remove all glyphs from the editor deck?", parent=win):
                deck_keys.clear()
                refresh_editor()

        def restore_selected_deck() -> None:
            source_keys = list(self.available_decks(faction).get(current_var.get(), []))
            deck_keys.clear()
            deck_keys.extend(source_keys[:DECK_SIZE])
            refresh_editor()
            status_var.set(f"Loaded currently selected setup deck: {current_var.get()}.")

        deck_list.bind("<Double-Button-1>", remove_selected)
        deck_list.bind("<Delete>", remove_selected)

        controls = tk.Frame(deck_box)
        controls.pack(fill="x", pady=8)
        tk.Button(controls, text="Remove", command=remove_selected).grid(row=0, column=0, sticky="ew", padx=3, pady=3)
        tk.Button(controls, text="Move up", command=lambda: move_selected(-1)).grid(row=0, column=1, sticky="ew", padx=3, pady=3)
        tk.Button(controls, text="Move down", command=lambda: move_selected(1)).grid(row=0, column=2, sticky="ew", padx=3, pady=3)
        tk.Button(controls, text="Clear", command=clear_deck).grid(row=1, column=0, sticky="ew", padx=3, pady=3)
        tk.Button(controls, text="Reload selected", command=restore_selected_deck).grid(row=1, column=1, columnspan=2, sticky="ew", padx=3, pady=3)
        for col in range(3):
            controls.columnconfigure(col, weight=1)

        composition = tk.LabelFrame(deck_box, text="Deck readout", padx=8, pady=6)
        composition.pack(fill="x", pady=(4, 8))
        tk.Label(composition, textvariable=composition_var, justify="left", anchor="w", font=("Segoe UI Emoji", 9), wraplength=340).pack(fill="x")

        save_buttons = tk.Frame(deck_box)
        save_buttons.pack(fill="x", pady=(4, 0))

        save_button: Optional[tk.Button] = None
        use_button: Optional[tk.Button] = None

        def make_deck_text() -> str:
            return "\n".join(CARDS[key].emoji for key in deck_keys) + "\n"

        def validate_ready(show_dialog: bool = True) -> bool:
            issues = deck_legality_issues(deck_keys)
            if issues:
                if show_dialog:
                    messagebox.showerror(
                        "Deck is not legal yet",
                        "This deck breaks v1.0.1 deckbuilding rules:\n\n"
                        + "\n".join(f"- {issue}" for issue in issues)
                        + f"\n\nRule: {DECK_SIZE} glyphs, rarity budget ≤ {DECK_RARITY_BUDGET}, rares ≤ {DECK_MAX_RARES}.",
                        parent=win,
                    )
                return False
            return True

        def default_filename() -> str:
            safe_side = FACTION_SHORT[faction].lower().replace(" ", "_")
            stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            return f"soulscale_{safe_side}_deck_{stamp}.txt"

        def save_deck_file(select_after_save: bool = True) -> None:
            if not validate_ready(True):
                return
            path = filedialog.asksaveasfilename(
                parent=win,
                title="Save Soulscale custom deck",
                defaultextension=".txt",
                initialfile=default_filename(),
                filetypes=[("Text decks", "*.txt"), ("All files", "*.*")],
            )
            if not path:
                return
            try:
                Path(path).write_text(make_deck_text(), encoding="utf-8")
            except OSError as exc:
                messagebox.showerror("Could not save deck", f"Could not write deck file:\n{exc}", parent=win)
                return

            deck_name = f"Custom: {Path(path).stem.strip() or 'Editor Deck'}"
            if select_after_save:
                self.register_editor_deck(faction, deck_name, deck_keys, target)
                status_var.set(f"Saved and selected {deck_name} for {target_label}.")
                messagebox.showinfo(
                    "Deck saved and selected",
                    f"Saved:\n{path}\n\nSelected as {target_label} deck:\n{deck_name}\n\n{self.deck_symbols(deck_keys)}",
                    parent=win,
                )
            else:
                status_var.set(f"Saved deck file: {path}")
                messagebox.showinfo("Deck saved", f"Saved:\n{path}", parent=win)

        def use_without_saving() -> None:
            if not validate_ready(True):
                return
            stamp = datetime.now().strftime("%H%M%S")
            deck_name = f"Editor: {FACTION_SHORT[faction]} {stamp}"
            self.register_editor_deck(faction, deck_name, deck_keys, target)
            status_var.set(f"Selected temporary editor deck: {deck_name}.")
            messagebox.showinfo(
                "Deck selected",
                f"Selected temporary deck for {target_label}:\n{deck_name}\n\nUse Save & Select if you also want a .txt file on disk.",
                parent=win,
            )

        save_button = tk.Button(
            save_buttons,
            text="Save .txt + select deck",
            command=lambda: save_deck_file(True),
            font=("Segoe UI", 10, "bold"),
        )
        save_button.pack(fill="x", pady=3)
        tk.Button(
            save_buttons,
            text="Save .txt only",
            command=lambda: save_deck_file(False),
        ).pack(fill="x", pady=3)
        use_button = tk.Button(
            save_buttons,
            text="Use without saving",
            command=use_without_saving,
        )
        use_button.pack(fill="x", pady=3)
        tk.Button(save_buttons, text="Close editor", command=win.destroy).pack(fill="x", pady=(10, 3))

        def refresh_editor() -> None:
            deck_list.delete(0, "end")
            for idx, key in enumerate(deck_keys, start=1):
                card = CARDS[key]
                deck_list.insert(
                    "end",
                    f"{idx:02d}. {card.emoji} {card.name} [{rarity_badge(card)}] — {card.type_label} — {card.aspect} • stack {card.stack_value} • {price_label(card)}",
                )

            issues = deck_legality_issues(deck_keys)
            legal = not issues
            count_var.set(deck_legality_label(deck_keys))
            if legal:
                status_var.set("Legal v1.0.1 deck. You can save it as a .txt and select it for play.")
            elif len(deck_keys) < DECK_SIZE:
                status_var.set(f"Needs {DECK_SIZE - len(deck_keys)} more glyph(s).")
            else:
                status_var.set("Not legal: " + "; ".join(issues))

            if save_button is not None:
                state = "normal" if legal else "disabled"
                save_button.config(state=state)
            if use_button is not None:
                use_button.config(state="normal" if legal else "disabled")

            chamber_counts = {ch: 0 for ch in CHAMBERS}
            aspect_counts: Dict[str, int] = {}
            rarity_counts: Dict[str, int] = {"Common": 0, "Uncommon": 0, "Rare": 0}
            price_counts: Dict[str, int] = {}
            direct_count = 0
            ongoing_count = 0
            disrupt_count = 0
            wild_count = 0
            stack_total = 0
            for key in deck_keys:
                card = CARDS[key]
                stack_total += card.stack_value
                rarity_counts[card.rarity] = rarity_counts.get(card.rarity, 0) + 1
                for price in card.prices:
                    price_counts[price] = price_counts.get(price, 0) + 1
                if card.base_move > 0 or card.special in {"miracle", "serpent"}:
                    direct_count += 1
                if card.ongoing:
                    ongoing_count += 1
                if is_disruptive(card):
                    disrupt_count += 1
                if "Wild" in card.type_label:
                    wild_count += 1
                aspect_counts[card.aspect] = aspect_counts.get(card.aspect, 0) + 1
                for chamber in card.valid_chambers:
                    chamber_counts[chamber] += 1

            chamber_line = "  ".join(f"{CHAMBER_EMOJI[ch]} {ch}: {chamber_counts[ch]}" for ch in CHAMBERS)
            aspect_line = ", ".join(f"{name}×{count}" for name, count in sorted(aspect_counts.items())) or "—"
            rarity_line = ", ".join(f"{name}×{count}" for name, count in rarity_counts.items() if count) or "—"
            price_line = ", ".join(f"{name}×{count}" for name, count in sorted(price_counts.items())) or "—"
            composition_var.set(
                f"Direct/scoring: {direct_count}   Ongoing: {ongoing_count}   Disruption/control: {disrupt_count}   Wild: {wild_count}\n"
                f"Rarity: {rarity_line}   Cost: {deck_rarity_cost(deck_keys)}/{DECK_RARITY_BUDGET}   Rares: {deck_rare_count(deck_keys)}/{DECK_MAX_RARES}\n"
                f"Prices: {price_line}\n"
                f"Total stack value: {stack_total}\n"
                f"Chamber coverage:\n{chamber_line}\n"
                f"Aspects:\n{aspect_line}"
            )

        refresh_editor()

    # ------------------------------------------------------------------
    # Game lifecycle / layout
    # ------------------------------------------------------------------

    def start_game(self) -> None:
        self.game_mode = self.game_mode_var.get()
        self.secret_hotseat = bool(self.secret_hotseat_var.get()) if hasattr(self, "secret_hotseat_var") else False
        self.secret_hotseat_show_p2 = False
        self.player_faction = self.side_var.get()
        self.ai_faction = opponent(self.player_faction)
        self.player_deck_name = self.deck_var.get()
        self.opponent_deck_name = self.opponent_deck_var.get()
        self.ai_profile_name = self.ai_profile_var.get()
        self.ai_profile = AI_PROFILES[self.ai_profile_name]
        self.goal = int(self.goal_var.get()) if hasattr(self, "goal_var") else self.goal

        player_decks = self.available_decks(self.player_faction)
        opponent_decks = self.available_decks(self.ai_faction)
        if self.player_deck_name not in player_decks:
            self.player_deck_name = self.default_deck_name(self.player_faction)
        if self.opponent_deck_name not in opponent_decks:
            self.opponent_deck_name = self.choose_ai_deck_name(self.ai_faction)
        self.ai_deck_name = self.opponent_deck_name

        self.soul = 0
        self.round_no = 0
        self.strain = 0
        self.player_next_will_bonus = 0
        self.ai_next_will_bonus = 0
        self.player_next_will_penalty = 0
        self.ai_next_will_penalty = 0
        self.next_angel_bonus = 0
        self.next_devil_bonus = 0
        self.next_all_bonus = 0
        self.next_heaven_cap = None
        self.next_soul_cap = None
        self.support_bonus = {ANGEL: 0, DEVIL: 0}
        self.delayed_movements = []
        self.next_delayed_movements = []
        self.chamber_locks = {ANGEL: set(), DEVIL: set()}
        self.next_chamber_locks = {ANGEL: set(), DEVIL: set()}
        self.board = {c: [] for c in CHAMBERS}
        self.conditions = {c: [] for c in CHAMBERS}
        self.stacks = {ANGEL: {c: [] for c in CHAMBERS}, DEVIL: {c: [] for c in CHAMBERS}}
        self.selected_card = None
        self.selected_card_button = None
        self.in_resolution = False
        self.game_over = False
        self.ai_committed_play = None
        self.reset_game_stats()

        self.player_deck = DeckState(player_decks[self.player_deck_name], self.rng)
        self.ai_deck = DeckState(opponent_decks[self.ai_deck_name], self.rng)

        if self.start_frame is not None:
            self.start_frame.destroy()
            self.start_frame = None
        self.build_game_screen()
        self.log_clear()
        self.log("Welcome to Soulscale: Purgatory v1.0.1.")
        if self.is_hotseat():
            if self.is_secret_hotseat():
                self.log("Mode: Human vs Human Secret Hot-seat. Player 2 hand and Player 1 choice are hidden until reveal.")
            else:
                self.log("Mode: Human vs Human open hot-seat. Player 1 chooses first each round; then pass the keyboard to Player 2.")
            self.log(f"Player 1 is {FACTION_LABEL[self.player_faction]} using {self.player_deck_name}: {self.deck_symbols(player_decks[self.player_deck_name])}.")
            self.log(f"Player 2 is {FACTION_LABEL[self.ai_faction]} using {self.ai_deck_name}: {self.deck_symbols(opponent_decks[self.ai_deck_name])}.")
        else:
            self.log("Mode: Human vs Computer.")
            self.log(f"You are {FACTION_LABEL[self.player_faction]} using {self.player_deck_name}: {self.deck_symbols(player_decks[self.player_deck_name])}.")
            self.log(f"Computer is {FACTION_LABEL[self.ai_faction]} using {self.ai_deck_name}: {self.deck_symbols(opponent_decks[self.ai_deck_name])}.")
            self.log(f"AI personality: {self.ai_profile.label} — {self.ai_profile.description}")
        self.log(f"Goal length: ±{self.goal}. Stack layer: Press, Anchor, Release, or Sever after choosing a Chamber.")
        self.new_round()

    def reset_game_stats(self) -> None:
        self.game_stats = GameStats(
            started_at=datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
            chamber_uses={c: 0 for c in CHAMBERS},
            intent_uses={"press": 0, "anchor": 0, "release": 0, "sever": 0},
        )

    def record_round_stats(
        self,
        player_play: PendingPlay,
        opponent_play: PendingPlay,
        player_move: int,
        opponent_move: int,
        ongoing_move: int,
        uncapped_net: int,
        final_net: int,
    ) -> None:
        first_name = self.player_actor_name()
        second_name = self.opponent_actor_name()

        for play in (player_play, opponent_play):
            self.game_stats.chamber_uses[play.chamber] = self.game_stats.chamber_uses.get(play.chamber, 0) + 1
            self.game_stats.intent_uses[play.mode.key] = self.game_stats.intent_uses.get(play.mode.key, 0) + 1

        for move in (player_move, opponent_move, ongoing_move):
            if move > 0:
                self.game_stats.total_heaven_pressure += move
            elif move < 0:
                self.game_stats.total_hell_pressure += abs(move)
        self.game_stats.total_net_movement += final_net

        self._consider_biggest_single(first_name, player_play, player_move)
        self._consider_biggest_single(second_name, opponent_play, opponent_move)
        if ongoing_move:
            self._consider_biggest_ongoing(ongoing_move)

        if abs(final_net) > self.game_stats.biggest_round_abs:
            direction = "Heaven" if final_net > 0 else "Hell"
            cap_note = "" if final_net == uncapped_net else f"; capped from {uncapped_net:+d}"
            self.game_stats.biggest_round_abs = abs(final_net)
            self.game_stats.biggest_round_label = f"Round {self.round_no}: net {final_net:+d} toward {direction}{cap_note}."

        self.game_stats.round_results.append(
            {
                "round": self.round_no,
                "player_card": player_play.card.short,
                "player_chamber": player_play.chamber,
                "player_intent": player_play.mode.key,
                "player_roll": player_play.raw_roll,
                "player_move": player_move,
                "opponent_card": opponent_play.card.short,
                "opponent_chamber": opponent_play.chamber,
                "opponent_intent": opponent_play.mode.key,
                "opponent_roll": opponent_play.raw_roll,
                "opponent_move": opponent_move,
                "ongoing_move": ongoing_move,
                "uncapped_net": uncapped_net,
                "final_net": final_net,
                "soul_after": self.soul,
            }
        )

    def _consider_biggest_single(self, actor_name: str, play: PendingPlay, move: int) -> None:
        if move == 0 or abs(move) <= self.game_stats.biggest_single_abs:
            return
        direction = "Heaven" if move > 0 else "Hell"
        self.game_stats.biggest_single_abs = abs(move)
        self.game_stats.biggest_single_label = (
            f"Round {self.round_no}: {actor_name} used {play.card.short} in "
            f"{CHAMBER_EMOJI[play.chamber]} {play.chamber} as {play.mode.label}, "
            f"scoring {move:+d} toward {direction}."
        )

    def _consider_biggest_ongoing(self, move: int) -> None:
        if abs(move) <= self.game_stats.biggest_single_abs:
            return
        direction = "Heaven" if move > 0 else "Hell"
        self.game_stats.biggest_single_abs = abs(move)
        self.game_stats.biggest_single_label = (
            f"Round {self.round_no}: ongoing board effects scored {move:+d} toward {direction}."
        )

    def build_endgame_summary(self, winner_faction: str) -> str:
        winner = FACTION_SHORT[winner_faction]
        loser = FACTION_SHORT[opponent(winner_faction)]
        if self.is_hotseat():
            mode = "Human vs Human / Secret Hot-seat" if self.is_secret_hotseat() else "Human vs Human / Open hot-seat"
        else:
            mode = f"Human vs Computer — {self.ai_profile.label}"
        player_line = f"{self.player_actor_name()}: {FACTION_LABEL[self.player_faction]} / {self.player_deck_name}"
        opponent_line = f"{self.opponent_actor_name()}: {FACTION_LABEL[self.ai_faction]} / {self.ai_deck_name}"

        chamber_name, chamber_count = self.most_used_chamber()
        intent_line = self.intent_summary_line()
        pressure_net = self.game_stats.total_heaven_pressure - self.game_stats.total_hell_pressure

        lines = [
            "SOULSCALE: PURGATORY v1.0.1 — ENDGAME SUMMARY",
            "=" * 46,
            f"Winner: {winner}",
            f"Loser: {loser}",
            f"Final Soul: {self.soul:+d} / ±{self.goal}",
            f"Rounds played: {self.round_no}",
            f"Started: {self.game_stats.started_at or 'unknown'}",
            f"Ended: {self.game_stats.ended_at or 'unknown'}",
            f"Mode: {mode}",
            player_line,
            opponent_line,
            "",
            "PRESSURE TOTALS",
            "-" * 46,
            f"Total Heaven pressure: +{self.game_stats.total_heaven_pressure}",
            f"Total Hell pressure: -{self.game_stats.total_hell_pressure}",
            f"Raw pressure difference: {pressure_net:+d}",
            f"Actual capped net Soul movement: {self.game_stats.total_net_movement:+d}",
            "",
            "PLAY PATTERNS",
            "-" * 46,
            f"Most used Chamber: {CHAMBER_EMOJI[chamber_name]} {chamber_name} ({chamber_count} plays)",
            f"Intent use: {intent_line}",
            f"Biggest single swing: {self.game_stats.biggest_single_label}",
            f"Biggest net round: {self.game_stats.biggest_round_label}",
            "",
            "Last 5 rounds",
            "-" * 46,
        ]
        last = self.game_stats.round_results[-5:]
        if not last:
            lines.append("No resolved rounds recorded.")
        for row in last:
            lines.append(
                f"R{row['round']}: {row['player_card']} {row['player_move']:+d}; "
                f"{row['opponent_card']} {row['opponent_move']:+d}; "
                f"ongoing {row['ongoing_move']:+d}; net {row['final_net']:+d}; Soul {row['soul_after']:+d}"
            )
        return "\n".join(lines)

    def most_used_chamber(self) -> Tuple[str, int]:
        if not self.game_stats.chamber_uses:
            return CHAMBERS[0], 0
        return max(self.game_stats.chamber_uses.items(), key=lambda item: (item[1], -CHAMBERS.index(item[0])))

    def intent_summary_line(self) -> str:
        labels = {
            "press": "Press",
            "anchor": "Anchor",
            "release": "Release",
            "sever": "Sever",
        }
        parts = []
        for key in ("press", "anchor", "release", "sever"):
            parts.append(f"{labels[key]} {self.game_stats.intent_uses.get(key, 0)}")
        return ", ".join(parts)

    def show_endgame_summary(self, winner_faction: str) -> None:
        self.game_stats.winner_faction = winner_faction
        self.game_stats.ended_at = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        summary = self.build_endgame_summary(winner_faction)
        self.last_endgame_summary = summary
        self.log("")
        for line in summary.splitlines():
            self.log(line)
        exported = self.auto_export_playtest_log(summary)
        if exported is not None:
            txt_path, csv_path = exported
            self.game_stats.last_export_path = str(txt_path)
            self.log(f"Playtest log auto-exported: {txt_path}")
            self.log(f"Round CSV auto-exported: {csv_path}")
        self.show_summary_window(summary)

    def show_summary_window(self, summary: str, *, preview: bool = False) -> None:
        """Show the real export-capable endgame window.

        v1.0.1 keeps the v0.7.4 behavior and deliberately avoids a fake smoke-test-only path. The start-screen
        test now injects a completed-game state and calls the same summary/export
        window used by a real victory. This function is also defensive: if Tk ever
        throws while building the full window, the blank window is destroyed and a
        plain fallback export window is shown instead.
        """
        win = None
        try:
            win = tk.Toplevel(self.root)
            win.title("Endgame Summary / Playtest Export — v1.0.1")
            win.geometry("980x760")
            win.minsize(780, 540)

            try:
                win.transient(self.root)
            except tk.TclError:
                pass

            status_var = tk.StringVar()
            if self.game_stats.last_export_path:
                status_var.set(f"Auto-exported full log to: {self.game_stats.last_export_path}")
            else:
                status_var.set("Use SAVE FULL LOG... to choose a report path and write the .txt + .rounds.csv files.")

            def copy_summary() -> None:
                self.root.clipboard_clear()
                self.root.clipboard_append(summary)
                self.root.update()
                messagebox.showinfo("Copied", "Summary copied to clipboard.")

            def save_full_log() -> None:
                exported = self.save_playtest_log(summary=summary, ask=True)
                if exported is None:
                    return
                txt_path, csv_path = exported
                self.game_stats.last_export_path = str(txt_path)
                status_var.set(f"Saved full log to: {txt_path}")
                messagebox.showinfo(
                    "Playtest log exported",
                    "Saved full report:\n" + str(txt_path) + "\n\nSaved round CSV:\n" + str(csv_path),
                )

            outer = tk.Frame(win, padx=14, pady=12)
            outer.pack(fill="both", expand=True)

            header = tk.Frame(outer)
            header.pack(fill="x", pady=(0, 8))
            tk.Label(
                header,
                text="Endgame Summary / Playtest Export — v1.0.1",
                font=("Segoe UI", 17, "bold"),
            ).pack(side="left")
            tk.Label(
                header,
                text="REAL ENDGAME WINDOW TEST PATH",
                font=("Segoe UI", 9, "bold"),
                fg="#7a2e00",
            ).pack(side="right")

            toolbar = tk.Frame(outer, padx=10, pady=10, relief="ridge", borderwidth=2)
            toolbar.pack(fill="x", pady=(0, 8))
            save_button = tk.Button(
                toolbar,
                text="SAVE FULL LOG...",
                command=save_full_log,
                width=24,
                font=("Segoe UI", 14, "bold"),
            )
            save_button.pack(side="left", padx=(0, 10), ipady=6)
            tk.Button(
                toolbar,
                text="Copy summary",
                command=copy_summary,
                width=16,
                font=("Segoe UI", 11, "bold"),
            ).pack(side="left", padx=5, ipady=5)
            tk.Button(
                toolbar,
                text="Close",
                command=win.destroy,
                width=12,
                font=("Segoe UI", 11),
            ).pack(side="right", padx=5, ipady=5)

            tk.Label(
                outer,
                textvariable=status_var,
                font=("Segoe UI", 9, "bold"),
                anchor="w",
                justify="left",
                wraplength=920,
                padx=2,
                pady=4,
            ).pack(fill="x")

            text_frame = tk.Frame(outer)
            text_frame.pack(fill="both", expand=True, pady=(8, 8))
            scrollbar = tk.Scrollbar(text_frame, orient="vertical")
            scrollbar.pack(side="right", fill="y")
            text = tk.Text(
                text_frame,
                wrap="word",
                font=self.font_mono,
                padx=10,
                pady=10,
                yscrollcommand=scrollbar.set,
            )
            text.pack(side="left", fill="both", expand=True)
            scrollbar.configure(command=text.yview)
            text.insert("1.0", summary)
            text.configure(state="disabled")

            bottom = tk.Frame(outer)
            bottom.pack(fill="x")
            tk.Button(
                bottom,
                text="SAVE FULL LOG...",
                command=save_full_log,
                width=24,
                font=("Segoe UI", 12, "bold"),
            ).pack(side="left", padx=(0, 10), ipady=4)
            tk.Button(bottom, text="Copy summary", command=copy_summary, width=16).pack(side="left", padx=4, ipady=3)
            tk.Button(bottom, text="Close", command=win.destroy, width=12).pack(side="right", padx=4, ipady=3)

            win.bind("<Control-s>", lambda _event: save_full_log())
            win.bind("<Control-S>", lambda _event: save_full_log())
            win.update_idletasks()
            self.center_window(win)
            win.lift()
            save_button.focus_set()
        except Exception as exc:
            try:
                if win is not None and win.winfo_exists():
                    win.destroy()
            except Exception:
                pass
            self.show_summary_window_fallback(summary, exc)

    def show_summary_window_fallback(self, summary: str, exc: Exception) -> None:
        """Emergency plain-Tk export window so a UI construction error never leaves a blank popup."""
        win = tk.Toplevel(self.root)
        win.title("Endgame Summary / Playtest Export — v1.0.1 FALLBACK")
        win.geometry("900x700")
        win.minsize(700, 500)

        def copy_summary() -> None:
            self.root.clipboard_clear()
            self.root.clipboard_append(summary)
            self.root.update()
            messagebox.showinfo("Copied", "Summary copied to clipboard.")

        def save_full_log() -> None:
            exported = self.save_playtest_log(summary=summary, ask=True)
            if exported is None:
                return
            txt_path, csv_path = exported
            self.game_stats.last_export_path = str(txt_path)
            messagebox.showinfo(
                "Playtest log exported",
                f"Saved full report:\n{txt_path}\n\nSaved round CSV:\n{csv_path}",
            )

        top = tk.Frame(win)
        top.pack(fill="x", padx=12, pady=12)
        tk.Label(top, text="FALLBACK EXPORT WINDOW — original popup failed", font=("Segoe UI", 13, "bold")).pack(anchor="w")
        tk.Label(top, text=f"Error while building rich popup: {type(exc).__name__}: {exc}", fg="red", wraplength=840, justify="left").pack(anchor="w", pady=(4, 8))
        tk.Button(top, text="SAVE FULL LOG...", command=save_full_log, font=("Segoe UI", 13, "bold"), width=24).pack(side="left", padx=(0, 8), ipady=5)
        tk.Button(top, text="Copy summary", command=copy_summary, width=16).pack(side="left", padx=4, ipady=4)
        tk.Button(top, text="Close", command=win.destroy, width=12).pack(side="right", padx=4, ipady=4)

        text = tk.Text(win, wrap="word", font=self.font_mono)
        text.pack(fill="both", expand=True, padx=12, pady=(0, 12))
        text.insert("1.0", summary)
        text.configure(state="disabled")
        win.bind("<Control-s>", lambda _event: save_full_log())
        win.bind("<Control-S>", lambda _event: save_full_log())
        win.update_idletasks()
        self.center_window(win)
        win.lift()

    def force_endgame_window_test(self) -> None:
        """Create a completed-game test state and run the exact real endgame/export path.

        This replaces the earlier misleading smoke test. It does not pretend there is
        no game data; it injects a small valid finished match ledger, then calls
        show_endgame_summary(), which is the same method used by real wins.
        """
        self.game_mode = self.game_mode_var.get() if hasattr(self, "game_mode_var") else self.game_mode
        self.player_faction = self.side_var.get() if hasattr(self, "side_var") else self.player_faction
        self.ai_faction = opponent(self.player_faction)
        self.goal = int(self.goal_var.get()) if hasattr(self, "goal_var") else self.goal
        self.ai_profile_name = self.ai_profile_var.get() if hasattr(self, "ai_profile_var") else self.ai_profile_name
        self.ai_profile = AI_PROFILES[self.ai_profile_name]
        self.player_deck_name = self.deck_var.get() if hasattr(self, "deck_var") else self.player_deck_name
        self.ai_deck_name = self.opponent_deck_var.get() if hasattr(self, "opponent_deck_var") else self.ai_deck_name
        self.opponent_deck_name = self.ai_deck_name

        now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        self.round_no = 6
        self.soul = self.goal
        self.game_over = True
        self.game_stats = GameStats(
            started_at=now,
            ended_at=now,
            winner_faction=ANGEL,
            chamber_uses={"Body": 2, "Mind": 2, "Heart": 3, "Time": 1, "Memory": 4},
            intent_uses={"press": 4, "anchor": 3, "release": 3, "sever": 2},
            total_heaven_pressure=22,
            total_hell_pressure=15,
            total_net_movement=self.goal,
            biggest_single_abs=6,
            biggest_single_label="Test Round 5: Heaven used 💌 Loved Memory in 🪞 Memory as Release, scoring +6 toward Heaven.",
            biggest_round_abs=5,
            biggest_round_label="Test Round 5: net +5 toward Heaven; capped from +6.",
        )
        self.game_stats.round_results = [
            {
                "round": 1,
                "player_card": "🦷 Ache",
                "player_chamber": "Body",
                "player_intent": "Press",
                "player_roll": 3,
                "player_move": -2,
                "opponent_card": "🕯️ Candle of Warmth",
                "opponent_chamber": "Heart",
                "opponent_intent": "Press",
                "opponent_roll": 4,
                "opponent_move": 2,
                "ongoing_move": 0,
                "uncapped_net": 0,
                "final_net": 0,
                "soul_after": 0,
            },
            {
                "round": 2,
                "player_card": "🪤 False Door",
                "player_chamber": "Memory",
                "player_intent": "Sever",
                "player_roll": 5,
                "player_move": 0,
                "opponent_card": "🎵 Hymn",
                "opponent_chamber": "Mind",
                "opponent_intent": "Anchor",
                "opponent_roll": 3,
                "opponent_move": 0,
                "ongoing_move": 0,
                "uncapped_net": 0,
                "final_net": 0,
                "soul_after": 0,
            },
            {
                "round": 3,
                "player_card": "🥀 Memory Thorn",
                "player_chamber": "Memory",
                "player_intent": "Press",
                "player_roll": 4,
                "player_move": -3,
                "opponent_card": "🤲 Open Hand",
                "opponent_chamber": "Heart",
                "opponent_intent": "Press",
                "opponent_roll": 5,
                "opponent_move": 4,
                "ongoing_move": 1,
                "uncapped_net": 2,
                "final_net": 2,
                "soul_after": 2,
            },
            {
                "round": 4,
                "player_card": "⛓️ Burden",
                "player_chamber": "Time",
                "player_intent": "Anchor",
                "player_roll": 2,
                "player_move": 0,
                "opponent_card": "🪽 Guardian",
                "opponent_chamber": "Heart",
                "opponent_intent": "Anchor",
                "opponent_roll": 4,
                "opponent_move": 0,
                "ongoing_move": 1,
                "uncapped_net": 1,
                "final_net": 1,
                "soul_after": 3,
            },
            {
                "round": 5,
                "player_card": "🐍 Whispering Serpent",
                "player_chamber": "Mind",
                "player_intent": "Release",
                "player_roll": 2,
                "player_move": -2,
                "opponent_card": "💌 Loved Memory",
                "opponent_chamber": "Memory",
                "opponent_intent": "Release",
                "opponent_roll": 6,
                "opponent_move": 6,
                "ongoing_move": 1,
                "uncapped_net": 5,
                "final_net": 5,
                "soul_after": 8,
            },
            {
                "round": 6,
                "player_card": "🕳️ Void Thought",
                "player_chamber": "Mind",
                "player_intent": "Press",
                "player_roll": 3,
                "player_move": -2,
                "opponent_card": "✨ Little Miracle",
                "opponent_chamber": "Memory",
                "opponent_intent": "Release",
                "opponent_roll": 5,
                "opponent_move": self.goal - 7,
                "ongoing_move": 1,
                "uncapped_net": self.goal - 8,
                "final_net": self.goal - 8,
                "soul_after": self.goal,
            },
        ]
        self.log_lines = [
            "[v1.0.1 TEST] Forced completed-game state created from the start screen.",
            "[v1.0.1 TEST] Calling show_endgame_summary(Heaven), the same path used after a real victory.",
            "[v1.0.1 TEST] Use SAVE FULL LOG... in the popup to test manual export without playing a full match.",
        ]
        self.show_endgame_summary(ANGEL)

    def preview_summary_window(self) -> None:
        """Backward-compatible alias for old buttons; now uses the real endgame test path."""
        self.force_endgame_window_test()

    def build_interim_summary(self) -> str:
        if self.is_hotseat():
            mode = "Human vs Human / Secret Hot-seat" if self.is_secret_hotseat() else "Human vs Human / Open hot-seat"
        else:
            mode = f"Human vs Computer — {self.ai_profile.label}"
        player_line = f"{self.player_actor_name()}: {FACTION_LABEL[self.player_faction]} / {self.player_deck_name}"
        opponent_line = f"{self.opponent_actor_name()}: {FACTION_LABEL[self.ai_faction]} / {self.ai_deck_name}"
        chamber_name, chamber_count = self.most_used_chamber()
        pressure_net = self.game_stats.total_heaven_pressure - self.game_stats.total_hell_pressure
        lines = [
            "SOULSCALE: PURGATORY — INTERIM PLAYTEST REPORT",
            "=" * 52,
            f"Status: in progress",
            f"Current Soul: {self.soul:+d} / ±{self.goal}",
            f"Current pull: {'Heaven' if self.soul > 0 else 'Hell' if self.soul < 0 else 'Balanced'}",
            f"Round counter: {self.round_no}",
            f"Started: {self.game_stats.started_at or 'unknown'}",
            f"Exported: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
            f"Mode: {mode}",
            player_line,
            opponent_line,
            "",
            "PRESSURE TOTALS SO FAR",
            "-" * 52,
            f"Total Heaven pressure: +{self.game_stats.total_heaven_pressure}",
            f"Total Hell pressure: -{self.game_stats.total_hell_pressure}",
            f"Raw pressure difference: {pressure_net:+d}",
            f"Actual capped net Soul movement: {self.game_stats.total_net_movement:+d}",
            "",
            "PLAY PATTERNS SO FAR",
            "-" * 52,
            f"Most used Chamber: {CHAMBER_EMOJI[chamber_name]} {chamber_name} ({chamber_count} plays)",
            f"Intent use: {self.intent_summary_line()}",
            f"Biggest single swing: {self.game_stats.biggest_single_label}",
            f"Biggest net round: {self.game_stats.biggest_round_label}",
        ]
        return "\n".join(lines)

    def build_playtest_export_text(self, summary: Optional[str] = None) -> str:
        if summary is None:
            if self.game_over and self.game_stats.winner_faction:
                summary = self.build_endgame_summary(self.game_stats.winner_faction)
            else:
                summary = self.build_interim_summary()

        lines = [
            summary,
            "",
            "FULL ROUND LEDGER",
            "=" * 52,
        ]
        if not self.game_stats.round_results:
            lines.append("No resolved rounds recorded.")
        else:
            for row in self.game_stats.round_results:
                lines.append(
                    f"R{row['round']:>2}: "
                    f"{self.player_actor_name()} {row['player_card']} → {row['player_chamber']} "
                    f"/{row['player_intent']} roll {row['player_roll']} = {row['player_move']:+d}; "
                    f"{self.opponent_actor_name()} {row['opponent_card']} → {row['opponent_chamber']} "
                    f"/{row['opponent_intent']} roll {row['opponent_roll']} = {row['opponent_move']:+d}; "
                    f"ongoing {row['ongoing_move']:+d}; uncapped {row['uncapped_net']:+d}; "
                    f"final {row['final_net']:+d}; Soul {row['soul_after']:+d}"
                )
        lines.extend([
            "",
            "RAW GAME LOG",
            "=" * 52,
            self.current_log_text().rstrip() or "No raw log lines recorded.",
            "",
        ])
        return "\n".join(lines)

    def current_log_text(self) -> str:
        if self.log_lines:
            return "\n".join(self.log_lines)
        if hasattr(self, "log_text"):
            try:
                return self.log_text.get("1.0", "end-1c")
            except tk.TclError:
                return ""
        return ""

    def default_export_dir(self) -> Path:
        return Path.cwd() / "soulscale_playtest_logs"

    def playtest_export_basename(self) -> str:
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        mode = "hotseat" if self.is_hotseat() else "ai"
        winner = FACTION_SHORT.get(self.game_stats.winner_faction, "in_progress").lower().replace(" ", "_")
        p1 = self.safe_slug(self.player_deck_name)
        p2 = self.safe_slug(self.ai_deck_name)
        return f"soulscale_v101_{timestamp}_{mode}_{winner}_r{self.round_no}_{p1}_vs_{p2}"

    def safe_slug(self, value: str, max_len: int = 42) -> str:
        allowed = []
        for ch in value.lower():
            if ch.isalnum():
                allowed.append(ch)
            elif ch in (" ", "-", "_", "."):
                allowed.append("_")
        slug = "".join(allowed).strip("_")
        while "__" in slug:
            slug = slug.replace("__", "_")
        return (slug or "deck")[:max_len]

    def auto_export_playtest_log(self, summary: str) -> Optional[Tuple[Path, Path]]:
        try:
            return self.save_playtest_log(summary=summary, ask=False)
        except OSError as exc:
            self.log(f"[warn] Could not auto-export playtest log: {exc}")
            return None

    def export_current_playtest_log(self) -> None:
        exported = self.save_playtest_log(summary=None, ask=True)
        if exported is None:
            return
        txt_path, csv_path = exported
        self.game_stats.last_export_path = str(txt_path)
        self.log(f"Manual playtest log export: {txt_path}")
        self.log(f"Manual round CSV export: {csv_path}")
        messagebox.showinfo(
            "Playtest log exported",
            f"Saved full report:\n{txt_path}\n\nSaved round CSV:\n{csv_path}",
        )

    def save_playtest_log(self, summary: Optional[str], ask: bool) -> Optional[Tuple[Path, Path]]:
        default_dir = self.default_export_dir()
        default_dir.mkdir(parents=True, exist_ok=True)
        base = self.playtest_export_basename()
        if ask:
            chosen = filedialog.asksaveasfilename(
                title="Export Soulscale playtest log",
                initialdir=str(default_dir),
                initialfile=f"{base}.txt",
                defaultextension=".txt",
                filetypes=[("Text report", "*.txt"), ("All files", "*.*")],
            )
            if not chosen:
                return None
            txt_path = Path(chosen)
        else:
            txt_path = default_dir / f"{base}.txt"

        if txt_path.suffix.lower() != ".txt":
            txt_path = txt_path.with_suffix(".txt")
        csv_path = txt_path.with_suffix(".rounds.csv")

        txt_path.write_text(self.build_playtest_export_text(summary), encoding="utf-8")
        self.write_round_csv(csv_path)
        return txt_path, csv_path

    def write_round_csv(self, path: Path) -> None:
        fields = [
            "round",
            "player_card",
            "player_chamber",
            "player_intent",
            "player_roll",
            "player_move",
            "opponent_card",
            "opponent_chamber",
            "opponent_intent",
            "opponent_roll",
            "opponent_move",
            "ongoing_move",
            "uncapped_net",
            "final_net",
            "soul_after",
        ]
        with path.open("w", newline="", encoding="utf-8-sig") as handle:
            writer = csv.DictWriter(handle, fieldnames=fields, extrasaction="ignore")
            writer.writeheader()
            for row in self.game_stats.round_results:
                writer.writerow(row)

    def is_hotseat(self) -> bool:
        return self.game_mode == "hotseat"

    def is_secret_hotseat(self) -> bool:
        return self.is_hotseat() and bool(getattr(self, "secret_hotseat", False))

    def player_actor_name(self) -> str:
        return "Player 1" if self.is_hotseat() else "You"

    def opponent_actor_name(self) -> str:
        return "Player 2" if self.is_hotseat() else "Computer"

    def choose_ai_deck_name(self, faction: str) -> str:
        names = list(DECKS[faction].keys())
        if self.ai_profile_name == "Architect / Stack Builder":
            preferred = "Sanctuary Control" if faction == ANGEL else "Despair Control"
            if preferred in DECKS[faction]:
                return preferred
        if self.ai_profile_name == "Cruel Accountant / Hard":
            preferred = "Hope From Behind" if faction == ANGEL else "Trickster Hell"
            if preferred in DECKS[faction]:
                return preferred
        if self.ai_profile_name == "Chaos Imp / Weird":
            preferred = "Hope From Behind" if faction == ANGEL else "Trickster Hell"
            if preferred in DECKS[faction]:
                return preferred
        return self.rng.choice(names)

    def build_game_screen(self) -> None:
        self.game_frame = tk.Frame(self.root, padx=10, pady=10)
        self.game_frame.pack(fill="both", expand=True)

        top = tk.Frame(self.game_frame)
        top.pack(fill="x")

        left_status = tk.Frame(top)
        left_status.pack(side="left", fill="x", expand=True)

        tk.Label(left_status, textvariable=self.soul_var, font=("Segoe UI Emoji", 16, "bold")).pack(anchor="w")
        tk.Label(left_status, textvariable=self.will_var, font=("Segoe UI", 11)).pack(anchor="w")
        tk.Label(left_status, textvariable=self.strain_var, font=("Segoe UI", 11)).pack(anchor="w")
        tk.Label(left_status, textvariable=self.selected_var, font=("Segoe UI Emoji", 11)).pack(anchor="w", pady=(6, 0))

        right_status = tk.Frame(top)
        right_status.pack(side="right", anchor="ne")

        tk.Button(right_status, text="New Game", command=self.confirm_new_game).pack(side="right", padx=4)
        tk.Button(right_status, text="Test Endgame Export", command=self.force_endgame_window_test).pack(side="right", padx=4)
        tk.Button(right_status, text="Export Log", command=self.export_current_playtest_log).pack(side="right", padx=4)
        tk.Button(right_status, text="Rules Help", command=self.show_help).pack(side="right", padx=4)
        tk.Button(right_status, text="Deck Glyphs", command=self.show_glyph_reference).pack(side="right", padx=4)

        self.soul_canvas = tk.Canvas(self.game_frame, height=72, bg="#f2f2f2", highlightthickness=1, highlightbackground="#cccccc")
        self.soul_canvas.pack(fill="x", pady=10)

        board_frame = tk.LabelFrame(self.game_frame, text="Purgatory Chambers — click a Chamber after selecting a glyph", padx=8, pady=8)
        board_frame.pack(fill="x", pady=(0, 10))

        for idx, chamber in enumerate(CHAMBERS):
            box = tk.Frame(board_frame, relief="ridge", borderwidth=1, padx=6, pady=6)
            box.grid(row=0, column=idx, padx=5, sticky="nsew")
            board_frame.columnconfigure(idx, weight=1)

            tk.Label(box, text=f"{CHAMBER_EMOJI[chamber]}\n{chamber}", font=("Segoe UI Emoji", 17, "bold")).pack()
            self.chamber_text_vars[chamber] = tk.StringVar()
            tk.Label(
                box,
                textvariable=self.chamber_text_vars[chamber],
                font=("Segoe UI Emoji", 9),
                justify="left",
                width=24,
                height=14,
                anchor="n",
            ).pack(pady=4)

            chamber_button = tk.Button(
                box,
                text=f"Play to {chamber}",
                command=lambda ch=chamber: self.choose_chamber(ch),
                font=("Segoe UI", 9),
            )
            chamber_button.pack(fill="x")
            self.chamber_buttons.append(chamber_button)

        reveal_frame = tk.Frame(self.game_frame)
        reveal_frame.pack(fill="x")

        player_reveal_box = tk.LabelFrame(reveal_frame, text=("Player 1 dealt glyphs" if self.is_hotseat() else "Your dealt glyphs"), padx=8, pady=8)
        player_reveal_box.pack(side="left", fill="both", expand=True, padx=(0, 6))
        self.player_cards_frame = tk.Frame(player_reveal_box)
        self.player_cards_frame.pack(fill="x")

        ai_reveal_box = tk.LabelFrame(reveal_frame, text=("Player 2 dealt glyphs" if self.is_hotseat() else "Computer dealt glyphs"), padx=8, pady=8)
        ai_reveal_box.pack(side="right", fill="both", expand=True, padx=(6, 0))
        tk.Label(ai_reveal_box, textvariable=self.ai_reveal_var, font=("Segoe UI Emoji", 10), justify="left").pack(anchor="w")

        log_frame = tk.LabelFrame(self.game_frame, text="Game log", padx=8, pady=8)
        log_frame.pack(fill="both", expand=True, pady=(10, 0))

        self.log_text = tk.Text(log_frame, height=13, wrap="word", font=self.font_mono)
        self.log_text.pack(side="left", fill="both", expand=True)
        scroll = tk.Scrollbar(log_frame, command=self.log_text.yview)
        scroll.pack(side="right", fill="y")
        self.log_text.configure(yscrollcommand=scroll.set, state="disabled")

    def confirm_new_game(self) -> None:
        if messagebox.askyesno("New Game", "Abandon this game and return to setup?"):
            if self.game_frame is not None:
                self.game_frame.destroy()
                self.game_frame = None
            self.build_start_screen()

    def show_help(self) -> None:
        messagebox.showinfo(
            "Rules Help",
            "Soulscale: Purgatory v1.0.1\n\n"
            "Modes\n"
            "Human vs Computer, or Human vs Human hot-seat for local playtesting.\n\n"
            "Playtest export\n"
            "The Export Log button writes a full .txt report and a per-round .csv ledger. Finished games also auto-export into ./soulscale_playtest_logs/. v0.7.4 added Test Real Endgame Export Window so you can test the actual finished-game export popup without playing a full match. v0.8 added the visual deck editor; v0.9 adds optional Secret Hot-seat mode; v1.0 adds rarity, price keywords, 10-card decks, 4-card reveals, and 16 new glyphs; v1.0.1 adds the rules-aware movement resolver.\n\n"
            "Goal\n"
            "Choose Quick ±12, Standard ±18, or Deep ±24 on the setup screen. Normal Heaven contributions are positive and normal Hell contributions are negative. v1.0.1 replaces the old blunt polarity guard with a movement resolver, so future cards can intentionally declare opponentward/heavenward/hellward/neutral movement as a cost, lure, or global event.\n"
            "The log prints a movement audit every round so you can see each side separately before the net movement is applied.\n\n"
            "Round\n"
            "Reveal 4 glyphs. In Human vs Computer, the AI secretly commits before you choose. In open Hot-seat, both hands are visible. In Secret Hot-seat, Player 2's hand is hidden while Player 1 chooses, then Player 1's hand/choice/log details are masked before Player 2 chooses. Then both plays reveal and resolve.\n\n"
            "Intent choices\n"
            "Press: uses existing friendly stack for immediate bonus and adds a normal stack token.\n"
            "Anchor: no immediate Soul movement, but builds a much bigger stack token.\n"
            "Release: spend stack before knowing the roll. Bigger spend + better roll = capped burst.\n"
            "Sever: weaker immediate result, but cuts the opponent's stack in that Chamber.\n\n"
            "Stack math\n"
            "Press bonus = floor((friendly stack + same-aspect harmony - half enemy stack) / 3).\n"
            "Release bonus = min(5, ceil(spent stack × intensity / 4)).\n"
            "Sever cut = card stack value + intensity, +1 if the glyph is disruptive.\n\n"
            "v1.0.1 rarity / price / movement resolver\n"
            f"Decks now use {DECK_SIZE} glyphs, reveal {REVEAL_SIZE}, budget ≤ {DECK_RARITY_BUDGET}, and rares ≤ {DECK_MAX_RARES}. "
            "Common = stable, Uncommon = tactical, Rare = build-around/volatile. Price keywords are the balancing costs: Heavy costs Will, Fragile hates contests, Volatile spikes or stumbles, Delayed scores next round, Binding locks its Chamber next round, Hungry consumes stack, Echo repeats weakly.\n\n"
            "Tip: Anchor when you can afford time. Release when it can swing or finish. Sever when the enemy is clearly building a bomb.",
        )

    def show_glyph_reference(self) -> None:
        lines = ["CUSTOM DECK FORMAT", f"Exactly {DECK_SIZE} non-empty lines, one emoji per line.", ""]
        for faction in (ANGEL, DEVIL):
            lines.append(FACTION_LABEL[faction])
            for card in CARDS.values():
                if card.faction == faction:
                    lines.append(f"{card.emoji}  {card.name}  [{card.rarity}; {card.aspect}; stack {card.stack_value}; price {price_label(card)}; move {movement_note(card)}]")
            lines.append("")
        win = tk.Toplevel(self.root)
        win.title("Glyph Reference")
        win.transient(self.root)
        win.geometry("520x620")
        text = tk.Text(win, wrap="word", font=("Segoe UI Emoji", 11), padx=10, pady=10)
        text.pack(fill="both", expand=True)
        text.insert("1.0", "\n".join(lines))
        text.config(state="disabled")

    # ------------------------------------------------------------------
    # Rounds and player actions
    # ------------------------------------------------------------------

    def new_round(self) -> None:
        if self.game_over:
            return
        assert self.player_deck is not None
        assert self.ai_deck is not None

        self.round_no += 1
        self.player_will = max(0, 3 + self.player_next_will_bonus - self.player_next_will_penalty)
        self.ai_will = max(0, 3 + self.ai_next_will_bonus - self.ai_next_will_penalty)
        self.player_next_will_bonus = 0
        self.ai_next_will_bonus = 0
        self.player_next_will_penalty = 0
        self.ai_next_will_penalty = 0

        self.chamber_locks = {ANGEL: set(self.next_chamber_locks[ANGEL]), DEVIL: set(self.next_chamber_locks[DEVIL])}
        self.next_chamber_locks = {ANGEL: set(), DEVIL: set()}
        self.apply_delayed_movements_at_round_start()
        if self.game_over:
            return

        player_reveal = self.player_deck.reveal(REVEAL_SIZE)
        opponent_reveal = self.ai_deck.reveal(REVEAL_SIZE)

        self.selected_card = None
        self.selected_card_button = None
        self.in_resolution = False
        self.secret_hotseat_show_p2 = False
        self.selected_var.set(f"{self.player_actor_name()}: select a glyph, then click a Chamber.")

        self.render_player_reveal(player_reveal)
        self.render_ai_reveal(opponent_reveal)
        self.refresh_status()
        self.refresh_board()

        self.log("")
        self.log(f"=== Round {self.round_no} ===")
        if self.is_secret_hotseat():
            self.log("Secret hot-seat reveal: both players' dealt glyphs are hidden from each other until played.")
        else:
            self.log(f"{self.player_actor_name()} reveal: {', '.join(c.short for c in player_reveal)}")
            self.log(f"{self.opponent_actor_name()} reveal: {', '.join(c.short for c in opponent_reveal)}")

        self.ai_committed_play = None
        if not self.is_hotseat():
            self.ai_committed_play = self.choose_ai_play(None, None, None)
            self.log("Computer secretly commits a play now, before your glyph/Chamber/Intent choice.")

    def render_player_reveal(self, cards: List[Card]) -> None:
        for child in self.player_cards_frame.winfo_children():
            child.destroy()
        self.card_buttons = []

        for idx, card in enumerate(cards):
            b = tk.Button(
                self.player_cards_frame,
                text=(
                    f"{card.emoji}  {card.name}  [{rarity_badge(card)}]\n"
                    f"{card.type_label}\n"
                    f"{self.card_chambers_label(card)}\n"
                    f"{card.aspect} • stack {card.stack_value} • {price_label(card)}"
                ),
                font=self.font_card,
                justify="center",
                width=24,
                height=6,
                command=lambda c=card, i=idx: self.select_card(c, i),
            )
            b.grid(row=0, column=idx, padx=5, pady=3, sticky="nsew")
            self.player_cards_frame.columnconfigure(idx, weight=1)
            self.card_buttons.append(b)

    def render_ai_reveal(self, cards: List[Card]) -> None:
        if self.is_secret_hotseat() and not self.secret_hotseat_show_p2:
            hidden = ["🔒 Player 2 hand hidden in Secret Hot-seat.", "", "Pass the keyboard only after Player 1 has committed."]
            for idx in range(max(1, len(cards))):
                hidden.append(f"   Slot {idx + 1}: [hidden]")
            self.ai_reveal_var.set("\n".join(hidden))
            return
        lines = []
        for card in cards:
            lines.append(f"{card.emoji} {card.name} [{rarity_badge(card)}] — {card.type_label}")
            lines.append(f"   {self.card_chambers_label(card)} — {card.aspect}, stack {card.stack_value}, price {price_label(card)}")
        self.ai_reveal_var.set("\n".join(lines))

    def render_player_reveal_hidden(self) -> None:
        for child in self.player_cards_frame.winfo_children():
            child.destroy()
        self.card_buttons = []
        count = len(self.player_deck.revealed) if self.player_deck is not None else REVEAL_SIZE
        for idx in range(max(1, count)):
            b = tk.Button(
                self.player_cards_frame,
                text="🔒 Player 1 glyph hidden\nSecret Hot-seat\nCommitted already",
                font=self.font_card,
                justify="center",
                width=24,
                height=6,
                state="disabled",
            )
            b.grid(row=0, column=idx, padx=5, pady=3, sticky="nsew")
            self.player_cards_frame.columnconfigure(idx, weight=1)
            self.card_buttons.append(b)

    def restore_public_reveals_after_secret_choice(self) -> None:
        if self.player_deck is not None:
            self.render_player_reveal([CARDS[key] for key in self.player_deck.revealed])
        self.secret_hotseat_show_p2 = True
        if self.ai_deck is not None:
            self.render_ai_reveal([CARDS[key] for key in self.ai_deck.revealed])
        self.rebuild_log_widget()

    def rebuild_log_widget(self) -> None:
        if not hasattr(self, "log_text"):
            return
        self.log_text.configure(state="normal")
        self.log_text.delete("1.0", "end")
        if self.log_lines:
            self.log_text.insert("end", "\n".join(self.log_lines) + "\n")
        self.log_text.configure(state="disabled")
        self.log_text.see("end")

    def mask_main_window_for_player2_secret_choice(self) -> None:
        self.render_player_reveal_hidden()
        self.secret_hotseat_show_p2 = True
        if self.ai_deck is not None:
            self.render_ai_reveal([CARDS[key] for key in self.ai_deck.revealed])
        self.selected_var.set("Secret Hot-seat: Player 2 is choosing. Player 1's committed play is hidden.")
        if hasattr(self, "log_text"):
            self.log_text.configure(state="normal")
            self.log_text.delete("1.0", "end")
            self.log_text.insert(
                "end",
                "🔒 Secret Hot-seat privacy screen\n\n"
                "Player 1 has committed a glyph, Chamber, Intent, and roll decision.\n"
                "Those details are hidden until Player 2 commits.\n\n"
                "Player 2 may now choose from their revealed glyphs."
            )
            self.log_text.configure(state="disabled")

    def show_secret_pass_screen(self) -> bool:
        result = {"ok": False}
        win = tk.Toplevel(self.root)
        win.title("Secret Hot-seat — pass to Player 2")
        win.transient(self.root)
        win.grab_set()
        win.resizable(False, False)
        win.configure(bg="#111111")

        frame = tk.Frame(win, padx=30, pady=26, bg="#111111")
        frame.pack(fill="both", expand=True)
        tk.Label(
            frame,
            text="🔒 PASS TO PLAYER 2",
            font=("Segoe UI Emoji", 22, "bold"),
            fg="#ffffff",
            bg="#111111",
        ).pack(pady=(0, 12))
        tk.Label(
            frame,
            text=(
                "Player 1 has locked in their play.\n\n"
                "Player 2: take the keyboard now. Player 1 should look away.\n"
                "The next panel shows only Player 2's available glyphs and choices."
            ),
            font=("Segoe UI", 12),
            fg="#eeeeee",
            bg="#111111",
            justify="center",
            wraplength=520,
        ).pack(pady=(0, 18))

        def proceed() -> None:
            result["ok"] = True
            win.destroy()

        def cancel() -> None:
            result["ok"] = False
            win.destroy()

        tk.Button(
            frame,
            text="I am Player 2 — choose my play",
            command=proceed,
            font=("Segoe UI", 13, "bold"),
            width=32,
            pady=8,
        ).pack(pady=(4, 8))
        tk.Button(frame, text="Cancel round choice", command=cancel, width=20).pack()
        win.protocol("WM_DELETE_WINDOW", cancel)
        self.center_window(win)
        self.root.wait_window(win)
        return result["ok"]

    def card_chambers_label(self, card: Card) -> str:
        return " / ".join(f"{CHAMBER_EMOJI[ch]} {ch}" for ch in card.valid_chambers)

    def select_card(self, card: Card, button_index: int) -> None:
        if self.in_resolution or self.game_over:
            return
        self.selected_card = card
        for b in self.card_buttons:
            b.configure(relief="raised", bg="SystemButtonFace")
        if 0 <= button_index < len(self.card_buttons):
            self.selected_card_button = self.card_buttons[button_index]
            self.selected_card_button.configure(relief="sunken", bg="#dff0ff")
        self.selected_var.set(f"Selected: {card.short}. Now click a valid Chamber.")
        if self.is_secret_hotseat():
            self.log("Player 1 selected a glyph. Details hidden until reveal.")
        else:
            self.log(f"Selected {card.short}. Valid: {', '.join(card.valid_chambers)}.")

    def choose_chamber(self, chamber: str) -> None:
        if self.in_resolution or self.game_over:
            return
        if self.selected_card is None:
            messagebox.showwarning("No glyph selected", "Pick one of your dealt glyphs first.")
            return
        if chamber not in self.selected_card.valid_chambers:
            messagebox.showwarning(
                "Invalid Chamber",
                f"{self.selected_card.short} cannot be played in {CHAMBER_EMOJI[chamber]} {chamber}.\n\n"
                f"Valid Chambers: {', '.join(self.selected_card.valid_chambers)}",
            )
            return
        if self.is_chamber_locked(self.selected_card.faction, chamber):
            messagebox.showwarning(
                "Binding lock",
                f"{FACTION_SHORT[self.selected_card.faction]} cannot use {CHAMBER_EMOJI[chamber]} {chamber} this round.\n\n"
                "A Binding glyph committed that Chamber last round.",
            )
            return

        mode = self.ask_player_mode(self.selected_card, chamber)
        if mode is None:
            return

        self.in_resolution = True
        if self.is_hotseat():
            self.resolve_hotseat_turn(self.selected_card, chamber, mode)
        else:
            self.resolve_turn(self.selected_card, chamber, mode)

    # ------------------------------------------------------------------
    # Player dialogs
    # ------------------------------------------------------------------

    def ask_player_mode(self, card: Card, chamber: str) -> Optional[ModeChoice]:
        own = self.stack_total(card.faction, chamber)
        enemy = self.stack_total(opponent(card.faction), chamber)
        harmony = self.harmony_count(card.faction, chamber, card.aspect)
        press_bonus = max(0, (own + harmony - enemy // 2) // 3)

        result: Dict[str, Optional[ModeChoice]] = {"mode": None}

        win = tk.Toplevel(self.root)
        win.title("Choose Intent")
        win.transient(self.root)
        win.grab_set()
        win.resizable(False, False)

        tk.Label(
            win,
            text=f"{card.emoji} {card.name} → {CHAMBER_EMOJI[chamber]} {chamber}",
            font=("Segoe UI Emoji", 15, "bold"),
            padx=22,
            pady=10,
        ).pack()

        summary = (
            f"Friendly stack here: {own}\n"
            f"Enemy stack here: {enemy}\n"
            f"Same-aspect harmony: {harmony}\n"
            f"Press bonus right now: +{press_bonus}\n"
            f"Rarity: {card.rarity}   Price: {price_label(card)}\n"
        )
        tk.Label(win, text=summary, justify="left", font=self.font_normal, padx=22).pack(anchor="w")

        button_frame = tk.Frame(win, padx=14, pady=10)
        button_frame.pack(fill="x")

        def choose(mode: ModeChoice) -> None:
            result["mode"] = mode
            win.destroy()

        self.intent_button(
            button_frame,
            "Press",
            "Use current friendly stack for an immediate bonus; add a normal stack token.",
            lambda: choose(ModeChoice("press", "Press")),
            row=0,
        )
        self.intent_button(
            button_frame,
            "Anchor",
            "No immediate Soul movement, but add stack value + intensity, with extra same-aspect growth.",
            lambda: choose(ModeChoice("anchor", "Anchor")),
            row=1,
        )
        self.intent_button(
            button_frame,
            "Sever",
            "Immediate effect -1, then cut enemy stack by stack value + intensity. Disruption glyphs cut +1.",
            lambda: choose(ModeChoice("sever", "Sever")),
            row=2,
        )

        release_box = tk.LabelFrame(win, text="Release: spend friendly stack before seeing the roll", padx=10, pady=8)
        release_box.pack(fill="x", padx=16, pady=(0, 12))
        if own <= 0:
            tk.Label(release_box, text="No friendly stack available to release.", font=self.font_normal).pack(anchor="w")
        else:
            tk.Label(
                release_box,
                text="Bonus after roll = min(5, ceil(spent stack × intensity / 4)). Choose spend:",
                font=self.font_normal,
            ).pack(anchor="w")
            spend_row = tk.Frame(release_box)
            spend_row.pack(anchor="w", pady=(6, 0))
            for spend in range(1, min(own, 6) + 1):
                tk.Button(
                    spend_row,
                    text=str(spend),
                    width=4,
                    command=lambda s=spend: choose(ModeChoice("release", f"Release {s}", s)),
                ).pack(side="left", padx=3)
            if own > 6:
                tk.Button(
                    spend_row,
                    text=f"All {own}",
                    width=7,
                    command=lambda s=own: choose(ModeChoice("release", f"Release {s}", s)),
                ).pack(side="left", padx=3)

        tk.Button(win, text="Cancel", command=win.destroy, width=14).pack(pady=(0, 12))
        win.protocol("WM_DELETE_WINDOW", win.destroy)
        self.center_window(win)
        self.root.wait_window(win)
        return result["mode"]

    def intent_button(self, parent: tk.Frame, title: str, detail: str, command, row: int) -> None:
        frame = tk.Frame(parent, relief="groove", borderwidth=1, padx=8, pady=5)
        frame.grid(row=row, column=0, sticky="ew", pady=4)
        parent.columnconfigure(0, weight=1)
        tk.Button(frame, text=title, width=12, command=command, font=("Segoe UI", 10, "bold")).pack(side="left", padx=(0, 8))
        tk.Label(frame, text=detail, justify="left", wraplength=520, font=self.font_normal).pack(side="left", anchor="w")

    def ask_player_reroll(self, card: Card, raw: int, intensity: int, mode: ModeChoice) -> Tuple[int, int]:
        actor = "Player 1" if self.is_hotseat() else "You"
        return self.ask_reroll_for_actor(card, raw, intensity, mode, actor, "player")

    def ask_opponent_human_reroll(self, card: Card, raw: int, intensity: int, mode: ModeChoice) -> Tuple[int, int]:
        return self.ask_reroll_for_actor(card, raw, intensity, mode, "Player 2", "opponent")

    def ask_reroll_for_actor(
        self,
        card: Card,
        raw: int,
        intensity: int,
        mode: ModeChoice,
        actor_label: str,
        will_owner: str,
    ) -> Tuple[int, int]:
        used_reroll = False
        result = {"raw": raw, "intensity": intensity}

        def get_will() -> int:
            return self.player_will if will_owner == "player" else self.ai_will

        def spend_will() -> None:
            if will_owner == "player":
                self.player_will = max(0, self.player_will - 1)
            else:
                self.ai_will = max(0, self.ai_will - 1)

        def close_keep() -> None:
            win.destroy()

        def do_reroll() -> None:
            nonlocal used_reroll
            if used_reroll or get_will() <= 0:
                return
            used_reroll = True
            spend_will()
            new_raw = self.rng.randint(1, 6)
            result["raw"] = new_raw
            result["intensity"] = intensity_from_d6(new_raw)
            self.refresh_status()
            roll_label.config(
                text=f"{card.emoji} {card.name}\nIntent: {mode.label}\nNew roll: {new_raw}\nIntensity: {result['intensity']}"
            )
            reroll_button.config(state="disabled", text="Rerolled")
            self.log(f"{actor_label} spends 1 Will to reroll {raw} → {new_raw}.")

        win = tk.Toplevel(self.root)
        win.title(f"{actor_label} Intensity Roll")
        win.transient(self.root)
        win.grab_set()
        win.resizable(False, False)

        tk.Label(win, text=f"{actor_label}: intensity roll", font=("Segoe UI", 15, "bold"), padx=24, pady=10).pack()

        roll_label = tk.Label(
            win,
            text=f"{card.emoji} {card.name}\nIntent: {mode.label}\nRoll: {raw}\nIntensity: {intensity}",
            font=("Segoe UI Emoji", 13),
            padx=24,
            pady=8,
        )
        roll_label.pack()

        tk.Label(
            win,
            text="1–2 = weak  •  3–4 = steady  •  5–6 = surge",
            font=("Segoe UI", 10),
            padx=24,
            pady=4,
        ).pack()

        button_row = tk.Frame(win, pady=12)
        button_row.pack()

        reroll_button = tk.Button(
            button_row,
            text=f"Reroll for 1 Will ({get_will()} available)",
            command=do_reroll,
            width=28,
        )
        reroll_button.grid(row=0, column=0, padx=6)
        if get_will() <= 0:
            reroll_button.config(state="disabled", text="No Will to reroll")

        tk.Button(button_row, text="Keep", command=close_keep, width=14).grid(row=0, column=1, padx=6)
        win.protocol("WM_DELETE_WINDOW", close_keep)
        self.center_window(win)
        self.root.wait_window(win)
        return result["raw"], result["intensity"]

    def center_window(self, win: tk.Toplevel) -> None:
        win.update_idletasks()
        x = self.root.winfo_x() + (self.root.winfo_width() // 2) - (win.winfo_width() // 2)
        y = self.root.winfo_y() + (self.root.winfo_height() // 2) - (win.winfo_height() // 2)
        win.geometry(f"+{max(0, x)}+{max(0, y)}")

    # ------------------------------------------------------------------
    # Resolution
    # ------------------------------------------------------------------

    def resolve_turn(self, player_card: Card, player_chamber: str, player_mode: ModeChoice) -> None:
        assert self.player_deck is not None
        assert self.ai_deck is not None

        player_raw = self.rng.randint(1, 6)
        player_intensity = intensity_from_d6(player_raw)
        player_raw, player_intensity = self.ask_player_reroll(player_card, player_raw, player_intensity, player_mode)

        if self.ai_committed_play is None:
            self.ai_committed_play = self.choose_ai_play(None, None, None)
        ai_card, ai_chamber, ai_mode = self.ai_committed_play
        self.ai_committed_play = None
        self.log(
            f"Computer reveals its blind committed play: {ai_card.short} → "
            f"{CHAMBER_EMOJI[ai_chamber]} {ai_chamber} as {ai_mode.label}."
        )
        ai_raw = self.rng.randint(1, 6)
        ai_intensity = intensity_from_d6(ai_raw)

        if self.ai_will > 0 and self.ai_should_reroll(ai_card, ai_chamber, ai_mode, ai_intensity, player_card, player_chamber):
            self.ai_will -= 1
            old_raw = ai_raw
            ai_raw = self.rng.randint(1, 6)
            ai_intensity = intensity_from_d6(ai_raw)
            self.log(f"Computer spends 1 Will to reroll {old_raw} → {ai_raw}.")

        if player_raw == 6 or ai_raw == 6:
            self.strain += int(player_raw == 6) + int(ai_raw == 6)

        player_play = self.make_pending_play(player_card, player_chamber, player_mode, player_raw, player_intensity)
        opponent_play = self.make_pending_play(ai_card, ai_chamber, ai_mode, ai_raw, ai_intensity)
        self.resolve_completed_plays(player_play, opponent_play)

    def resolve_hotseat_turn(self, player_card: Card, player_chamber: str, player_mode: ModeChoice) -> None:
        assert self.player_deck is not None
        assert self.ai_deck is not None

        player_raw = self.rng.randint(1, 6)
        player_intensity = intensity_from_d6(player_raw)
        player_raw, player_intensity = self.ask_player_reroll(player_card, player_raw, player_intensity, player_mode)

        second = self.ask_second_human_play()
        if second is None:
            self.in_resolution = False
            self.selected_var.set("Player 1 turn cancelled. Select a glyph, then click a Chamber.")
            return
        ai_card, ai_chamber, ai_mode, ai_raw, ai_intensity = second

        if player_raw == 6 or ai_raw == 6:
            self.strain += int(player_raw == 6) + int(ai_raw == 6)

        player_play = self.make_pending_play(player_card, player_chamber, player_mode, player_raw, player_intensity)
        opponent_play = self.make_pending_play(ai_card, ai_chamber, ai_mode, ai_raw, ai_intensity)
        self.resolve_completed_plays(player_play, opponent_play)

    def ask_second_human_play(self) -> Optional[Tuple[Card, str, ModeChoice, int, int]]:
        assert self.ai_deck is not None
        if not self.ai_deck.revealed:
            return None

        if self.is_secret_hotseat():
            self.mask_main_window_for_player2_secret_choice()
            if not self.show_secret_pass_screen():
                self.restore_public_reveals_after_secret_choice()
                return None
        else:
            messagebox.showinfo(
                "Player 2 turn",
                f"Pass the keyboard to Player 2 ({FACTION_LABEL[self.ai_faction]}).\n\n"
                "Player 2 will choose a revealed glyph, a valid Chamber, an Intent, then keep or reroll intensity.",
            )

        selected: Dict[str, Optional[object]] = {"card": None, "chamber": None}

        win = tk.Toplevel(self.root)
        win.title("Player 2 chooses a play" + (" — Secret Hot-seat" if self.is_secret_hotseat() else ""))
        win.transient(self.root)
        win.grab_set()
        win.geometry("720x520")

        tk.Label(
            win,
            text=f"Player 2: {FACTION_LABEL[self.ai_faction]}",
            font=("Segoe UI", 16, "bold"),
            padx=16,
            pady=10,
        ).pack(anchor="w")

        info_var = tk.StringVar(value="Choose a glyph.")
        tk.Label(win, textvariable=info_var, font=("Segoe UI Emoji", 11), padx=16, justify="left").pack(anchor="w")

        body = tk.Frame(win, padx=14, pady=8)
        body.pack(fill="both", expand=True)

        card_frame = tk.LabelFrame(body, text="Player 2 revealed glyphs", padx=8, pady=8)
        card_frame.pack(fill="x", pady=(0, 8))
        chamber_frame = tk.LabelFrame(body, text="Valid Chambers", padx=8, pady=8)
        chamber_frame.pack(fill="x")

        card_buttons: List[tk.Button] = []
        chamber_buttons: List[tk.Button] = []

        def redraw_chambers(card: Card) -> None:
            for child in chamber_frame.winfo_children():
                child.destroy()
            chamber_buttons.clear()
            selected["chamber"] = None
            for idx, chamber in enumerate(card.valid_chambers):
                locked = self.is_chamber_locked(card.faction, chamber)
                b = tk.Button(
                    chamber_frame,
                    text=f"{CHAMBER_EMOJI[chamber]} {chamber}" + (" 🔒" if locked else ""),
                    font=("Segoe UI Emoji", 12),
                    width=16,
                    state=("disabled" if locked else "normal"),
                    command=lambda ch=chamber: choose_chamber(ch),
                )
                b.grid(row=0, column=idx, padx=5, pady=4)
                chamber_buttons.append(b)

        def choose_card(card: Card, index: int) -> None:
            selected["card"] = card
            for b in card_buttons:
                b.configure(relief="raised", bg="SystemButtonFace")
            card_buttons[index].configure(relief="sunken", bg="#ffe8d8")
            info_var.set(f"Selected {card.short}. Now choose a Chamber.")
            redraw_chambers(card)

        def choose_chamber(chamber: str) -> None:
            selected["chamber"] = chamber
            for b in chamber_buttons:
                b.configure(relief="raised", bg="SystemButtonFace")
            for b in chamber_buttons:
                if chamber in b.cget("text"):
                    b.configure(relief="sunken", bg="#ffe8d8")
            card = selected["card"]
            if isinstance(card, Card):
                info_var.set(f"Selected {card.short} → {CHAMBER_EMOJI[chamber]} {chamber}. Click Continue.")

        for idx, key in enumerate(self.ai_deck.revealed):
            card = CARDS[key]
            b = tk.Button(
                card_frame,
                text=(
                    f"{card.emoji}  {card.name}  [{rarity_badge(card)}]\n"
                    f"{card.type_label}\n"
                    f"{self.card_chambers_label(card)}\n"
                    f"{card.aspect} • stack {card.stack_value} • {price_label(card)}"
                ),
                font=self.font_card,
                justify="center",
                width=24,
                height=6,
                command=lambda c=card, i=idx: choose_card(c, i),
            )
            b.grid(row=0, column=idx, padx=5, pady=3, sticky="nsew")
            card_frame.columnconfigure(idx, weight=1)
            card_buttons.append(b)

        action_row = tk.Frame(win, pady=12)
        action_row.pack(fill="x")

        result: Dict[str, bool] = {"ok": False}

        def cont() -> None:
            if not isinstance(selected["card"], Card) or not isinstance(selected["chamber"], str):
                messagebox.showwarning("Incomplete Player 2 play", "Player 2 must choose both a glyph and a Chamber.")
                return
            result["ok"] = True
            win.destroy()

        def cancel() -> None:
            result["ok"] = False
            win.destroy()

        tk.Button(action_row, text="Continue to Intent", command=cont, width=20).pack(side="right", padx=10)
        tk.Button(action_row, text="Cancel", command=cancel, width=12).pack(side="right")
        win.protocol("WM_DELETE_WINDOW", cancel)
        self.center_window(win)
        self.root.wait_window(win)

        if not result["ok"] or not isinstance(selected["card"], Card) or not isinstance(selected["chamber"], str):
            if self.is_secret_hotseat():
                self.restore_public_reveals_after_secret_choice()
            return None

        card = selected["card"]
        chamber = selected["chamber"]
        mode = self.ask_player_mode(card, chamber)
        if mode is None:
            if self.is_secret_hotseat():
                self.restore_public_reveals_after_secret_choice()
            return None

        raw = self.rng.randint(1, 6)
        intensity = intensity_from_d6(raw)
        raw, intensity = self.ask_opponent_human_reroll(card, raw, intensity, mode)
        if self.is_secret_hotseat():
            self.restore_public_reveals_after_secret_choice()
        return card, chamber, mode, raw, intensity

    def resolve_completed_plays(self, player_play: PendingPlay, opponent_play: PendingPlay) -> None:
        assert self.player_deck is not None
        assert self.ai_deck is not None

        first_name = self.player_actor_name()
        second_name = self.opponent_actor_name()

        self.log(
            f"{first_name} play {player_play.card.short} to {CHAMBER_EMOJI[player_play.chamber]} {player_play.chamber} "
            f"as {player_play.mode.label} (roll {player_play.raw_roll}, intensity {player_play.intensity})."
        )
        self.log(
            f"{second_name} play {opponent_play.card.short} to {CHAMBER_EMOJI[opponent_play.chamber]} {opponent_play.chamber} "
            f"as {opponent_play.mode.label} (roll {opponent_play.raw_roll}, intensity {opponent_play.intensity})."
        )

        self.player_deck.discard_revealed_card(player_play.card.key)
        self.ai_deck.discard_revealed_card(opponent_play.card.key)

        self.apply_pre_clash_modifiers(player_play, opponent_play)

        if player_play.chamber == opponent_play.chamber:
            self.log(
                f"Clash in {CHAMBER_EMOJI[player_play.chamber]} {player_play.chamber}: "
                f"{first_name} {player_play.power} vs {second_name} {opponent_play.power}."
            )
            if player_play.power > opponent_play.power:
                opponent_play.half = True
                self.log(f"{first_name} wins the clash. {second_name} effect is halved.")
            elif opponent_play.power > player_play.power:
                player_play.half = True
                self.log(f"{second_name} wins the clash. {first_name} effect is halved.")
            else:
                player_play.half = True
                opponent_play.half = True
                self.log("The clash is tied. Both effects are halved.")

        player_move = self.apply_play(player_play, is_player=True)
        opponent_move = self.apply_play(opponent_play, is_player=False)
        ongoing_move = self.apply_ongoing_effects()
        total_move = player_move + opponent_move + ongoing_move

        self.log(
            f"Round movement audit: {first_name} ({FACTION_SHORT[player_play.card.faction]}) {player_move:+d}; "
            f"{second_name} ({FACTION_SHORT[opponent_play.card.faction]}) {opponent_move:+d}; "
            f"ongoing board effects {ongoing_move:+d}; net {total_move:+d}."
        )

        uncapped_total_move = total_move
        capped_move = self.apply_movement_caps(total_move)
        if capped_move != total_move:
            self.log(f"Soul movement capped: {total_move:+d} becomes {capped_move:+d}.")
        total_move = capped_move

        actual_soul_delta = 0
        if total_move != 0:
            old = self.soul
            self.soul = max(-self.goal, min(self.goal, self.soul + total_move))
            actual_soul_delta = self.soul - old
            if actual_soul_delta != total_move:
                self.log(
                    f"Soul moves {actual_soul_delta:+d}: {old:+d} → {self.soul:+d} "
                    f"(track edge absorbed {total_move - actual_soul_delta:+d})."
                )
            else:
                self.log(f"Soul moves {total_move:+d}: {old:+d} → {self.soul:+d}.")
        else:
            self.log("The Soul does not move this round.")

        self.record_round_stats(
            player_play, opponent_play, player_move, opponent_move, ongoing_move, uncapped_total_move, actual_soul_delta
        )

        self.resolve_strain_events()
        self.refresh_status()
        self.refresh_board()
        self.check_winner()

        if not self.game_over:
            self.root.after(650, self.new_round)

    def make_pending_play(self, card: Card, chamber: str, mode: ModeChoice, raw: int, intensity: int) -> PendingPlay:
        power = card.base_power + intensity
        amount = card.base_move + intensity if card.base_move else 0
        stack_add = 0
        stack_spend = 0
        sever_amount = 0
        resonance_bonus = 0

        # Global support/event bonuses before intent math.
        if self.support_bonus[card.faction] > 0 and amount > 0:
            amount += self.support_bonus[card.faction]
            power += self.support_bonus[card.faction]
            self.log(f"{FACTION_LABEL[card.faction]} support adds +{self.support_bonus[card.faction]} to {card.short}.")
            self.support_bonus[card.faction] = 0

        if card.faction == ANGEL and amount > 0 and self.next_angel_bonus:
            amount += self.next_angel_bonus
            power += self.next_angel_bonus
            self.log(f"Reprieve bonus adds +{self.next_angel_bonus} to {card.short}.")
            self.next_angel_bonus = 0
        if card.faction == DEVIL and amount > 0 and self.next_devil_bonus:
            amount += self.next_devil_bonus
            power += self.next_devil_bonus
            self.log(f"Collapse bonus adds +{self.next_devil_bonus} to {card.short}.")
            self.next_devil_bonus = 0
        if amount > 0 and self.next_all_bonus:
            amount += self.next_all_bonus
            power += self.next_all_bonus
            self.log(f"Intensification adds +{self.next_all_bonus} to {card.short}.")
            self.next_all_bonus = 0

        # Card-specific modifiers.
        if card.special == "body_bonus" and chamber == "Body":
            amount += 1
            power += 1
        if card.special in ("behind_bonus", "screech"):
            if card.faction == ANGEL and self.soul < 0:
                amount += 1
                power += 1
            elif card.faction == DEVIL and self.soul > 0:
                amount += 1
                power += 1
            elif card.special == "screech" and self.soul < 0:
                amount += 1
                power += 1

        # Conditions.
        if card.faction == ANGEL and "Cold" in self.conditions[chamber] and card.key in {"candle", "small_comfort", "sweetness"}:
            power = max(0, power - 1)
            amount = max(0, amount - 1)
            self.log(f"Cold weakens {card.short} by 1 in {chamber}.")
        if card.faction == ANGEL and "Thorn" in self.conditions[chamber] and card.key == "loved_memory":
            power = max(0, power - 2)
            amount = max(0, amount - 2)
            self.conditions[chamber].remove("Thorn")
            self.log("Memory Thorn weakens Loved Memory by 2, then fades.")

        # v1.0 price keyword modifiers that affect immediate force without
        # changing the core Press/Anchor/Release/Sever grammar.
        if has_price(card, "Heavy"):
            if self.spend_will_for_faction(card.faction, 1):
                self.log(f"{card.short} is Heavy: {FACTION_SHORT[card.faction]} spends 1 Will to play it at full force.")
            else:
                power = max(0, power - 1)
                amount = max(0, amount - 1)
                self.log(f"{card.short} is Heavy, but {FACTION_SHORT[card.faction]} has no Will; it loses 1 force.")

        if has_price(card, "Desperate"):
            behind = (card.faction == ANGEL and self.soul < 0) or (card.faction == DEVIL and self.soul > 0)
            ahead = (card.faction == ANGEL and self.soul > 0) or (card.faction == DEVIL and self.soul < 0)
            if behind:
                amount += 1
                power += 1
                self.log(f"{card.short} is Desperate: +1 while {FACTION_SHORT[card.faction]} is behind.")
            elif ahead:
                amount = max(0, amount - 1)
                power = max(0, power - 1)
                self.log(f"{card.short} is Desperate: -1 while {FACTION_SHORT[card.faction]} is already ahead.")

        if has_price(card, "Volatile"):
            if raw <= 2:
                amount = max(0, amount - 2)
                power = max(0, power - 2)
                self.strain += 1
                self.log(f"{card.short} is Volatile: weak roll backfires, -2 force and +1 Strain.")
            elif raw >= 5:
                amount += 1
                power += 1
                self.log(f"{card.short} is Volatile: surge roll adds +1 force.")

        # Intent stack math.
        own_stack = self.stack_total(card.faction, chamber)
        enemy_stack = self.stack_total(opponent(card.faction), chamber)
        harmony = self.harmony_count(card.faction, chamber, card.aspect)

        if mode.key == "press":
            resonance_bonus = max(0, (own_stack + harmony - enemy_stack // 2) // 3)
            if resonance_bonus:
                amount += resonance_bonus
                power += resonance_bonus
            stack_add = card.stack_value
        elif mode.key == "anchor":
            # v0.5: Anchor is a true investment. It can still contest a clash,
            # but it does not move the Soul immediately.
            amount = 0
            power = max(0, power - 1)
            stack_add = card.stack_value + intensity + (1 if harmony > 0 else 0)
        elif mode.key == "release":
            stack_spend = min(mode.spend, own_stack)
            release_bonus = min(5, math.ceil((stack_spend * intensity) / 4))
            amount += release_bonus
            power += release_bonus
            resonance_bonus = release_bonus
            stack_add = 0
        elif mode.key == "sever":
            amount = max(0, amount - 1)
            power = max(0, power - 1)
            sever_amount = card.stack_value + intensity + (1 if is_disruptive(card) else 0)
            stack_add = max(1, card.stack_value - 1)
            if enemy_stack > 0:
                power += 1

        if has_price(card, "Hungry") and mode.key != "release":
            if own_stack > 0:
                stack_spend += 1
                amount += 1
                power += 1
                self.log(f"{card.short} is Hungry: consumes 1 friendly stack for +1 force.")
            else:
                amount = max(0, amount - 1)
                power = max(0, power - 1)
                self.log(f"{card.short} is Hungry but has no friendly stack to feed on; -1 force.")

        return PendingPlay(
            card=card,
            chamber=chamber,
            mode=mode,
            raw_roll=raw,
            intensity=intensity,
            power=power,
            amount=amount,
            stack_add=stack_add,
            stack_spend=stack_spend,
            sever_amount=sever_amount,
            resonance_bonus=resonance_bonus,
        )

    def apply_pre_clash_modifiers(self, player_play: PendingPlay, ai_play: PendingPlay) -> None:
        plays = [player_play, ai_play]

        for play in plays:
            opposing = opponent(play.card.faction)
            effects = list(self.board[play.chamber])
            for effect in effects:
                effect_card = effect.card

                if effect.faction == opposing and effect_card.guard:
                    play.power = max(0, play.power - 2)
                    play.amount = max(0, play.amount - 2)
                    self.log(f"{effect_card.short} guards {play.chamber}: {play.card.short} loses 2 power.")

                if effect.faction == opposing and effect_card.trap:
                    play.power = max(0, play.power - 3)
                    play.amount = max(0, play.amount - 3)
                    self.board[play.chamber].remove(effect)
                    self.log(f"{effect_card.short} springs in {play.chamber}: {play.card.short} loses 3 power.")

        if player_play.chamber == ai_play.chamber:
            if has_price(player_play.card, "Fragile"):
                player_play.power = max(0, player_play.power - 1)
                player_play.amount = max(0, player_play.amount - 1)
                self.log(f"{player_play.card.short} is Fragile and loses 1 force when contested.")
            if has_price(ai_play.card, "Fragile"):
                ai_play.power = max(0, ai_play.power - 1)
                ai_play.amount = max(0, ai_play.amount - 1)
                self.log(f"{ai_play.card.short} is Fragile and loses 1 force when contested.")
            if player_play.card.special == "calm":
                ai_play.power = max(0, ai_play.power - 2)
                ai_play.amount = max(0, ai_play.amount - 2)
                self.log(f"Calm Breath softens {FACTION_SHORT[ai_play.card.faction]} play by 2.")
            if ai_play.card.special == "calm":
                player_play.power = max(0, player_play.power - 2)
                player_play.amount = max(0, player_play.amount - 2)
                self.log(f"{ai_play.card.short} softens {FACTION_SHORT[player_play.card.faction]} play by 2.")
            if player_play.card.special == "tangle":
                ai_play.power = max(0, ai_play.power - 2)
                ai_play.amount = max(0, ai_play.amount - 2)
                self.log(f"Tangled Desire weakens {FACTION_SHORT[ai_play.card.faction]} play by 2.")
            if ai_play.card.special == "tangle":
                player_play.power = max(0, player_play.power - 2)
                player_play.amount = max(0, player_play.amount - 2)
                self.log(f"Tangled Desire weakens {FACTION_SHORT[player_play.card.faction]} play by 2.")

    def movement_sign(self, card: Card, movement_type: Optional[str] = None) -> int:
        """Return the signed Soul-track direction declared by a card.

        v1.0 used a blunt polarity guard: Angel cards could only score + and
        Devil cards could only score -. That was a useful bug shield, but it
        blocked intentional sacrifice/bait/cost mechanics. v1.0.1 resolves
        movement by declared type instead:

        - ownerward: toward the card owner's win condition
        - opponentward: intentionally toward the opponent as a cost or lure
        - heavenward: always +
        - hellward: always -
        - neutral: never moves the Soul
        """
        kind = movement_type or card.movement_type
        if kind not in MOVEMENT_TYPES:
            self.log(f"Unknown movement_type {kind!r} on {card.short}; treating as ownerward.")
            kind = MOVEMENT_OWNERWARD
        if kind == MOVEMENT_OWNERWARD:
            return FACTION_SIGN[card.faction]
        if kind == MOVEMENT_OPPONENTWARD:
            return -FACTION_SIGN[card.faction]
        if kind == MOVEMENT_HEAVENWARD:
            return FACTION_SIGN[ANGEL]
        if kind == MOVEMENT_HELLWARD:
            return FACTION_SIGN[DEVIL]
        return 0

    def signed_card_movement(self, card: Card, amount: int, movement_type: Optional[str] = None) -> int:
        """Build a signed movement value from a non-negative magnitude."""
        if amount <= 0:
            return 0
        sign = self.movement_sign(card, movement_type)
        return amount * sign

    def movement_guard(self, card: Card, move: int) -> int:
        """Validate signed movement against the card's declared movement type.

        Normal cards remain protected from accidental wrong-way movement. Special
        future cards can opt into opponentward/heavenward/hellward/neutral movement
        by declaring movement_type on the Card. This keeps bug protection without
        blocking intentional reverse movement costs.
        """
        if move == 0:
            return 0

        expected_sign = self.movement_sign(card)
        kind = card.movement_type if card.movement_type in MOVEMENT_TYPES else MOVEMENT_OWNERWARD

        if expected_sign == 0:
            self.log(
                f"Movement resolver blocked {card.short}: it is declared neutral but produced {move:+d}."
            )
            return 0

        if move * expected_sign < 0:
            corrected = abs(move) * expected_sign
            target = "Heaven" if corrected > 0 else "Hell"
            self.log(
                f"Movement resolver corrected {card.short}: declared {MOVEMENT_TYPE_LABEL[kind]} movement, "
                f"so {move:+d} becomes {corrected:+d} toward {target}."
            )
            return corrected

        if kind == MOVEMENT_OPPONENTWARD:
            target = "Heaven" if move > 0 else "Hell"
            self.log(f"{card.short} intentionally moves opponentward: {move:+d} toward {target}.")

        return move

    def apply_play(self, play: PendingPlay, is_player: bool) -> int:
        card = play.card
        chamber = play.chamber

        if play.power <= 0:
            self.log(f"{card.short} fizzles; its power was reduced to 0.")
            return 0

        amount = play.amount
        if play.half:
            amount = amount // 2

        if play.stack_spend:
            spent = self.consume_stack(card.faction, chamber, play.stack_spend)
            if spent:
                self.log(f"{FACTION_SHORT[card.faction]} releases {spent} stack from {chamber}.")

        if play.sever_amount:
            cut = self.cut_stack(opponent(card.faction), chamber, play.sever_amount)
            if cut:
                self.log(f"{card.short} severs {cut} enemy stack in {chamber}.")
            else:
                self.log(f"{card.short} tries to Sever, but finds no enemy stack in {chamber}.")

        move = 0

        if card.special == "miracle":
            move += self.resolve_miracle(play, amount)
        elif card.special == "serpent":
            move += self.resolve_serpent(play, amount)
        elif card.special == "bell":
            removed = self.remove_one_effect(chamber, DEVIL)
            if removed:
                self.log(f"Bell of Mercy cancels {removed.card.short} in {chamber}.")
            elif amount > 0:
                move += max(1, amount) * FACTION_SIGN[ANGEL]
                self.log("Bell of Mercy finds no ongoing Devil effect, so it rings as a Heaven push.")
            else:
                move += 1 * FACTION_SIGN[ANGEL]
                self.log("Bell of Mercy finds no ongoing Devil effect, so it nudges the Soul +1.")
        elif card.special == "calm":
            removed = self.remove_one_effect(chamber, DEVIL)
            if removed:
                self.log(f"Calm Breath cleanses {removed.card.short} from {chamber}.")
            elif amount > 0:
                move += amount * FACTION_SIGN[ANGEL]
        elif card.special == "rest":
            if amount > 0:
                move += amount * FACTION_SIGN[ANGEL]
            removed = self.remove_one_effect(chamber, DEVIL)
            if removed:
                self.log(f"Rest removes {removed.card.short} from {chamber}.")
            if "Cold" in self.conditions[chamber]:
                self.conditions[chamber].remove("Cold")
                self.log("Rest thaws Cold.")
        elif card.special == "loved_memory":
            if chamber == "Memory":
                removed = self.remove_one_effect(chamber, DEVIL)
                if removed:
                    self.log(f"Loved Memory dissolves {removed.card.short} in Memory.")
            if amount > 0:
                move += amount * FACTION_SIGN[ANGEL]
        elif card.special == "acceptance":
            removed = self.remove_one_effect(chamber, DEVIL)
            if removed:
                self.log(f"Acceptance releases {removed.card.short} from {chamber}.")
            if amount > 0:
                move += amount * FACTION_SIGN[ANGEL]
        elif card.special == "burden":
            if amount > 0:
                move += amount * FACTION_SIGN[DEVIL]
            self.apply_next_will_penalty(opponent(card.faction), 1)
            self.log("Burden reduces the opponent's Will next round by 1.")
        elif card.special == "watching_eye":
            move += max(1, amount) * FACTION_SIGN[DEVIL]
            self.apply_next_will_bonus(DEVIL, 1)
            self.log("Watching Eye gives Hell +1 Will next round.")
        elif card.special == "thorn":
            if amount > 0:
                move += amount * FACTION_SIGN[DEVIL]
            if "Thorn" not in self.conditions[chamber]:
                self.conditions[chamber].append("Thorn")
                self.log(f"Memory Thorn leaves Thorn in {chamber}.")
        elif card.special == "cold":
            if amount > 0:
                move += amount * FACTION_SIGN[DEVIL]
            if "Cold" not in self.conditions[chamber]:
                self.conditions[chamber].append("Cold")
                self.log(f"Cold Room leaves Cold in {chamber}.")
        elif card.special == "endless_wait":
            self.next_heaven_cap = 2
            if amount > 0:
                move += amount * FACTION_SIGN[DEVIL]
            self.log("Endless Wait will cap Heaven movement next round.")
        elif card.special == "support_next":
            if amount > 0:
                move += amount * FACTION_SIGN[ANGEL]
            self.support_bonus[ANGEL] += 1
            self.log("Small Comfort gives the next Heaven effect +1.")
        elif card.special == "sweetness":
            if amount > 0:
                move += amount * FACTION_SIGN[ANGEL]
            if self.soul > 0:
                self.apply_next_will_bonus(ANGEL, 1)
                self.log("Sweetness gives Heaven +1 Will next round.")
        else:
            if amount > 0:
                move += amount * FACTION_SIGN[card.faction]

        if has_price(card, "Delayed") and move != 0:
            self.next_delayed_movements.append((move, card.short))
            self.log(f"{card.short} is Delayed: {move:+d} is scheduled for next round instead of moving now.")
            move = 0

        if has_price(card, "Echo") and move != 0:
            echo_move = self.signed_card_movement(card, 1)
            self.next_delayed_movements.append((echo_move, f"Echo of {card.short}"))
            self.log(f"{card.short} Echo schedules {echo_move:+d} for next round.")

        move = self.movement_guard(card, move)

        if play.stack_add:
            self.add_stack(card.faction, chamber, card, play.stack_add)
            self.log(f"{FACTION_SHORT[card.faction]} adds {play.stack_add} stack to {chamber} from {card.emoji} {card.aspect}.")

        if has_price(card, "Binding"):
            self.lock_chamber_next_round(card.faction, chamber)
            self.log(f"{card.short} is Binding: {FACTION_SHORT[card.faction]} cannot use {chamber} next round.")

        if card.ongoing or card.trap or card.guard:
            self.place_board_effect(card, chamber)

        if move != 0:
            side_word = "Heaven" if move > 0 else "Hell"
            extra = f" Resonance bonus +{play.resonance_bonus}." if play.resonance_bonus else ""
            self.log(f"{card.short} contributes {move:+d} toward {side_word}.{extra}")
        elif card.ongoing or card.trap or card.guard:
            self.log(f"{card.short} takes hold in {CHAMBER_EMOJI[chamber]} {chamber}.")
        elif play.stack_add or play.sever_amount or play.stack_spend:
            self.log(f"{card.short} changes the stack state but has no direct Soul movement.")
        else:
            self.log(f"{card.short} has no direct Soul movement.")

        return move

    def resolve_miracle(self, play: PendingPlay, amount: int) -> int:
        roll = self.rng.randint(1, 6)
        bonus = max(0, amount - play.intensity)
        if roll <= 2:
            self.apply_next_will_bonus(ANGEL, 1)
            self.log(f"Little Miracle rolls {roll}: Heaven gains +1 Will next round.")
            return bonus
        if roll <= 4:
            self.log(f"Little Miracle rolls {roll}: a clean Heaven push.")
            return 2 + bonus
        self.log(f"Little Miracle rolls {roll}: blazing Heaven push.")
        return 4 + bonus

    def resolve_serpent(self, play: PendingPlay, amount: int) -> int:
        roll = self.rng.randint(1, 6)
        bonus = max(0, amount - play.intensity)
        if roll <= 2:
            self.apply_next_will_penalty(ANGEL, 1)
            self.log(f"Whispering Serpent rolls {roll}: Heaven loses 1 Will next round.")
            return -bonus
        if roll <= 4:
            self.log(f"Whispering Serpent rolls {roll}: a clean Hell push.")
            return -(2 + bonus)
        self.log(f"Whispering Serpent rolls {roll}: venomous Hell push.")
        return -(4 + bonus)

    def place_board_effect(self, card: Card, chamber: str) -> None:
        existing_same = [e for e in self.board[chamber] if e.faction == card.faction and e.card_key == card.key]
        if existing_same:
            self.log(f"{card.short} refreshes its presence in {chamber}.")
            return
        self.board[chamber].append(BoardEffect(faction=card.faction, card_key=card.key, chamber=chamber))

    def remove_one_effect(self, chamber: str, faction: str) -> Optional[BoardEffect]:
        for effect in list(self.board[chamber]):
            if effect.faction == faction:
                self.board[chamber].remove(effect)
                return effect
        if faction == DEVIL:
            for cond in ("Thorn", "Cold"):
                if cond in self.conditions[chamber]:
                    self.conditions[chamber].remove(cond)
                    self.log(f"{cond} is cleansed from {chamber}.")
                    return None
        return None

    def apply_ongoing_effects(self) -> int:
        total = 0
        for chamber in CHAMBERS:
            for effect in list(self.board[chamber]):
                card = effect.card
                if card.ongoing_move:
                    # Strong stacks slightly improve ongoing engines without letting them explode.
                    stack = self.stack_total(card.faction, chamber)
                    bonus = 1 if stack >= 6 else 0
                    move = self.signed_card_movement(card, card.ongoing_move + bonus)
                    total += move
                    self.log(f"Ongoing {card.short} in {chamber} contributes {move:+d}.")
                if card.special == "endless_wait":
                    self.next_heaven_cap = 2
        return total

    def apply_movement_caps(self, total_move: int) -> int:
        capped = total_move
        if self.next_heaven_cap is not None and capped > self.next_heaven_cap:
            capped = self.next_heaven_cap
        if self.next_soul_cap is not None:
            cap = self.next_soul_cap
            if capped > cap:
                capped = cap
            elif capped < -cap:
                capped = -cap
        self.next_heaven_cap = None
        self.next_soul_cap = None
        return capped

    def resolve_strain_events(self) -> None:
        while self.strain >= 3:
            self.strain -= 3
            event = self.rng.randint(1, 6)
            if event == 1:
                self.next_devil_bonus += 2
                self.log("Purgatory Event — 🕳️ Collapse: the next Hell push gets +2.")
            elif event == 2:
                self.next_angel_bonus += 2
                self.log("Purgatory Event — 🌤️ Reprieve: the next Heaven push gets +2.")
            elif event == 3:
                self.conditions["Memory"].clear()
                self.cut_stack(ANGEL, "Memory", 2)
                self.cut_stack(DEVIL, "Memory", 2)
                self.log("Purgatory Event — 🪞 Flashback: Memory conditions clear and both Memory stacks lose 2.")
            elif event == 4:
                self.next_soul_cap = 2
                self.log("Purgatory Event — 🕰️ Stasis: next round's Soul movement is capped at ±2.")
            elif event == 5:
                self.next_all_bonus += 1
                self.log("Purgatory Event — 🔥 Intensification: the next active effect gets +1.")
            else:
                if self.soul > 0:
                    self.apply_next_will_bonus(DEVIL, 2)
                    self.log("Purgatory Event — ⚖️ Judgement: Hell is behind and gets +2 Will next round.")
                elif self.soul < 0:
                    self.apply_next_will_bonus(ANGEL, 2)
                    self.log("Purgatory Event — ⚖️ Judgement: Heaven is behind and gets +2 Will next round.")
                else:
                    self.apply_next_will_bonus(ANGEL, 1)
                    self.apply_next_will_bonus(DEVIL, 1)
                    self.log("Purgatory Event — ⚖️ Judgement: both sides get +1 Will next round.")

    # ------------------------------------------------------------------
    # v1.0 price keyword helpers
    # ------------------------------------------------------------------

    def actor_key_for_faction(self, faction: str) -> str:
        return "player" if faction == self.player_faction else "opponent"

    def spend_will_for_faction(self, faction: str, amount: int) -> bool:
        if amount <= 0:
            return True
        if faction == self.player_faction:
            if self.player_will >= amount:
                self.player_will -= amount
                return True
            return False
        if self.ai_will >= amount:
            self.ai_will -= amount
            return True
        return False

    def is_chamber_locked(self, faction: str, chamber: str) -> bool:
        return chamber in self.chamber_locks.get(faction, set())

    def lock_chamber_next_round(self, faction: str, chamber: str) -> None:
        self.next_chamber_locks.setdefault(faction, set()).add(chamber)

    def apply_delayed_movements_at_round_start(self) -> None:
        if not self.delayed_movements:
            self.delayed_movements = list(self.next_delayed_movements)
            self.next_delayed_movements = []
        if not self.delayed_movements:
            return
        total = sum(move for move, _label in self.delayed_movements)
        labels = "; ".join(f"{label} {move:+d}" for move, label in self.delayed_movements)
        self.log(f"Delayed effects resolve at round start: {labels}; total {total:+d}.")
        old = self.soul
        capped = self.apply_movement_caps(total)
        self.soul = max(-self.goal, min(self.goal, self.soul + capped))
        actual = self.soul - old
        self.log(f"Delayed movement shifts the Soul {actual:+d}: {old:+d} → {self.soul:+d}.")
        self.delayed_movements = []
        self.refresh_status()
        self.refresh_board()
        self.check_winner()

    # ------------------------------------------------------------------
    # Stack operations
    # ------------------------------------------------------------------

    def stack_total(self, faction: str, chamber: str) -> int:
        return sum(token.value for token in self.stacks[faction][chamber])

    def harmony_count(self, faction: str, chamber: str, aspect: str) -> int:
        return sum(1 for token in self.stacks[faction][chamber] if token.aspect == aspect)

    def add_stack(self, faction: str, chamber: str, card: Card, value: int) -> None:
        if value <= 0:
            return
        tokens = self.stacks[faction][chamber]
        tokens.append(StackToken(faction=faction, card_key=card.key, aspect=card.aspect, value=value))
        while len(tokens) > MAX_VISIBLE_STACK_TOKENS:
            # Keep the UI readable. Merge the two oldest tokens of the same faction/chamber.
            first = tokens.pop(0)
            tokens[0].value += first.value

    def consume_stack(self, faction: str, chamber: str, amount: int) -> int:
        return self.remove_stack_value(faction, chamber, amount, newest_first=True)

    def cut_stack(self, faction: str, chamber: str, amount: int) -> int:
        return self.remove_stack_value(faction, chamber, amount, newest_first=False)

    def remove_stack_value(self, faction: str, chamber: str, amount: int, newest_first: bool) -> int:
        if amount <= 0:
            return 0
        tokens = self.stacks[faction][chamber]
        removed = 0
        while amount > 0 and tokens:
            idx = -1 if newest_first else 0
            token = tokens[idx]
            take = min(token.value, amount)
            token.value -= take
            amount -= take
            removed += take
            if token.value <= 0:
                tokens.pop(idx)
        return removed

    # ------------------------------------------------------------------
    # AI
    # ------------------------------------------------------------------

    def choose_ai_play(self, player_card: Optional[Card], player_chamber: Optional[str], player_mode: Optional[ModeChoice]) -> Tuple[Card, str, ModeChoice]:
        assert self.ai_deck is not None
        candidates: List[Tuple[float, Card, str, ModeChoice]] = []

        for key in self.ai_deck.revealed:
            card = CARDS[key]
            for chamber in card.valid_chambers:
                if self.is_chamber_locked(card.faction, chamber):
                    continue
                for mode in self.ai_modes_for(card, chamber):
                    score = self.score_ai_candidate(card, chamber, mode, player_card, player_chamber, player_mode)
                    candidates.append((score, card, chamber, mode))

        if not candidates:
            fallback = CARDS[self.ai_deck.revealed[0]]
            return fallback, fallback.valid_chambers[0], ModeChoice("press", "Press")

        candidates.sort(key=lambda x: x[0], reverse=True)
        if self.ai_profile.randomness <= 0.35:
            _, card, chamber, mode = candidates[0]
            return card, chamber, mode

        top_count = 2 if self.ai_profile.randomness < 1.0 else 4 if self.ai_profile.randomness < 2.0 else 7
        top = candidates[: min(top_count, len(candidates))]
        weights = [max(0.1, item[0] + 8.0) for item in top]
        _, card, chamber, mode = self.weighted_choice(top, weights)
        return card, chamber, mode

    def ai_modes_for(self, card: Card, chamber: str) -> List[ModeChoice]:
        own = self.stack_total(card.faction, chamber)
        enemy = self.stack_total(opponent(card.faction), chamber)
        modes = [ModeChoice("press", "Press"), ModeChoice("anchor", "Anchor"), ModeChoice("sever", "Sever")]
        if own > 0:
            spends = sorted(set([1, min(3, own), min(6, own), own]))
            for spend in spends:
                modes.append(ModeChoice("release", f"Release {spend}", spend))
        if enemy <= 0:
            # Sever is still legal, but bad AIs may try it. Serious AIs mostly won't score it well.
            pass
        return modes

    def score_ai_candidate(
        self,
        card: Card,
        chamber: str,
        mode: ModeChoice,
        player_card: Optional[Card],
        player_chamber: Optional[str],
        player_mode: Optional[ModeChoice],
    ) -> float:
        profile = self.ai_profile
        score = 0.0
        own_stack = self.stack_total(card.faction, chamber)
        enemy_stack = self.stack_total(opponent(card.faction), chamber)
        harmony = self.harmony_count(card.faction, chamber, card.aspect)
        expected_intensity = 2
        expected_amount = card.base_move + expected_intensity if card.base_move else 0

        if card.special == "body_bonus" and chamber == "Body":
            expected_amount += 1
        if card.special in {"behind_bonus", "screech"}:
            if card.faction == ANGEL and self.soul < 0:
                expected_amount += 1
            elif card.faction == DEVIL and self.soul > 0:
                expected_amount += 1
            elif card.special == "screech" and self.soul < 0:
                expected_amount += 1

        if has_price(card, "Heavy"):
            available_will = self.player_will if card.faction == self.player_faction else self.ai_will
            score -= 0.8 if available_will > 0 else 2.2
            if available_will <= 0:
                expected_amount = max(0, expected_amount - 1)
        if has_price(card, "Desperate"):
            behind = (card.faction == ANGEL and self.soul < 0) or (card.faction == DEVIL and self.soul > 0)
            ahead = (card.faction == ANGEL and self.soul > 0) or (card.faction == DEVIL and self.soul < 0)
            if behind:
                expected_amount += 1
                score += 1.0
            elif ahead:
                expected_amount = max(0, expected_amount - 1)
                score -= 0.8
        if has_price(card, "Volatile"):
            score += 0.35 if self.ai_profile.randomness >= 1.0 else -0.35
        if has_price(card, "Delayed"):
            score += profile.planning * 0.6
            if abs(self.soul) >= self.goal - 4:
                score -= 1.5
        if has_price(card, "Binding"):
            score -= 0.7
        if has_price(card, "Echo"):
            score += profile.planning * 0.35
        if has_price(card, "Hungry") and own_stack <= 0:
            expected_amount = max(0, expected_amount - 1)
            score -= 1.2

        if mode.key == "press":
            expected_amount += max(0, (own_stack + harmony - enemy_stack // 2) // 3)
            score += 0.8
        elif mode.key == "anchor":
            expected_amount = 0
            score += profile.planning * (1.2 + card.stack_value + (0.4 * harmony))
            if abs(self.soul) >= 8:
                score -= 2.0
        elif mode.key == "release":
            release_bonus = min(5, math.ceil((mode.spend * expected_intensity) / 4))
            expected_amount += release_bonus
            score += profile.release_hunger * (release_bonus + (1.5 if self.can_threaten_win(card.faction, expected_amount, card) else 0.0))
            if mode.spend >= own_stack and own_stack >= 5 and abs(self.soul) < 7:
                score -= 1.2
        elif mode.key == "sever":
            expected_amount = max(0, expected_amount - 1)
            cut = card.stack_value + expected_intensity + (1 if is_disruptive(card) else 0)
            score += profile.disruption * min(enemy_stack, cut)
            if enemy_stack <= 0:
                score -= 2.2

        movement_sign = self.movement_sign(card)
        if movement_sign == FACTION_SIGN[card.faction]:
            ownerward_expected_amount = expected_amount
        elif movement_sign == -FACTION_SIGN[card.faction]:
            ownerward_expected_amount = -expected_amount
        else:
            ownerward_expected_amount = 0
        score += profile.aggression * ownerward_expected_amount

        if card.ongoing or card.trap or card.guard:
            score += profile.planning * 1.4
        if card.special in {"serpent", "miracle"}:
            score += 1.0
        if is_disruptive(card):
            score += profile.disruption * 0.8

        same = player_chamber is not None and chamber == player_chamber
        if same:
            score += 1.0
            if player_mode is not None and mode.key == "sever" and player_mode.key in {"anchor", "press"}:
                score += profile.disruption * 1.6
            if card.special in {"false_door", "tangle", "calm"}:
                score += profile.disruption * 2.0

        if any(e.faction == opponent(card.faction) for e in self.board[chamber]):
            if card.special in {"calm", "bell", "rest", "loved_memory"}:
                score += profile.disruption * 2.5
            if card.trap or card.guard:
                score += profile.planning

        # Win/defense urgency.
        if card.faction == DEVIL:
            if self.soul <= -8 and ownerward_expected_amount > 0:
                score += 2.0
            if self.soul >= 6 and card.special in {"void_thought", "serpent", "false_door"}:
                score += 1.5
        else:
            if self.soul >= 8 and ownerward_expected_amount > 0:
                score += 2.0
            if self.soul <= -6 and card.special in {"behind_bonus", "miracle", "bell", "calm"}:
                score += 1.5

        # Thematic chamber nudges.
        if card.key in {"ache", "candle", "rest", "cold_room"} and chamber == "Body":
            score += 0.5
        if card.key in {"screech", "calm_breath", "hymn", "void_thought"} and chamber == "Mind":
            score += 0.5
        if card.key in {"loved_memory", "memory_thorn", "watching_eye"} and chamber == "Memory":
            score += 0.5
        if card.key in {"hope", "endless_wait", "burden", "dawn", "futility", "purpose", "weight"} and chamber == "Time":
            score += 0.5
        if card.key in {"shame_mirror", "obsession", "acceptance", "grace", "silver_thread"} and chamber == "Memory":
            score += 0.5

        score += self.rng.uniform(-profile.randomness, profile.randomness)
        return score

    def can_threaten_win(self, faction: str, amount: int, card: Optional[Card] = None) -> bool:
        if card is None:
            signed = amount * FACTION_SIGN[faction]
        else:
            signed = self.signed_card_movement(card, amount)
        projected = self.soul + signed
        return projected >= self.goal if faction == ANGEL else projected <= -self.goal

    def ai_should_reroll(
        self,
        ai_card: Card,
        ai_chamber: str,
        ai_mode: ModeChoice,
        ai_intensity: int,
        player_card: Card,
        player_chamber: str,
    ) -> bool:
        # Never reroll a surge. Usually reroll only weak rolls; Release plays may gamble
        # away a merely steady roll because spent stack is precious and explosive.
        if ai_intensity >= 3:
            return False
        if ai_intensity == 2 and ai_mode.key != "release":
            return False

        profile = self.ai_profile
        desire = profile.reroll_bias
        if ai_chamber == player_chamber:
            desire += 0.20
        if ai_mode.key == "release":
            desire += 0.30
        if ai_card.base_move > 0 and abs(self.soul) >= 7:
            desire += 0.25
        if ai_card.special in {"serpent", "miracle"}:
            desire += 0.15
        return self.rng.random() < min(0.95, desire)

    def weighted_choice(self, items: Sequence[Tuple[float, Card, str, ModeChoice]], weights: Sequence[float]) -> Tuple[float, Card, str, ModeChoice]:
        total = sum(weights)
        r = self.rng.uniform(0, total)
        upto = 0.0
        for item, weight in zip(items, weights):
            upto += weight
            if upto >= r:
                return item
        return items[-1]

    # ------------------------------------------------------------------
    # Status/rendering
    # ------------------------------------------------------------------

    def apply_next_will_bonus(self, faction: str, amount: int) -> None:
        if faction == self.player_faction:
            self.player_next_will_bonus += amount
        else:
            self.ai_next_will_bonus += amount

    def apply_next_will_penalty(self, faction: str, amount: int) -> None:
        if faction == self.player_faction:
            self.player_next_will_penalty += amount
        else:
            self.ai_next_will_penalty += amount

    def refresh_status(self) -> None:
        side = "Heaven" if self.soul > 0 else "Hell" if self.soul < 0 else "Balanced"
        self.soul_var.set(f"⚖️ Soul: {self.soul:+d} / ±{self.goal}    Current pull: {side}")
        if self.is_hotseat():
            self.will_var.set(
                f"Round {self.round_no}    Player 1 Will: {self.player_will}    Player 2 Will: {self.ai_will}    "
                f"Player 1: {FACTION_LABEL[self.player_faction]}    Player 2: {FACTION_LABEL[self.ai_faction]}"
            )
        else:
            self.will_var.set(
                f"Round {self.round_no}    Your Will: {self.player_will}    Computer Will: {self.ai_will}    "
                f"You: {FACTION_LABEL[self.player_faction]}    Computer: {FACTION_LABEL[self.ai_faction]}    AI: {self.ai_profile.label}"
            )
        self.strain_var.set(f"Strain: {self.strain}/3    Rule: Heaven scores +, Hell scores -")
        self.draw_soul_track()

    def draw_soul_track(self) -> None:
        if not hasattr(self, "soul_canvas"):
            return
        c = self.soul_canvas
        c.delete("all")
        width = max(700, c.winfo_width())
        height = 72
        margin = 46
        y = 34
        x0 = margin
        x1 = width - margin

        c.create_line(x0, y, x1, y, width=4)
        c.create_text(x0, y + 22, text="HELL", font=("Segoe UI", 9, "bold"))
        c.create_text(x1, y + 22, text="HEAVEN", font=("Segoe UI", 9, "bold"))

        for val in range(-self.goal, self.goal + 1, 2):
            x = self.value_to_x(val, x0, x1)
            c.create_line(x, y - 7, x, y + 7, width=1)
            c.create_text(x, y - 18, text=str(val), font=("Segoe UI", 7))

        zero_x = self.value_to_x(0, x0, x1)
        c.create_line(zero_x, y - 13, zero_x, y + 13, width=2)
        soul_x = self.value_to_x(self.soul, x0, x1)
        c.create_oval(soul_x - 11, y - 11, soul_x + 11, y + 11, fill="white", outline="black", width=2)
        c.create_text(soul_x, y, text="⚖️", font=("Segoe UI Emoji", 13))

    def value_to_x(self, value: int, x0: int, x1: int) -> float:
        return x0 + ((value + self.goal) / (2 * self.goal)) * (x1 - x0)

    def refresh_board(self) -> None:
        for chamber in CHAMBERS:
            angel_effects = [e.card.emoji for e in self.board[chamber] if e.faction == ANGEL]
            devil_effects = [e.card.emoji for e in self.board[chamber] if e.faction == DEVIL]
            conds = self.conditions[chamber]

            angel_line = " ".join(angel_effects) if angel_effects else "—"
            devil_line = " ".join(devil_effects) if devil_effects else "—"
            cond_line = " ".join(self.condition_emoji(c) for c in conds) if conds else "—"
            angel_stack = self.stack_display(ANGEL, chamber)
            devil_stack = self.stack_display(DEVIL, chamber)

            text = (
                f"Heaven effects:\n  {angel_line}\n"
                f"Heaven stack ({self.stack_total(ANGEL, chamber)}):\n  {angel_stack}\n\n"
                f"Condition:\n  {cond_line}\n\n"
                f"Hell effects:\n  {devil_line}\n"
                f"Hell stack ({self.stack_total(DEVIL, chamber)}):\n  {devil_stack}"
            )
            self.chamber_text_vars[chamber].set(text)

    def stack_display(self, faction: str, chamber: str) -> str:
        tokens = self.stacks[faction][chamber]
        if not tokens:
            return "—"
        parts = []
        for token in tokens[-MAX_VISIBLE_STACK_TOKENS:]:
            parts.append(f"{token.card.emoji}{token.value}")
        return " ".join(parts)

    def condition_emoji(self, condition: str) -> str:
        return {
            "Cold": "❄️ Cold",
            "Thorn": "🥀 Thorn",
        }.get(condition, condition)

    def check_winner(self) -> None:
        if self.soul >= self.goal:
            self.game_over = True
            self.in_resolution = True
            self.selected_var.set("Game over. Click New Game to play again.")
            self.log("")
            self.log("HEAVEN WINS — the Soul ascends.")
            self.refresh_status()
            self.disable_card_buttons()
            self.show_endgame_summary(ANGEL)
        elif self.soul <= -self.goal:
            self.game_over = True
            self.in_resolution = True
            self.selected_var.set("Game over. Click New Game to play again.")
            self.log("")
            self.log("HELL WINS — the Soul descends.")
            self.refresh_status()
            self.disable_card_buttons()
            self.show_endgame_summary(DEVIL)

    def disable_card_buttons(self) -> None:
        for b in self.card_buttons:
            b.config(state="disabled")
        for b in getattr(self, "chamber_buttons", []):
            b.config(state="disabled")

    def log_clear(self) -> None:
        self.log_lines = []
        if hasattr(self, "log_text"):
            self.log_text.configure(state="normal")
            self.log_text.delete("1.0", "end")
            self.log_text.configure(state="disabled")

    def log(self, msg: str) -> None:
        self.log_lines.append(msg)
        if not hasattr(self, "log_text"):
            return
        self.log_text.configure(state="normal")
        self.log_text.insert("end", msg + "\n")
        self.log_text.see("end")
        self.log_text.configure(state="disabled")


# -----------------------------------------------------------------------------
# Entrypoint
# -----------------------------------------------------------------------------

def main() -> int:
    try:
        root = tk.Tk()
    except tk.TclError as exc:
        print("Could not start Tkinter GUI.")
        print("On Linux, ensure a graphical display is available and python3-tk is installed.")
        print(f"Tk error: {exc}")
        return 1

    SoulscaleApp(root)
    root.mainloop()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
