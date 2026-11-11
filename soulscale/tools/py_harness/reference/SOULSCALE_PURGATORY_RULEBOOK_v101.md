# Soulscale: Purgatory — Definitive Rulebook

**Rules version:** v1.0.1 — Movement Resolver  
**Document status:** first full playtest rulebook draft  
**Last updated:** 2026-05-07

---

## 1. Game Overview

**Soulscale: Purgatory** is a compact two-sided tactical loadout game about a single human Soul caught between Heaven and Hell.

One player controls **Heaven / Angel** and tries to move the Soul upward into bliss and ascension. The other controls **Hell / Devil** and tries to move the Soul downward into suffering and descent.

The game is built around four linked decisions:

1. **Deck construction** — choose which glyphs your side can access.
2. **Reveal luck** — each round, only part of your deck is available.
3. **Placement** — choose which Chamber of the Soul your glyph affects.
4. **Intent** — choose whether to Press, Anchor, Release, or Sever Resonance.

The core tactical question is:

> Do you score now, build for later, cash out a prepared Chamber, or cut the opponent's setup?

---

## 2. Components

A complete digital game contains:

- 1 shared **Soul Track**
- 5 **Chambers**
- 2 player decks, one Heaven and one Hell
- revealed glyph rows for both players
- Will counters
- Resonance stacks for each side in each Chamber
- board effects, traps, guards, conditions, delayed effects, and log/export tools

For tabletop/proxy play, use:

- one marker for the Soul Track
- paper/card glyphs or tokens
- five labelled Chamber spaces
- counters for Will and Resonance
- a d6 for intensity rolls

---

## 3. Objective

The Soul starts at **0**.

Heaven wins when the Soul reaches the positive goal.

Hell wins when the Soul reaches the negative goal.

Game length options:

| Mode | Goal |
|---|---:|
| Quick | ±12 |
| Standard | ±18 |
| Deep | ±24 |

The current recommended default is **Standard ±18**.

---

## 4. The Soul Track

The Soul Track is a single shared number.

```text
HELL  -24 ... -18 ... -12 ... -1   0   +1 ... +12 ... +18 ... +24  HEAVEN
```

- Positive movement helps Heaven.
- Negative movement helps Hell.
- Round effects are summed before the Soul moves.
- If movement would pass the goal, the track edge absorbs the excess.
- Some effects can cap movement for a round.

Example:

```text
Heaven active play: +4
Hell active play:   -2
Ongoing effects:    -1
Net movement:       +1
Soul: 0 → +1
```

---

## 5. Sides

## 5.1 Heaven / Angel

Heaven uses comfort, mercy, warmth, peace, memory, sanctuary, and rescue.

Heaven movement is normally positive.

## 5.2 Hell / Devil

Hell uses pain, noise, waiting, dread, shame, burden, obsession, and despair.

Hell movement is normally negative.

---

## 6. Chambers

All played glyphs are placed into one of five Chambers.

| Chamber | Emoji | Represents |
|---|---|---|
| Body | 🫀 | pain, warmth, exhaustion, comfort, sickness |
| Mind | 🧠 | noise, calm, panic, focus, confusion |
| Heart | ❤️ | love, loneliness, desire, despair, mercy |
| Time | 🕰️ | waiting, patience, delay, urgency, futility |
| Memory | 🪞 | regret, nostalgia, shame, forgiveness, identity |

Each card lists which Chambers it may legally target.

---

## 7. Deck Construction

Each player uses a deck of exactly **10 glyphs**.

Each round, a player reveals **4 glyphs** from their deck.

## 7.1 Rarity

Each glyph has a rarity.

| Rarity | Cost |
|---|---:|
| Common | 1 |
| Uncommon | 2 |
| Rare | 3 |

A legal v1.0.1 deck must obey:

```text
Deck size: exactly 10 glyphs
Rarity budget: 24 or less
Rare limit: 4 or less
```

Duplicates are allowed unless a later mode explicitly forbids them.

## 7.2 Custom Deck Files

A custom deck is a UTF-8 `.txt` file with one emoji per usable line.

Blank lines and lines beginning with `#` are ignored.

A v1.0.1 custom deck should contain exactly 10 glyphs.

Legacy 8-glyph decks are accepted by the digital version and auto-padded with stable common glyphs before validation.

Example:

```text
🕯️
🤲
🛏️
🎵
🪶
🫖
🌤️
🧸
🍯
💌
```

---

## 8. Setup

1. Choose game mode:
   - Human vs Computer
   - Human vs Human / hot-seat
   - Human vs Human / secret hot-seat
