# Rules changes: v1.0.1 → v1.0.2

**Short version:** the Rust engine runs the Python game exactly (rules **v1.0.1**, proven by 300
games / 214,033 log lines matching), plus a **v1.0.2** with four fixes. New games use v1.0.2; the
setup screen can still pick v1.0.1. Every saved match records which one it used.

The fixes came from cross-checking the Python source against the Unity-port handover notes. Each one
is a place where the code disagrees with the card text or with what the code was clearly meant to do.

## The four fixes

### 1. Screech (🔊, Hell): "stronger if the Soul is already below 0"

- **v1.0.1:** Screech shared the code path of the "behind" cards (Hope, Void Thought), with an extra
  clause for itself. The result was that Screech got +1 whenever the Soul was **anywhere except
  exactly 0**: below 0 because of its own clause, and above 0 because Hell counts as "behind" there.
- **v1.0.2:** +1 only while the Soul is below 0, as printed. The AI's scoring of Screech uses the same
  rule, so the computer no longer overrates it.

### 2. The computer's sense of urgency ignored the goal length

The AI has thresholds such as "the Soul is 8 from centre: go for the close" and "6 from centre on
my side: defend". They were tuned in v0.1–v0.4, when the goal was always ±12, so 8 meant "4 from
winning". They were never rescaled when ±18 and ±24 arrived. At ±24 the AI tried to close out with
the Soul a third of the way up, and started panic-defending 18 steps from losing.

- **v1.0.2:** each threshold keeps its original **distance from the goal**:
  `threshold = goal − (12 − original)`. At ±12 nothing changes; at ±18 "close out" moves from 8 to 14
  and "defend" from 6 to 12.
- One Unity handover (D2) assumed the original goal was ±10. The Python history shows ±12; the fix
  uses 12.

### 3. Void Thought never counted as a Hell defender

The AI gives a defence bonus to Hell cards that protect against Heaven pushes. The Python list named
a special ability (`void_thought`) that no card actually has, so Void Thought never got the bonus.
**v1.0.2** checks the card itself.

### 4. Silver Thread's log line named the wrong card

Silver Thread (🧵, Heaven support) logged "Small Comfort gives the next Heaven effect +1." The effect
was right; only the text was wrong. **v1.0.2** names the card that was played.

## Not bugs in the Python (Unity-port artefacts)

The handover notes list several things as broken that are only broken in the Unity C# port:

- The "unwired specials" list: every special is implemented in the Python and in this Rust port.
- The Unity port lost Hope's and Void Thought's behind-bonus. The Python has it and so does this port.

## Open design questions (unchanged, your call)

1. **Hope and Void Thought get +2 when behind.** They have the "behind" special (+1) and the
   Desperate price (+1 behind, −1 ahead), and the two stack. Either that's the intended identity of a
   comeback card, or one of the two was meant to replace the other. If it should be +1, it's a
   one-line change in `rules/pending.rs` (and the matching line in `ai.rs`).
2. **Deck balance** is uneven; see below. Nothing has been changed, because the numbers come from
   AI-vs-AI play and need confirming with people.
3. **Endless Wait** caps Heaven's movement in the round it's played and every round it stays on the
   board (the cap is set during that round's resolution and used straight away). The card text says
   "next round". The Python does the same.
4. **Stasis** is used up by Delayed or Echo movement at the start of the next round, so the main
   movement that round is uncapped.
5. **One-shot bonuses** (Small Comfort, Silver Thread, Reprieve, Collapse, Intensification) are used up
   by a play that then Anchors, which wastes them. Intensification always goes to Player 1's play
   first.
6. **Candle of Warmth** is typed "Direct / Guard" but places no guard.

The full rulebook (`docs/RULEBOOK.md`) describes all of these as they actually play.

## Simulator results

200 games per matchup (half with each side moving first), goal ±18, Balanced AI on both sides,
seed 1. Reproduce with:

```powershell
cargo run --release -p soulscale-sim -- compare --games 200 --goal 18
```

Heaven win % of decided games. Rows are Heaven decks, columns are Hell decks.

**v1.0.2**

| Heaven ↓ / Hell → | Starter Torment | Pain Engine | Despair Control | Trickster Hell | Fever Break | Memory Shame | Avg |
|---|---|---|---|---|---|---|---|
| Starter Mercy | 72.0 | 66.0 | 79.5 | 99.5 | 54.5 | 67.0 | 73.1 |
| Comfort Engine | 84.5 | 81.5 | 91.0 | 100.0 | 71.5 | 50.0 | 79.8 |
| Sanctuary Control | 95.5 | 88.5 | 95.5 | 98.5 | 84.5 | 85.5 | **91.3** |
| Hope From Behind | 1.5 | 1.5 | 3.5 | 87.0 | 1.0 | 4.5 | **12.2** |
| Memory Loom | 1.0 | 0.0 | 8.5 | 88.4 | 0.0 | 2.5 | **13.4** |
| Delayed Dawn | 78.0 | 76.0 | 91.0 | 97.0 | 69.5 | 36.0 | 74.6 |

Overall Heaven 58.2%, average 54.5 rounds, 126 stalemates in 7,200 games.
(v1.0.1: Heaven 57.6%, 54.0 rounds, 76 stalemates.)

What it suggests:

- **Sanctuary Control** wins about 9 in 10 against everything. It's the outlier on the Heaven side.
- **Hope From Behind** and **Memory Loom** almost never win except against Trickster Hell. Either
  the decks are weak or the AI can't play them (both rely on falling behind or building up first,
  which a greedy AI is bad at). Test them by hand before changing cards.
- **Trickster Hell** loses to nearly every Heaven deck. **Memory Shame** is the strongest Hell deck.
- Heaven wins a bit more than Hell overall in this pairing of decks.
- The AI fix (2) changed little overall but shifted some matchups by 5–10 points. Stalemates went up
  slightly (76 → 126 of 7,200), probably because the AI now saves its closing plays until it's
  genuinely near the goal.

The per-matchup v1.0.1 table and the difference table print with the same command.
