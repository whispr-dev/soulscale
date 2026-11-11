# Testing Soulscale

Five layers, fastest first. Run the first one after any change; run the UI smoke test before
putting a build in front of people.

## 1. Rust tests (seconds)

```powershell
cargo test --workspace
```

| Test file | What it proves |
|---|---|
| `core/tests/rng_parity.rs` | The random number generator matches CPython's `random` exactly (seeding, `randint`, `random`, `shuffle`, `choice`), checked against numbers produced by real Python. |
| `core/tests/python_parity.rs` | **The port is faithful.** 300 games played by the original Python v1.0.1 engine are replayed in Rust with the same seeds, decks, AI personalities and modes; all 214,033 log lines must match exactly. |
| `core/tests/replay_and_rules.rs` | Replays reproduce games in every mode; the rulebook's §19 walkthrough; Duel secrecy at the session level; illegal moves are refused. |
| `core/tests/rules_v102.rs` | Each v1.0.2 fix does what it says, and changes nothing else (v1.0.1 and v1.0.2 agree at goal ±12 when no fixed card is involved). |
| `core/src/**` unit tests | Card data loads and validates, deck legality and parsing, reveal/discard, Press/Release/Anchor/Sever maths, RNG streams. |
| `server/tests/ws_e2e.rs` | Real WebSocket clients against a real server: a vs-Computer game to the end with files saved; a Duel where neither player nor a spectator ever receives private data; a duelist who drops is covered by the AI. |

`cargo clippy --workspace --all-targets` should print nothing.

## 2. Python parity: regenerate after changing the reference

Only needed if you edit the original Python file in `tools/py_harness/reference/`.
Python 3.11+, no Tkinter needed (it's stubbed):

```powershell
py -I tools\py_harness\run_parity.py tools\py_harness\reference\soulscale_purgatory_v101.py crates\soulscale-core\tests\fixtures\python_games.jsonl.gz --games 300
cargo test -p soulscale-core --test python_parity
```

The harness is deterministic: the same Python file always produces the same fixture byte for byte.

## 3. UI smoke test (a few minutes)

Drives real Chromium tabs through the actual interface: every screen, full games against the
computer, a Duel between two tabs with a third tab spectating, a secret Pass & Play game, and phone
layouts in portrait and landscape. It fails on any JavaScript error, any action the server rejects,
any private data reaching the wrong tab, or any sideways scrolling on a phone. Screenshots are saved
for you to eyeball.

```powershell
py -m pip install playwright
py -m playwright install chromium
powershell -ExecutionPolicy Bypass -File tools\run.ps1 -NoBrowser     # terminal 1
py tools\qa\ui_smoke.py --out qa-shots                                  # terminal 2
py tools\qa\ui_smoke.py --headed --only duel                            # watch it play
py tools\qa\ui_smoke.py --games 5                                       # more AI games
```

Use `--channel chrome` or `--channel msedge` to use an installed browser instead of downloading
Chromium. The server must have browser test logins on (the default when `DISCORD_CLIENT_SECRET` is
empty, or `SOULSCALE_DEV_LOGIN=on`).

## 4. By hand in the browser

`tools\run.ps1` opens `http://127.0.0.1:3001/?name=Wofl`. The URL decides who you are and which
table you're at: `?name=Bob` is a different player, and `&room=xyz` is a different table
(default `local`).

Checklist worth doing once per release:

- [ ] Each AI personality, each goal length, both sides.
- [ ] Duel: tab A opens it, tab B (`?name=Bob`) accepts, tab C (`?name=Carol`) watches. Carol and
      Bob must see card backs for each other's hands until plays are revealed.
- [ ] Close tab B mid-Duel. After 30 s the AI takes the seat (notice in tab A). Reopen it: Bob gets
      the seat back.
- [ ] In a Duel, don't act for 2 minutes: the AI plays that round for you.
- [ ] Pass & Play with Secret on: the curtain hides each hand until "I'm Player N".
- [ ] Rules v1.0.1 in setup: the Chronicle's first line says v1.0.1.
- [ ] Deck Builder: build and save a deck, then pick it in the vs Computer setup.
- [ ] Sound Lab: remap a sound, hear it in game, Reset, Export manifest.
- [ ] Settings: volumes, speed 1×/2×/4×, reduced motion (fewer particles, no shake or tilt), Chronicle on/off.
- [ ] Export (⬇): report text, rounds CSV and replay download. (Inside Discord the frame may block
      downloads and the clipboard; the server keeps the same three files in `data\replays\` anyway.)
- [ ] Narrow the window to phone width: portrait layout, no sideways scrolling.

## 5. Balance simulator

AI-vs-AI games, thousands per minute, all cores. Reproducible: the same `--seed` gives the same
numbers.

```powershell
cargo run --release -p soulscale-sim -- matrix --games 200 --goal 18
cargo run --release -p soulscale-sim -- compare --games 200 --goal 18      # v1.0.1 vs v1.0.2
cargo run --release -p soulscale-sim -- matrix --profile "Cruel Accountant / Hard" --csv matrix.csv
cargo run --release -p soulscale-sim -- game --seed 42                     # one game's full log
cargo run --release -p soulscale-sim -- profiles                           # personalities and decks
```

Read the matrix as "Heaven deck (row) beats Hell deck (column) this % of decided games".
Stalemates (no winner by `--max-rounds`, default 400) are counted separately.
[RULES_CHANGES.md](RULES_CHANGES.md) has the current numbers and what they suggest.

Remember the simulator measures **how the AI plays the decks**, not how people do. A deck that
needs patience (Hope From Behind) may look worse than it is because the AI plays it badly.

## 6. Replays

Every finished match is saved to `data\replays\` as three files with the same name:

- `.txt`: the full Chronicle plus the endgame summary (the same text as Export)
- `.rounds.csv`: one row per round (opens in Excel)
- `.replay.json`: seed, decks, settings and every input, enough to recreate the match

```powershell
cargo run --release -p soulscale-sim -- replay data\replays\<name>.replay.json
```

It re-runs the match from its inputs and confirms the log comes out identical. If a player reports
something odd, ask for the time of the match and replay it: you'll see exactly what the engine did.