2. Player 1 chooses a side:
   - Heaven / Angel
   - Hell / Devil
3. The opponent takes the other side.
4. Choose game length:
   - Quick ±12
   - Standard ±18
   - Deep ±24
5. Choose or build a 10-glyph deck for each side.
6. Set the Soul to 0.
7. Clear all Chambers, stacks, conditions, delayed effects, and board effects.
8. Begin Round 1.

---

## 9. Round Structure

A round contains one play from each side, followed by simultaneous resolution.

## 9.1 Start of Round

At the start of each round:

1. Advance the round counter.
2. Set each side's Will:
   ```text
   Will = 3 + next Will bonuses - next Will penalties
   minimum 0
   ```
3. Apply Chamber locks from the previous round.
4. Resolve delayed movements from previous rounds.
5. Check for victory from delayed movement.
6. Reveal 4 glyphs for each player.
7. In Human vs Computer mode, the AI secretly commits its glyph, Chamber, and Intent before the human chooses.

## 9.2 Player Choice

On their turn, a player chooses:

1. one revealed glyph
2. one legal Chamber
3. one Intent:
   - Press
   - Anchor
   - Release
   - Sever

In open hot-seat, both players can see available reveals.

In secret hot-seat:

- Player 2's reveal is hidden from Player 1.
- Player 1's committed play is hidden before Player 2 chooses.
- Both plays are revealed together after Player 2 commits.

## 9.3 Intensity Roll

Each played glyph rolls one d6.

| d6 | Intensity |
|---:|---:|
| 1–2 | 1 |
| 3–4 | 2 |
| 5–6 | 3 |

A player may spend **1 Will** to reroll their intensity roll once.

The rerolled result is final.

The AI may reroll depending on its personality profile and available Will.

Each raw roll of **6** adds 1 Strain.

## 9.4 Build Pending Plays

After cards, Chambers, Intents, rolls, and rerolls are known, each play calculates:

```text
Power  = base_power + intensity + modifiers
Amount = base_move + intensity + modifiers, if the card has base movement
```

- **Power** decides clashes.
- **Amount** decides how much movement or effect magnitude the card contributes.
- Some cards have base movement 0 and therefore do not score directly unless their special effect says otherwise.

## 9.5 Pre-Clash Modifiers

Before clashes are checked, ongoing board effects and keywords may modify plays.

Examples:

- Guard effects reduce opposing plays in their Chamber.
- Trap effects reduce the next opposing play in their Chamber, then vanish.
- Fragile cards lose force when contested.
- Calm Breath and Tangled Desire can weaken opposing plays in the same Chamber.

## 9.6 Clash

If both players choose the same Chamber, they clash.

Compare final Power:

| Result | Rule |
|---|---|
| Player A higher | Player B's Amount is halved |
| Player B higher | Player A's Amount is halved |
| Tie | both Amounts are halved |

Halving rounds down.

Power determines the clash. Amount is what gets halved.

## 9.7 Apply Plays

After the clash:

1. Apply Player 1's play.
2. Apply Player 2 / opponent's play.
3. Spend or cut stacks as required.
4. Apply card special effects.
5. Add new Resonance stacks.
6. Place ongoing, guard, or trap effects.
7. Schedule Echo or Delayed movement if applicable.
8. Apply Binding locks for next round.

## 9.8 Apply Ongoing Effects

After both active plays are applied, ongoing effects trigger.

Ongoing effects can contribute movement in the same round they are placed.

If an ongoing effect is in a Chamber where its side has at least 6 friendly stack, its ongoing movement gains +1.

## 9.9 Move the Soul

Add together:

```text
Player 1 active movement
Player 2 active movement
Ongoing board movement
```

Then apply movement caps.

Then move the Soul.

## 9.10 Strain and Purgatory Events

After Soul movement, resolve Strain.

Whenever Strain reaches 3, spend 3 Strain and roll a Purgatory Event.

If Strain is 6 or more, multiple events may occur.

## 9.11 Victory Check

If the Soul reaches the positive goal, Heaven wins.

If the Soul reaches the negative goal, Hell wins.

If nobody has won, begin the next round.

---

## 10. Will

Will is the tactical resource used for rerolls and some card costs.

At the start of each round, each side normally receives **3 Will**.

Will can be modified by cards and events.

Current common Will uses:

- spend 1 Will to reroll intensity once
- Heavy cards automatically spend 1 Will if available
- certain cards add or subtract Will next round

Unused Will does not carry over into the next round in the current digital rules.

---

## 11. Intents

Every played glyph uses one Intent.

## 11.1 Press

Press uses existing friendly Resonance for immediate value.

Formula:

```text
Press bonus = floor((friendly stack + same-aspect harmony - floor(enemy stack / 2)) / 3)
minimum 0
```

Press then adds the card's normal stack value to that Chamber.

Use Press when:

- you want reliable movement now
- you already have some friendly stack
- you do not want to commit to a bigger setup play

## 11.2 Anchor

Anchor is pure investment.

Rules:

```text
Immediate Soul movement = 0
Power is reduced by 1
Stack added = card stack value + intensity + 1 if same-aspect harmony exists
```

Anchor can still contest a clash because it still has Power, but it does not move the Soul directly.

Use Anchor when:

- you want a stronger future Release
- you want to establish Chamber identity
- you can afford a low-scoring round

## 11.3 Release

Release spends friendly stack in the chosen Chamber for a burst.

The stack spend is chosen before seeing or using the final intensity result.

Formula:

```text
Release bonus = min(5, ceil(spent stack × intensity / 4))
```

Release adds this bonus to both movement Amount and clash Power.

Release does not add new stack.

Use Release when:

- you have prepared a Chamber
- the current reveal gives you a good payoff card
- you need a swing round

## 11.4 Sever

Sever attacks the opponent's Resonance.

Rules:

```text
Immediate Amount -1
Power -1
Sever cut = card stack value + intensity + 1 if the card is disruptive
Friendly stack added = max(1, card stack value - 1)
```

If there is enemy stack in the Chamber, Sever gains +1 Power.

Use Sever when:

- the opponent is preparing a Release
- you need to break a stack engine
- your card is naturally disruptive or trap-like

---

## 12. Resonance Stacks

Each Chamber can hold Heaven stack and Hell stack.

A stack token has:

```text
Faction
Card source
Aspect
Value
```

Stacks do not directly move the Soul by themselves.

Stacks matter because they:

- increase Press bonuses
- fuel Release
- can be cut by Sever
- improve ongoing engines at 6+ friendly stack
- create same-aspect harmony for certain formulas

## 12.1 Friendly Stack

The total value of all your stack tokens in that Chamber.

## 12.2 Enemy Stack

The total value of the opponent's stack tokens in that Chamber.

## 12.3 Same-Aspect Harmony

Harmony counts friendly stack tokens in that Chamber with the same aspect as the played card.

Harmony improves Press and Anchor.

## 12.4 Stack Removal

The digital version removes stack as follows:

- Release / Hungry consumes friendly stack.
- Sever cuts enemy stack.
- Stack is removed by value, not necessarily by whole token.
- The visual UI may merge old stack tokens for readability.

---

## 13. Movement Resolver

v1.0.1 uses a rules-aware movement resolver.

Each card declares a movement type.

| Movement Type | Meaning |
|---|---|
| ownerward | movement goes toward the card owner's win condition |
| opponentward | movement intentionally goes toward the opponent as a cost, bait, or sacrifice |
| heavenward | movement is always positive |
| hellward | movement is always negative |
| neutral | the card cannot move the Soul |

Current live cards are mostly ownerward, but the engine now supports future sacrifice and bait cards.

The movement resolver protects normal cards from accidental wrong-way movement while allowing declared reverse movement.

Example future mechanics now possible:

```text
Devil card: move +2 toward Heaven, then add a Hell trap.
Angel card: move -2 toward Hell, then cleanse multiple effects.
```

---

## 14. Price Keywords

Price Keywords are tactical costs or constraints that allow stronger or stranger cards without simply increasing numbers.

## 14.1 Heavy

A Heavy card automatically spends 1 Will from its side if possible.

If its side cannot pay, the card loses 1 force.

## 14.2 Fragile

If a Fragile card is contested in the same Chamber by the opponent's play, it loses 1 force.

## 14.3 Volatile

Volatile cards care more about the raw d6 roll.

| Raw Roll | Effect |
|---:|---|
| 1–2 | loses 2 force and adds +1 Strain |
| 3–4 | no extra effect |
| 5–6 | gains +1 force |

## 14.4 Delayed

If a Delayed card would move the Soul, that movement is scheduled for the start of the next round instead of happening immediately.

Other non-movement effects still occur normally.

## 14.5 Binding

After a Binding card is played, its side cannot use that same Chamber next round.

## 14.6 Hungry

If the card is not using Release:

- if friendly stack exists in the Chamber, consume 1 friendly stack and gain +1 force
- if no friendly stack exists, lose 1 force

Hungry does not trigger during Release.

## 14.7 Echo

If an Echo card moves the Soul, it schedules a weak +1 or -1 repeat movement for the next round.

The Echo movement uses the card's movement type.

## 14.8 Desperate

A Desperate card is stronger while its side is behind and weaker while its side is ahead.

| Side | Behind means | Ahead means |
|---|---|---|
| Heaven | Soul below 0 | Soul above 0 |
| Hell | Soul above 0 | Soul below 0 |

Behind: +1 force.  
Ahead: -1 force.

## 14.9 Narrow

A Narrow card is restricted to its listed Chamber or Chambers.

It has no extra math by itself; the limitation is the cost.

---

## 15. Board Effects

Some cards remain in a Chamber after they are played.

## 15.1 Ongoing

Ongoing cards persist and contribute their ongoing movement after active plays resolve each round.

An ongoing effect with friendly stack 6+ in its Chamber gets +1 ongoing movement.

## 15.2 Guard

Guard effects reduce opposing plays in their Chamber by 2 force before clash resolution.

Guardian and Sanctuary are current examples.

## 15.3 Trap

Trap effects reduce the next opposing play in their Chamber by 3 force, then vanish.

False Door and Snare are current examples.

## 15.4 Refreshing

If the same side plays the same ongoing/trap/guard card in the same Chamber while one is already present, the presence refreshes rather than duplicating.

---

## 16. Conditions

Conditions are Chamber states left by some cards or events.

Current conditions:

| Condition | Source | Effect |
|---|---|---|
| Cold | 🧊 Cold Room | weakens Candle, Small Comfort, and Sweetness by 1 in that Chamber |
| Thorn | 🥀 Memory Thorn / 🪞 Shame Mirror | weakens Loved Memory by 2, then fades |

Some cleansing effects can remove Devil effects and conditions.

---

## 17. Card Special Rules

This section explains the main named special effects.

## 17.1 Heaven Specials

- **Candle of Warmth** gains +1 force in Body.
- **Open Hand / Hope** become stronger when Heaven is behind.
- **Calm Breath** weakens opposing same-Chamber plays and can cleanse Devil effects.
- **Hymn** is ongoing +1 Heaven.
- **Rest** moves Heaven, removes a Devil effect, and thaws Cold.
- **Loved Memory** is a strong Heaven push and dissolves a Devil effect in Memory.
- **Guardian** guards a Chamber and weakens Devil plays there.
- **Little Miracle** makes an additional wild d6 roll:
  - 1–2: Heaven gains +1 Will next round
  - 3–4: clean Heaven push
  - 5–6: larger Heaven push
- **Bell of Mercy** cancels a Devil ongoing/trap/condition; if nothing is found, it nudges or pushes Heaven.
- **Small Comfort / Silver Thread** give the next Heaven effect +1.
- **Sweetness** gives Heaven +1 Will next round if Heaven is already ahead.
- **Acceptance** cleanses a Devil effect or condition.
- **Dawn** uses Delayed movement.
- **Grace** is a Heavy Desperate rescue card.

## 17.2 Hell Specials

- **Ache** is direct Hell pressure and ongoing Body pain.
- **Screech** is stronger if the Soul is already below 0.
- **Fly Swarm** is ongoing Hell irritation.
- **Void Thought** is stronger when Hell is behind.
- **Memory Thorn / Shame Mirror** leave Thorn.
- **Burden** reduces the opponent's Will next round.
- **False Door / Snare** are traps.
- **Watching Eye** gives Hell +1 Will next round and makes a small Hell push.
- **Endless Wait** creates ongoing Hell pressure and caps Heaven movement next round.
- **Cold Room** leaves Cold.
- **Tangled Desire** weakens opposing Heart plays and is strong as a Sever tool.
- **Whispering Serpent** makes an additional wild d6 roll:
  - 1–2: Heaven loses 1 Will next round
  - 3–4: clean Hell push
  - 5–6: larger Hell push
- **Weight** is a Heavy common pressure card.
- **Fever** is Volatile burst.
- **Obsession** is Binding ongoing pressure.
- **Futility** uses Delayed movement.
- **Rupture** is a Hungry burst card.
- **Crawling Skin** is Echo irritation.

---

## 18. Strain and Purgatory Events

Strain measures instability in the Soul.

Strain increases when:

- a raw intensity roll is 6
- a Volatile card weak-rolls and backfires

When Strain reaches 3:

1. reduce Strain by 3
2. roll a d6
3. resolve a Purgatory Event

| d6 | Event | Effect |
|---:|---|---|
| 1 | 🕳️ Collapse | next Hell push gets +2 |
| 2 | 🌤️ Reprieve | next Heaven push gets +2 |
| 3 | 🪞 Flashback | Memory conditions clear and both Memory stacks lose 2 |
| 4 | 🕰️ Stasis | next round's Soul movement is capped at ±2 |
| 5 | 🔥 Intensification | next active effect gets +1 |
| 6 | ⚖️ Judgement | side behind gets +2 Will next round; if tied, both get +1 |

---

## 19. Full Round Walkthrough

This example uses Standard ±18.

Initial state:

```text
Soul: 0
Heaven Will: 3
Hell Will: 3
No stacks
No board effects
```

## 19.1 Reveals

Heaven reveals:

```text
🕯️ Candle of Warmth
🤲 Open Hand
🛏️ Rest
🪶 Lightness
```

Hell reveals:

```text
🦷 Ache
🔊 Screech
🪨 Weight
🐜 Crawling Skin
```

## 19.2 Commit Plays

Heaven chooses:

```text
🕯️ Candle of Warmth → 🫀 Body / Press
```

Hell chooses:

```text
🦷 Ache → 🫀 Body / Press
```

Both chose Body, so they will clash.

## 19.3 Intensity Rolls

Heaven rolls 4.

```text
Raw 4 = intensity 2
```

Hell rolls 3.

```text
Raw 3 = intensity 2
```

Neither rerolls.

## 19.4 Calculate Pending Plays

Heaven Candle:

```text
base_power 1 + intensity 2 = power 3
base_move 1 + intensity 2 = amount 3
Body bonus +1 = power 4, amount 4
Press bonus = 0 because no friendly stack exists yet
```

Hell Ache:

```text
base_power 1 + intensity 2 = power 3
base_move 1 + intensity 2 = amount 3
Press bonus = 0 because no friendly stack exists yet
```

## 19.5 Clash

Both are in Body.

```text
Heaven power 4 vs Hell power 3
```

Heaven wins the clash.

Hell's amount is halved:

```text
Hell amount 3 → 1
```

## 19.6 Apply Plays

Candle contributes:

```text
+4 Heaven movement
+2 Heaven Warmth stack in Body
```

Ache contributes:

```text
-1 Hell movement after clash halving
+2 Hell Pain stack in Body
Ache becomes an ongoing Body effect
```

## 19.7 Ongoing Effects

Ache is ongoing and contributes immediately:

```text
Ongoing Ache: -1
```

## 19.8 Net Movement

```text
Heaven active: +4
Hell active:   -1
Ongoing:       -1
Net:           +2
```

Soul moves:

```text
0 → +2
```

## 19.9 End of Round

No Strain event occurs unless enough Strain was generated.

No player has reached ±18.

Proceed to the next round.

---

## 20. Human vs Computer

In Human vs Computer mode:

1. The computer reveals its hand.
2. At round start, the computer secretly commits a glyph, Chamber, and Intent before the human chooses.
3. The human chooses their glyph, Chamber, and Intent.
4. The computer reveals its committed play.
5. Both plays roll and resolve.

The AI is not supposed to see the human's committed move before deciding.

## 20.1 AI Personalities

| AI | Style |
|---|---|
| Novice / Forgiving | loose, beginner-friendly, sometimes wasteful |
| Balanced / Tactical | default practical opponent |
| Architect / Stack Builder | builds and protects Resonance, then Releases |
| Cruel Accountant / Hard | calculating, contests leverage, low randomness |
| Chaos Imp / Weird | high-variance, unpredictable |

---

## 21. Human vs Human

## 21.1 Open Hot-seat

Both players can see both revealed hands.

Use this for early playtesting, teaching, and balance inspection.

## 21.2 Secret Hot-seat

Secret hot-seat hides information between choices.

Flow:

1. Player 1 chooses privately.
2. The screen masks Player 1's hand, selected play, and relevant log details.
3. Player 1 passes control to Player 2.
4. Player 2 chooses from their own visible reveal.
5. Both plays reveal and resolve.

Use this for fairer competitive local play.

---

## 22. Endgame Summary and Playtest Export

When a game ends, the digital version records:

- winner
- final Soul value
- rounds played
- total Heaven pressure
- total Hell pressure
- raw pressure difference
- actual capped movement
- most-used Chamber
- Intent usage
- biggest single swing
- biggest net round
- last 5 rounds
- full round ledger

The game can export:

```text
full playtest report .txt
per-round ledger .rounds.csv
```

These reports are for balance analysis and design iteration.

---

## 23. Current Built-in Decks

## 23.1 Heaven Decks

| Deck | Glyphs |
|---|---|
| Starter Mercy | 🕯️ 🤲 🛏️ 🎵 🪶 🫖 🌤️ 🧸 🍯 💌 |
| Comfort Engine | 🕯️ 🕯️ 🛏️ 🧸 🍯 🪶 🫖 🤲 🎵 🪽 |
| Sanctuary Control | 🪽 🏰 🌊 🔔 🎵 🛏️ 🪷 🫖 🌤️ 🪶 |
| Hope From Behind | 🌤️ 🧭 🕊️ 🤲 💌 🛏️ 🌊 ✨ 🕯️ 🪶 |
| Memory Loom | 💌 🧵 🪷 🤲 🕊️ 🪶 🌤️ 🧸 🛏️ 🫖 |
| Delayed Dawn | ☀️ 🧭 🎵 🪽 🕯️ 🛏️ 🪶 🫖 🧸 🍯 |

## 23.2 Hell Decks

| Deck | Glyphs |
|---|---|
| Starter Torment | 🦷 🔊 🪰 🕳️ ⛓️ 🧊 🪨 🐜 🪦 🥀 |
| Pain Engine | 🦷 🦷 🔊 🪰 🧊 ⛓️ 🪨 🐜 🕳️ 🐍 |
| Despair Control | 🕳️ 🥀 ⏳ 👁️ 🪤 🕸️ 🪦 ⛓️ 🐜 🔊 |
| Trickster Hell | 🪤 👁️ 🧵 🐍 🔊 🕸️ 🐜 🪦 ⛓️ 🕳️ |
| Fever Break | 🔥 🧨 🪨 🦷 🐜 ⛓️ 🧊 🪰 🕳️ 🔊 |
| Memory Shame | 🪞 🥀 🧿 👁️ 🕸️ 🪦 🕳️ 🐜 ⛓️ 🔊 |

---

## 24. Card Reference

## 24.1 Heaven Cards

| Glyph | Name | Rarity | Price | Chambers | Type | Stack | Move | Ongoing | Special Summary |
|---|---|---:|---|---|---|---:|---:|---:|---|
| 🕯️ | Candle of Warmth | Common | — | 🫀 Body, ❤️ Heart | Direct / Guard | 2 | +1 | — | A reliable comfort push. +1 extra in Body. |
| 🤲 | Open Hand | Uncommon | — | ❤️ Heart, 🪞 Memory | Direct | 2 | +1 | — | A mercy play. Stronger if the Soul is below 0. |
| 🌊 | Calm Breath | Rare | — | 🧠 Mind, 🫀 Body, ❤️ Heart | Disruption / Cleanse | 1 | — | — | Weakens opposing plays in the same Chamber, or cleanses Devil effects. |
| 🎵 | Hymn | Uncommon | — | 🧠 Mind, ❤️ Heart | Ongoing | 2 | — | +1 | Ongoing: pushes +1 each round. |
| 🛏️ | Rest | Common | — | 🫀 Body, 🕰️ Time | Direct / Cleanse | 2 | +1 | — | Pushes Heaven and removes one Devil effect in this Chamber. |
| 💌 | Loved Memory | Rare | — | 🪞 Memory, ❤️ Heart | Direct / Cleanse | 3 | +2 | — | Strong Heaven push. In Memory, dissolves one Devil effect before scoring. |
| 🪽 | Guardian | Rare | — | 🫀 Body, 🧠 Mind, ❤️ Heart, 🕰️ Time, 🪞 Memory | Ongoing / Guard | 2 | — | — | Ongoing guard: Devil plays in this Chamber lose 2 power. |
| ✨ | Little Miracle | Rare | Volatile | 🫀 Body, 🧠 Mind, ❤️ Heart, 🕰️ Time, 🪞 Memory | Wild | 1 | — | — | Wild: rolls for Will or a burst Heaven push. |
| 🌤️ | Hope | Uncommon | Desperate | ❤️ Heart, 🕰️ Time | Scaling | 2 | +1 | — | Pushes Heaven. Stronger when Heaven is losing. |
| 🔔 | Bell of Mercy | Rare | — | 🫀 Body, 🧠 Mind, ❤️ Heart, 🕰️ Time, 🪞 Memory | Disruption | 1 | — | — | Cancels one Devil ongoing/trap in the chosen Chamber. |
| 🧸 | Small Comfort | Common | — | 🫀 Body, ❤️ Heart | Support | 2 | +1 | — | Small push. Your next Heaven effect gets +1. |
| 🍯 | Sweetness | Uncommon | — | 🫀 Body, ❤️ Heart | Direct / Will | 2 | +1 | — | Pushes Heaven. If Heaven is ahead, gain +1 Will next round. |
| 🪶 | Lightness | Common | Echo | 🫀 Body, 🧠 Mind, ❤️ Heart | Direct / Echo | 1 | +1 | — | Gentle push. Echo repeats as a small +1 Heaven nudge next round. |
| 🧭 | Purpose | Uncommon | Desperate | 🧠 Mind, 🕰️ Time | Direct / Comeback | 2 | +1 | — | Desperate: stronger while Heaven is behind, weaker while Heaven is already ahead. |
| 🏰 | Sanctuary | Rare | Heavy, Binding | 🫀 Body, ❤️ Heart | Ongoing / Guard | 3 | — | — | Heavy Binding guard. Very strong defense, but costs Will and locks that Chamber next round. |
| ☀️ | Dawn | Rare | Delayed | 🕰️ Time, ❤️ Heart | Direct / Delayed | 2 | +2 | — | Delayed: schedules its Heaven movement for the next round instead of scoring immediately. |
| 🫖 | Tea & Quiet | Common | Narrow | 🧠 Mind | Direct / Narrow | 1 | +1 | — | Efficient Mind-only calm. Narrow but reliable. |
| 🪷 | Acceptance | Uncommon | Hungry | 🧠 Mind, 🪞 Memory | Disruption / Cleanse | 2 | +1 | — | Hungry cleanse. Consumes friendly stack if possible; removes a Devil effect/condition here. |
| 🧵 | Silver Thread | Rare | Fragile | ❤️ Heart, 🪞 Memory | Support / Fragile | 3 | +1 | — | Fragile support. Builds a powerful connection stack and boosts the next Heaven effect. |
| 🕊️ | Grace | Rare | Heavy, Desperate | ❤️ Heart, 🪞 Memory | Direct / Rescue | 2 | +2 | — | Heavy rescue. Stronger when Heaven is behind; expensive when you are trying to turn the tide. |

## 24.2 Hell Cards

| Glyph | Name | Rarity | Price | Chambers | Type | Stack | Move | Ongoing | Special Summary |
|---|---|---:|---|---|---|---:|---:|---:|---|
| 🦷 | Ache | Common | — | 🫀 Body | Direct / Ongoing | 2 | +1 | +1 | Pushes Hell and leaves ongoing Body pain. |
| 🔊 | Screech | Common | — | 🧠 Mind | Direct | 2 | +1 | — | Pushes Hell. Stronger if the Soul is already below 0. |
| 🪰 | Fly Swarm | Uncommon | — | 🫀 Body, 🧠 Mind | Ongoing | 2 | — | +1 | Ongoing: pushes -1 each round. |
| 🕳️ | Void Thought | Uncommon | Desperate | 🧠 Mind, 🕰️ Time | Scaling | 2 | +1 | — | Pushes Hell. Stronger when Hell is losing. |
| 🥀 | Memory Thorn | Rare | — | 🪞 Memory, ❤️ Heart | Direct / Condition | 3 | +1 | — | Pushes Hell and leaves Thorn to weaken Loved Memory. |
| ⛓️ | Burden | Uncommon | — | 🫀 Body, 🕰️ Time | Direct / Delay | 2 | +1 | — | Pushes Hell and reduces opponent Will next round. |
| 🪤 | False Door | Rare | — | 🫀 Body, 🧠 Mind, ❤️ Heart, 🕰️ Time, 🪞 Memory | Trap | 1 | — | — | Trap: next Angel play in this Chamber loses 3 power, then vanishes. |
| 👁️ | Watching Eye | Rare | — | 🧠 Mind, 🪞 Memory | Control | 1 | — | — | Gives Hell +1 Will next round and a small Hell push. |
| ⏳ | Endless Wait | Rare | — | 🕰️ Time | Ongoing / Cap | 2 | — | +1 | Ongoing Hell pressure. Caps Heaven movement next round. |
| 🧊 | Cold Room | Uncommon | — | 🫀 Body, ❤️ Heart | Direct / Condition | 2 | +1 | — | Pushes Hell and leaves Cold, weakening warmth/comfort here. |
| 🧵 | Tangled Desire | Rare | — | ❤️ Heart | Disruption | 1 | — | — | Disrupts Heart. Weakens opposing Heart plays and can Sever stack well. |
| 🐍 | Whispering Serpent | Rare | Volatile | 🫀 Body, 🧠 Mind, ❤️ Heart, 🕰️ Time, 🪞 Memory | Wild | 1 | — | — | Wild: rolls for Will drain or a burst Hell push. |
| 🪨 | Weight | Common | Heavy | 🫀 Body, 🕰️ Time | Direct / Heavy | 2 | +2 | — | Heavy reliable Hell pressure. Strong for a common, but costs Will or loses force. |
| 🕸️ | Snare | Uncommon | Fragile | 🫀 Body, 🕰️ Time | Trap / Fragile | 2 | — | — | Fragile trap. Strong tempo catch if uncontested, poor if Angel fights the same Chamber. |
| 🔥 | Fever | Rare | Volatile | 🫀 Body, 🧠 Mind | Direct / Volatile | 3 | +2 | — | Volatile burst. Weak rolls lose force; surge rolls bite harder. |
| 🧿 | Obsession | Rare | Binding | ❤️ Heart, 🪞 Memory | Ongoing / Binding | 3 | +1 | +1 | Binding ongoing lock. Good pressure, but you cannot use that Chamber next round. |
| 🪦 | Futility | Uncommon | Delayed | 🧠 Mind, 🕰️ Time | Direct / Delayed | 2 | +2 | — | Delayed despair pulse. Telegraphs pressure instead of scoring immediately. |
| 🧨 | Rupture | Rare | Hungry | 🫀 Body, 🪞 Memory | Direct / Hungry | 3 | +2 | — | Hungry burst. Feeds on Hell stack; painful if prepared, blunter if played naked. |
| 🐜 | Crawling Skin | Common | Echo | 🫀 Body, 🧠 Mind | Direct / Echo | 1 | +1 | — | Low irritation. Echo repeats as a small -1 Hell nudge next round. |
| 🪞 | Shame Mirror | Rare | Narrow, Fragile | 🪞 Memory | Direct / Narrow | 3 | +2 | — | Memory-only rare. Brutal if protected, fragile if Angel contests Memory. |

---

## 25. Rules Priorities and Edge Cases

1. **Card text and coded special rules override generic rules.**
2. **Movement type determines legal movement direction.**
3. **Power decides clashes; Amount decides movement/effect size.**
4. **If Power is reduced to 0, the play fizzles.**
5. **If Amount is halved, round down.**
6. **Delayed movement happens at the start of the next round before new reveals.**
7. **Echo only schedules if the card actually moved the Soul.**
8. **Heavy spends Will automatically if available.**
9. **Hungry does not trigger while using Release.**
10. **Binding locks only the Chamber used by that side for the next round.**
11. **Traps trigger before clash and then vanish.**
12. **Guards persist and keep weakening opposing plays in their Chamber.**
13. **Ongoing effects apply after both active plays, including the round they are placed.**
14. **Movement caps apply to the round's total movement, not to each individual play.**
15. **The Soul cannot move beyond the selected goal.**

---

## 26. Design Notes for Future Expansions

The current rules deliberately avoid power creep.

Future glyphs should create tactical identity through:

- movement type
- price keywords
- Chamber restrictions
- stack interaction
- delayed timing
- self-costs
- opponentward movement
- conditional payoff

Rare cards should not simply be stronger.

They should be more specific, more volatile, more committed, or more dangerous to misuse.

---

## 27. Quick Reference

## Round Sequence

```text
1. Start round
2. Set Will
3. Apply Chamber locks
4. Resolve delayed movement
5. Reveal 4 glyphs per side
6. AI commits blind if playing Human vs Computer
7. Player 1 chooses glyph, Chamber, Intent
8. Player 1 rolls intensity and may reroll
9. Player 2 / AI chooses or reveals committed play
10. Player 2 / AI rolls intensity and may reroll
11. Build pending plays
12. Apply guard/trap/keyword pre-clash modifiers
13. Resolve clash if both chose same Chamber
14. Apply both active plays
15. Apply ongoing effects
16. Sum movement and apply caps
17. Move Soul
18. Resolve Strain events
19. Check victory
20. Begin next round
```

## Intent Formulas

```text
Press bonus = floor((friendly stack + harmony - floor(enemy stack / 2)) / 3)
Anchor stack = card stack value + intensity + 1 if harmony exists
Release bonus = min(5, ceil(spent stack × intensity / 4))
Sever cut = card stack value + intensity + 1 if disruptive
```

## Deck Rules

```text
Deck size: 10
Reveal size: 4
Rarity budget: ≤ 24
Maximum Rares: 4
```

## Intensity

```text
1–2 = intensity 1
3–4 = intensity 2
5–6 = intensity 3
```

---

**End of Rulebook.**
