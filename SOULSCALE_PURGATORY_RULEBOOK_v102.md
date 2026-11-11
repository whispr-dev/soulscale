# Soulscale: Purgatory — Definitive Rulebook

**Rules version:** v1.0.2 — Movement Resolver (corrected)  
**Document status:** rulebook for the Discord release (Rust engine)  
**Last updated:** 2026-11-11

> **What's new since the v1.0.1 rulebook**
>
> - **Four rule fixes** (v1.0.2): Screech now works as printed, the computer opponent scales its sense of urgency to the game length, Void Thought counts as a defender for the computer, and Silver Thread's log line names itself. See §28.
> - **v1.0.1 is still playable**: choose it in game setup for the exact original rules.
> - **Online Duels** inside Discord, with spectators, a turn timer and computer cover for dropped players (§21.3).
> - **Details the old rulebook left out**, taken from how the engine actually behaves: how the deck cycles (§7.3), forced plays (§9.2), one-shot bonuses (§9.4), stack token merging (§12.4), when Heavy pays (§14.1), and exactly what each special does (§17).
> - **Open design questions**: a few behaviours that may not be what was intended are described as they actually play, and listed in §28.3.

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

The digital version is played as a **Discord Activity**: it opens inside a Discord voice or text channel, and everyone who launches it in the same place shares one table.

---

## 2. Components

A complete digital game contains:

- 1 shared **Soul Track**
- 5 **Chambers**
- 2 player decks, one Heaven and one Hell
- revealed glyph rows for both players
- Will counters
- Resonance stacks for each side in each Chamber
- a Strain gauge
- board effects, traps, guards, conditions, delayed effects, and log/export tools
- a Deck Builder for making and saving your own decks

For tabletop/proxy play, use:

- one marker for the Soul Track
- paper/card glyphs or tokens
- five labelled Chamber spaces
- counters for Will, Strain and Resonance
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

The recommended default is **Standard ±18**.

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
- Some effects can cap movement for a round (§9.9).

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

A legal deck must obey:

```text
Deck size: exactly 10 glyphs
Rarity budget: 24 or less
Rare limit: 4 or less
```

Duplicates are allowed unless a later mode explicitly forbids them.

## 7.2 Building Decks

In the digital game, the **Deck Builder** shows every glyph for the chosen side, a live budget and rare meter, and whether the deck is legal. Saved decks appear in game setup next to the built-in decks.

Decks can also be exported and imported as text: one emoji per line, UTF-8. Blank lines and lines beginning with `#` are ignored.

```text
# Starter Mercy
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

Legacy 8-glyph decks are still accepted. They are padded to 10 with that side's first two common glyphs (Heaven: 🕯️ Candle of Warmth, 🛏️ Rest; Hell: 🦷 Ache, 🔊 Screech) and then checked for legality.

The game uses one glyph for both 🧵 Silver Thread (Heaven) and 🧵 Tangled Desire (Hell). Which one you get depends on the side the deck is for.

## 7.3 How the Deck Cycles

1. At the start of the game, each deck is shuffled into a draw pile.
2. Each round, any glyphs left over from last round's reveal go to the discard pile, then 4 new glyphs are revealed from the top of the draw pile.
3. The glyph you play goes to the discard pile.
4. Whenever the draw pile runs out, even halfway through a reveal, the discard pile is shuffled to make a new draw pile.

So over the first two rounds you see 8 of your 10 cards. The deck reshuffles partway through the third round's reveal.

---

## 8. Setup

1. Choose game mode:
   - **vs Computer**: Human vs Computer
   - **Duel**: Human vs Human online, each on their own screen (§21.3)
   - **Pass & Play**: Human vs Human on one screen, open (§21.1)
   - **Pass & Play, secret**: Human vs Human on one screen, hands hidden (§21.2)
2. Player 1 chooses a side:
   - Heaven / Angel
   - Hell / Devil
3. The opponent takes the other side.
4. Choose game length:
   - Quick ±12
   - Standard ±18
   - Deep ±24
5. Choose the **rules version**: v1.0.2 (current, the default) or v1.0.1 (the original rules exactly). See §28.
6. Choose or build a 10-glyph deck for each side. Against the computer, also choose its personality and deck (or let its personality pick).
7. Set the Soul to 0.
8. Clear all Chambers, stacks, conditions, delayed effects, and board effects.
9. Begin Round 1.

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
4. Resolve delayed movements from previous rounds (Delayed cards and Echo repeats).
5. Check for victory from delayed movement.
6. Reveal 4 glyphs for each player (§7.3).
7. Against the computer, the computer secretly commits its glyph, Chamber, and Intent before the human chooses.

## 9.2 Player Choice

On their turn, a player chooses:

1. one revealed glyph
2. one legal Chamber: one the glyph lists, and not locked for their side this round (§14.5)
3. one Intent:
   - Press
   - Anchor
   - Release (choosing how much stack to spend at the same time)
   - Sever

**Forced play.** If none of your revealed glyphs has a legal, unlocked Chamber, you must play your first revealed glyph, in the first Chamber it lists, as Press. The lock is ignored for that play.

## 9.3 Intensity Roll

Each played glyph rolls one d6.

| d6 | Intensity |
|---:|---:|
| 1–2 | 1 |
| 3–4 | 2 |
| 5–6 | 3 |

After seeing the roll, a player may spend **1 Will** to reroll once. The rerolled result is final.

The computer may reroll depending on its personality and available Will.

Each **final** raw roll of **6** adds 1 Strain. A 6 that you reroll away adds nothing.

## 9.4 Build Pending Plays

After cards, Chambers, Intents and final rolls are known, each play calculates:

```text
Power  = base_power + intensity + modifiers
Amount = base_move + intensity + modifiers, if the card has base movement (otherwise 0)
```

- **Power** decides clashes.
- **Amount** decides how much movement or effect magnitude the card contributes.
- Some cards have base movement 0 and therefore do not score directly unless their special effect or a Release says otherwise.
- No modifier can take Power or Amount below 0.

Modifiers are added in this order:

1. **One-shot bonuses** waiting for this side (see below)
2. **Card specials** that change force (Candle in Body, the "behind" bonus, Screech)
3. **Conditions** in the Chamber (Cold, Thorn)
4. **Price keywords**: Heavy, then Desperate, then Volatile
5. **The Intent's** stack maths (§11)
6. **Hungry** (§14.6)

**One-shot bonuses** are the +1 from Small Comfort or Silver Thread, the +2 from the Reprieve or Collapse events, and the +1 from the Intensification event. Each is used up by the next play on its side that has an Amount above 0 at step 1. That includes a play that then chooses **Anchor**, which sets its Amount to 0, so the bonus is wasted. Several Small Comfort / Silver Thread bonuses add together and are all used at once. Intensification goes to whichever play is built first, and Player 1's play is always built first.

## 9.5 Pre-Clash Modifiers

Before clashes are checked, board effects and keywords weaken plays:

| Source | Applies to | Effect |
|---|---|---|
| Guard (🪽 Guardian, 🏰 Sanctuary) | opposing plays in that Chamber | -2 Power and Amount, every round it stays |
| Trap (🪤 False Door, 🕸️ Snare) | the next opposing play in that Chamber | -3 Power and Amount, then the trap vanishes |
| Fragile | a Fragile card contested in its Chamber | -1 Power and Amount |
| 🌊 Calm Breath | the opposing play in the same Chamber | -2 Power and Amount |
| 🧵 Tangled Desire | the opposing play in the same Chamber | -2 Power and Amount |

Guards and traps are checked first, Player 1's play then Player 2's. Then Fragile, then Calm Breath, then Tangled Desire.

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

After the clash, Player 1's play is applied in full, then Player 2's. For each play:

1. If its Power is 0, it **fizzles**: nothing happens and it scores nothing.
2. Halve its Amount if it lost (or tied) a clash.
3. Spend friendly stack (Release, Hungry) or cut enemy stack (Sever).
4. Apply the card's special effect, which decides its movement.
5. If it's **Delayed**, move that movement to the start of next round. If it's **Echo** and it moved, schedule its ±1 repeat.
6. Add new Resonance stack.
7. Apply a Binding lock for next round.
8. Place an ongoing, guard, or trap effect.

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

Then apply movement caps:

- **Endless Wait:** the round's net movement can be at most +2 toward Heaven (§17.2).
- **Stasis:** net movement can be at most ±2 (§18).

Then move the Soul. The track edge absorbs anything past the goal.

## 9.10 Strain and Purgatory Events

After Soul movement, resolve Strain.

Whenever Strain reaches 3, spend 3 Strain and roll a Purgatory Event (§18).

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

Current Will uses:

- spend 1 Will to reroll intensity once
- Heavy cards automatically spend 1 Will if available (§14.1)
- certain cards add or subtract Will next round

Unused Will does not carry over into the next round.

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

The bonus is added to both Amount and Power. Press then adds the card's normal stack value to that Chamber.

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

You choose how much stack to spend when you choose the Intent, before your intensity is rolled. You can't spend more than you have there.

Formula:

```text
Release bonus = min(5, ceil(spent stack × intensity / 4))
```

Release adds this bonus to both movement Amount and clash Power. Released stack is taken from your **newest** tokens first.

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

Sever cuts the opponent's **oldest** tokens first. Disruptive cards are 🌊 Calm Breath, 🔔 Bell of Mercy, 🪷 Acceptance, 🧵 Tangled Desire, 🪤 False Door, 🕸️ Snare, 👁️ Watching Eye and 🐍 Whispering Serpent.

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

Harmony is the **number of tokens** (not their value) of your stack in that Chamber with the same aspect as the played card.

Harmony improves Press and Anchor.

## 12.4 Stack Removal and Merging

- Release and Hungry consume your own stack, **newest tokens first**.
- Sever and the Flashback event cut stack **oldest tokens first**.
- Stack is removed by value, not necessarily by whole token.
- Each side can hold at most **9 tokens** in one Chamber. Adding a 10th merges the two oldest into one: the value is kept, and the merged token takes the aspect of the newer of the two. Because harmony counts tokens, merging can lower harmony.

---

## 13. Movement Resolver

Each card declares a movement type.

| Movement Type | Meaning |
|---|---|
| ownerward | movement goes toward the card owner's win condition |
| opponentward | movement intentionally goes toward the opponent as a cost, bait, or sacrifice |
| heavenward | movement is always positive |
| hellward | movement is always negative |
| neutral | the card cannot move the Soul |

Every current card is ownerward. The engine supports the other types for future sacrifice and bait cards.

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

Heavy pays when the round resolves, **after rerolls**. If you spend your last Will on a reroll, your Heavy card loses 1 force.

## 14.2 Fragile

If a Fragile card is contested in the same Chamber by the opponent's play, it loses 1 force.

## 14.3 Volatile

Volatile cards care about the final raw d6 roll.

| Raw Roll | Effect |
|---:|---|
| 1–2 | loses 2 force and adds +1 Strain |
| 3–4 | no extra effect |
| 5–6 | gains +1 force |

## 14.4 Delayed

If a Delayed card would move the Soul, that movement is scheduled for the start of the next round instead of happening immediately.

Other non-movement effects (stack, conditions, locks) still occur normally.

## 14.5 Binding

After a Binding card is played, its side cannot use that same Chamber next round. The other side is unaffected.

## 14.6 Hungry

If the card is not using Release:

- if friendly stack exists in the Chamber, consume 1 friendly stack and gain +1 force
- if no friendly stack exists, lose 1 force

Hungry does not trigger during Release.

## 14.7 Echo

If an Echo card moves the Soul, it schedules a weak +1 or -1 repeat movement for the start of next round.

The Echo movement uses the card's movement type.

## 14.8 Desperate

A Desperate card is stronger while its side is behind and weaker while its side is ahead.

| Side | Behind means | Ahead means |
|---|---|---|
| Heaven | Soul below 0 | Soul above 0 |
| Hell | Soul above 0 | Soul below 0 |

Behind: +1 force.  
Ahead: -1 force.  
At exactly 0: no change.

🌤️ Hope and 🕳️ Void Thought also have their own "stronger when behind" rule (§17). The two stack: **+2 when behind, -1 when ahead.**

## 14.9 Narrow

A Narrow card is restricted to its listed Chamber or Chambers.

It has no extra math by itself; the limitation is the cost.

"Force" in this section means both Power and Amount.

---

## 15. Board Effects

Some cards remain in a Chamber after they are played.

## 15.1 Ongoing

Ongoing cards persist and contribute their ongoing movement after active plays resolve each round, including the round they are placed.

An ongoing effect with friendly stack 6+ in its Chamber gets +1 ongoing movement.

Ongoing cards: 🎵 Hymn, 🦷 Ache, 🪰 Fly Swarm, ⏳ Endless Wait, 🧿 Obsession.

## 15.2 Guard

Guard effects reduce opposing plays in their Chamber by 2 force before clash resolution, every round.

🪽 Guardian and 🏰 Sanctuary are current examples.

## 15.3 Trap

Trap effects reduce the next opposing play in their Chamber by 3 force, then vanish.

🪤 False Door and 🕸️ Snare are current examples.

## 15.4 No Duplicates

If the same side plays the same ongoing, trap or guard card in a Chamber where its copy is already present, nothing new is placed. Board effects never stack with themselves.

## 15.5 Removing Board Effects

Heaven's cleansing cards (🛏️ Rest, 💌 Loved Memory in Memory, 🌊 Calm Breath, 🔔 Bell of Mercy, 🪷 Acceptance) remove **one** Hell board effect in their Chamber: the oldest one. If there is none, they clear a condition instead: Thorn first, otherwise Cold.

No current card removes a Heaven board effect. Once placed, Hymn, Guardian and Sanctuary stay for the rest of the game.

---

## 16. Conditions

Conditions are Chamber states left by some cards or events.

| Condition | Source | Effect | Ends when |
|---|---|---|---|
| Cold | 🧊 Cold Room | Heaven's 🕯️ Candle, 🧸 Small Comfort and 🍯 Sweetness lose 1 force in that Chamber | cleansed (🛏️ Rest always thaws it) |
| Thorn | 🥀 Memory Thorn / 🪞 Shame Mirror | 💌 Loved Memory loses 2 force in that Chamber | it weakens a Loved Memory, is cleansed, or Flashback clears it (Memory only) |

A Chamber can't hold two of the same condition.

---

## 17. Card Special Rules

This section explains exactly what each named special does. Cards not listed simply move the Soul by their Amount toward their side, plus any keyword or board effect shown in the card table.

## 17.1 Heaven Specials

- **🕯️ Candle of Warmth**: +1 force in Body.
- **🤲 Open Hand**: +1 force while the Soul is below 0.
- **🌤️ Hope**: +1 force while the Soul is below 0. Also Desperate, so +2 when behind and -1 when ahead.
- **🌊 Calm Breath**: -2 to the opposing play in the same Chamber (§9.5). It removes one Hell board effect here, or a condition if there's no effect. If it didn't remove a board effect, it pushes Heaven by its Amount (which is 0 unless it Released or picked up a bonus).
- **🎵 Hymn**: ongoing +1 Heaven.
- **🛏️ Rest**: pushes Heaven, removes one Hell board effect here (or a condition if there's no effect), and thaws Cold.
- **💌 Loved Memory**: strong Heaven push. In Memory, it first dissolves one Hell board effect (or condition).
- **🪽 Guardian**: guard (§15.2).
- **✨ Little Miracle**: rolls an extra d6. 1–2: Heaven gets +1 Will next round. 3–4: pushes Heaven +2. 5–6: pushes Heaven +4. If its Amount is above its intensity (only possible through Release or bonuses), the excess is added to whichever result it rolls.
- **🔔 Bell of Mercy**: removes one Hell board effect here, or a condition if there's no effect. If it didn't remove a board effect, it pushes Heaven by its Amount, at least 1.
- **🧸 Small Comfort / 🧵 Silver Thread**: push, and give Heaven's next play with movement +1 (§9.4).
- **🍯 Sweetness**: push. If the Soul is above 0 when it resolves, Heaven gets +1 Will next round.
- **🪷 Acceptance**: removes one Hell board effect or condition here, and pushes Heaven. Hungry.
- **🧭 Purpose**: push. Desperate.
- **☀️ Dawn**: push, Delayed.
- **🕊️ Grace**: strong push. Heavy and Desperate.
- **🫖 Tea & Quiet**: push, Mind only.
- **🪶 Lightness**: push, Echo.
- **🏰 Sanctuary**: guard. Heavy and Binding.

## 17.2 Hell Specials

- **🦷 Ache**: pushes Hell and leaves ongoing Body pain (-1 a round).
- **🔊 Screech**: +1 force while the Soul is below 0. *(Under v1.0.1 it got +1 whenever the Soul was anywhere but exactly 0. See §28.)*
- **🪰 Fly Swarm**: ongoing -1 Hell.
- **🕳️ Void Thought**: +1 force while the Soul is above 0. Also Desperate, so +2 when behind and -1 when ahead.
- **🥀 Memory Thorn / 🪞 Shame Mirror**: push and leave Thorn.
- **⛓️ Burden**: push. Heaven gets 1 less Will next round.
- **🪤 False Door / 🕸️ Snare**: traps (§15.3). Snare is Fragile.
- **👁️ Watching Eye**: Hell gets +1 Will next round, and it pushes Hell by its Amount, at least 1.
- **⏳ Endless Wait**: ongoing -1 Hell. While it is on the board, **including the round it is played**, the round's net movement can't go above +2 toward Heaven. *(The card text says "next round". See §28.3.)*
- **🧊 Cold Room**: push and leave Cold.
- **🧵 Tangled Desire**: -2 to the opposing Heart play (§9.5). Disruptive, so it Severs well.
- **🐍 Whispering Serpent**: rolls an extra d6. 1–2: Heaven gets 1 less Will next round. 3–4: pushes Hell -2. 5–6: pushes Hell -4. If its Amount is above its intensity (only possible through Release or bonuses), the excess is added to whichever result it rolls.
- **🪨 Weight**: strong push. Heavy.
- **🔥 Fever**: strong push. Volatile.
- **🧿 Obsession**: push, ongoing -1 Hell, Binding.
- **🪦 Futility**: push, Delayed.
- **🧨 Rupture**: strong push. Hungry.
- **🐜 Crawling Skin**: push, Echo.

---

## 18. Strain and Purgatory Events

Strain measures instability in the Soul. It carries over from round to round.

Strain increases when:

- a final raw intensity roll is 6
- a Volatile card weak-rolls and backfires

When Strain reaches 3:

1. reduce Strain by 3
2. roll a d6
3. resolve a Purgatory Event

| d6 | Event | Effect |
|---:|---|---|
| 1 | 🕳️ Collapse | Hell's next play with movement gets +2 |
| 2 | 🌤️ Reprieve | Heaven's next play with movement gets +2 |
| 3 | 🪞 Flashback | Memory conditions clear and both Memory stacks lose 2 |
| 4 | 🕰️ Stasis | the next Soul movement is capped at ±2 (see note) |
| 5 | 🔥 Intensification | the next play with movement, either side, gets +1 |
| 6 | ⚖️ Judgement | side behind gets +2 Will next round; if tied, both get +1 |

**Stasis note:** the cap is used up by the **first** movement of the next round. If any Delayed or Echo movement happens at the start of that round, the cap applies to that movement only, and the round's main movement is not capped. See §28.3.

---

## 19. Full Round Walkthrough

This example uses Standard ±18. The digital engine's tests replay it exactly.

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

1. Both sides' reveals are visible to you.
2. At round start, the computer secretly commits a glyph, Chamber, and Intent before the human chooses.
3. The human chooses their glyph, Chamber, and Intent, rolls, and may reroll.
4. The computer reveals its committed play, rolls, and may reroll.
5. Both plays resolve.

The computer never sees the human's committed move before deciding.

## 20.1 AI Personalities

| AI | Style |
|---|---|
| Novice / Forgiving | Loose and a bit wasteful. Good for learning the board. |
| Balanced / Tactical | Reasonable tactical play with some personality noise. |
| Architect / Stack Builder | Prefers Anchor, protects strong Chambers, then releases hard. |
| Cruel Accountant / Hard | Calculating, low-noise, contests leverage and rerolls important plays. |
| Chaos Imp / Weird | High-variance gremlin. Sometimes brilliant, sometimes soup. |

## 20.2 How the Computer Judges Danger (v1.0.2)

The computer plays more urgently as the Soul nears either goal. Its thresholds were tuned when every game was ±12. Under v1.0.2 they keep the same distance from the goal at every game length: at ±18 it goes for the win when the Soul is 4 from the goal, as it does at ±12. Under v1.0.1 it reacted at the old ±12 positions whatever the goal, which made it hasty in Standard and Deep games.

---

## 21. Human vs Human

## 21.1 Pass & Play (open)

Two players share one screen. Both can see both revealed hands.

Use this for early playtesting, teaching, and balance inspection.

## 21.2 Pass & Play (secret)

Secret Pass & Play hides each player's revealed glyphs and choices from the other.

Flow:

1. A curtain covers the screen. Player 1 reveals their hand and chooses privately.
2. The curtain covers Player 1's hand, choice and roll.
3. Player 2 reveals their own hand and chooses.
4. Both plays reveal and resolve together.

Use this for fairer competitive local play.

## 21.3 Duel (online)

Two players in the same Discord Activity each play from their own screen.

1. One player opens a Duel, choosing side, deck, game length and rules version.
2. Anyone else at the table can **Accept**, choosing their own deck, and takes the other side.
3. Each round, both players choose at the same time. Your revealed glyphs, your choice and your roll are seen only by you. The opponent sees that you are "choosing" or "locked in".
4. When both are locked in, both plays are revealed and resolve together. Rerolls are shown to everyone at that point.

Table rules:

- **Spectators.** Anyone else at the table can watch. They see the board and every play as it's revealed, never a hand, choice or roll before the reveal.
- **Turn timer.** If a player doesn't act within 2 minutes, the computer plays that round for them.
- **Disconnects.** If a player drops out, their seat is held for 30 seconds. After that the computer covers it until they come back, and they get the seat back when they reconnect.
- **Open challenges** expire after 30 minutes if nobody accepts.
- **Resign** ends the match at once. The record is saved with the resignation noted, and no winner is declared.
- **Rematch** starts a new game with the same settings.

The server runs the rules for every mode. Your device only shows the game and sends your choices, so a modified client can't cheat.

---

## 22. Endgame Summary and Playtest Export

When a game ends, the digital version records:

- winner
- final Soul value
- rounds played
- rules version
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
full playtest report        .txt
per-round ledger            .rounds.csv
replay (seed + every input) .replay.json
```

A replay recreates the match exactly. The game server also keeps the same three files for each finished match, for bug reports and balance analysis, and deletes them after 12 months.

---

## 23. Current Built-in Decks

## 23.1 Heaven Decks

| Deck | Glyphs | Budget | Rares |
|---|---|---:|---:|
| Starter Mercy | 🕯️ 🤲 🛏️ 🎵 🪶 🫖 🌤️ 🧸 🍯 💌 | 16 | 1 |
| Comfort Engine | 🕯️ 🕯️ 🛏️ 🧸 🍯 🪶 🫖 🤲 🎵 🪽 | 15 | 1 |
| Sanctuary Control | 🪽 🏰 🌊 🔔 🎵 🛏️ 🪷 🫖 🌤️ 🪶 | 21 | 4 |
| Hope From Behind | 🌤️ 🧭 🕊️ 🤲 💌 🛏️ 🌊 ✨ 🕯️ 🪶 | 21 | 4 |
| Memory Loom | 💌 🧵 🪷 🤲 🕊️ 🪶 🌤️ 🧸 🛏️ 🫖 | 19 | 3 |
| Delayed Dawn | ☀️ 🧭 🎵 🪽 🕯️ 🛏️ 🪶 🫖 🧸 🍯 | 17 | 2 |

## 23.2 Hell Decks

| Deck | Glyphs | Budget | Rares |
|---|---|---:|---:|
| Starter Torment | 🦷 🔊 🪰 🕳️ ⛓️ 🧊 🪨 🐜 🪦 🥀 | 17 | 1 |
| Pain Engine | 🦷 🦷 🔊 🪰 🧊 ⛓️ 🪨 🐜 🕳️ 🐍 | 16 | 1 |
| Despair Control | 🕳️ 🥀 ⏳ 👁️ 🪤 🕸️ 🪦 ⛓️ 🐜 🔊 | 22 | 4 |
| Trickster Hell | 🪤 👁️ 🧵 🐍 🔊 🕸️ 🐜 🪦 ⛓️ 🕳️ | 22 | 4 |
| Fever Break | 🔥 🧨 🪨 🦷 🐜 ⛓️ 🧊 🪰 🕳️ 🔊 | 18 | 2 |
| Memory Shame | 🪞 🥀 🧿 👁️ 🕸️ 🪦 🕳️ 🐜 ⛓️ 🔊 | 22 | 4 |

All built-in decks are legal under §7.1.

---

## 24. Card Reference

**Power** is base power and **Stack** is the stack value. **Move** and **Ongoing** are sizes toward the card's own side, before intensity: Hell's "+1" moves the Soul down.

## 24.1 Heaven Cards

| Glyph | Name | Rarity | Price | Chambers | Type | Power | Stack | Move | Ongoing | Special Summary |
|---|---|---|---|---|---|---:|---:|---:|---:|---|
| 🕯️ | Candle of Warmth | Common | — | 🫀 Body, ❤️ Heart | Direct / Guard¹ | 1 | 2 | +1 | — | A reliable comfort push. +1 extra in Body. |
| 🤲 | Open Hand | Uncommon | — | ❤️ Heart, 🪞 Memory | Direct | 1 | 2 | +1 | — | A mercy play. Stronger if the Soul is below 0. |
| 🌊 | Calm Breath | Rare | — | 🧠 Mind, 🫀 Body, ❤️ Heart | Disruption / Cleanse | 2 | 1 | — | — | Weakens opposing plays in the same Chamber, or cleanses Devil effects. |
| 🎵 | Hymn | Uncommon | — | 🧠 Mind, ❤️ Heart | Ongoing | 1 | 2 | — | +1 | Ongoing: pushes +1 each round. |
| 🛏️ | Rest | Common | — | 🫀 Body, 🕰️ Time | Direct / Cleanse | 1 | 2 | +1 | — | Pushes Heaven and removes one Devil effect in this Chamber. |
| 💌 | Loved Memory | Rare | — | 🪞 Memory, ❤️ Heart | Direct / Cleanse | 2 | 3 | +2 | — | Strong Heaven push. In Memory, dissolves one Devil effect before scoring. |
| 🪽 | Guardian | Rare | — | 🫀 Body, 🧠 Mind, ❤️ Heart, 🕰️ Time, 🪞 Memory | Ongoing / Guard | 2 | 2 | — | — | Ongoing guard: Devil plays in this Chamber lose 2 power. |
| ✨ | Little Miracle | Rare | Volatile | 🫀 Body, 🧠 Mind, ❤️ Heart, 🕰️ Time, 🪞 Memory | Wild | 1 | 1 | — | — | Wild: rolls for Will or a burst Heaven push. |
| 🌤️ | Hope | Uncommon | Desperate | ❤️ Heart, 🕰️ Time | Scaling | 1 | 2 | +1 | — | Pushes Heaven. Stronger when Heaven is losing. |
| 🔔 | Bell of Mercy | Rare | — | 🫀 Body, 🧠 Mind, ❤️ Heart, 🕰️ Time, 🪞 Memory | Disruption | 2 | 1 | — | — | Cancels one Devil ongoing/trap in the chosen Chamber. |
| 🧸 | Small Comfort | Common | — | 🫀 Body, ❤️ Heart | Support | 1 | 2 | +1 | — | Small push. Your next Heaven effect gets +1. |
| 🍯 | Sweetness | Uncommon | — | 🫀 Body, ❤️ Heart | Direct / Will | 1 | 2 | +1 | — | Pushes Heaven. If Heaven is ahead, gain +1 Will next round. |
| 🪶 | Lightness | Common | Echo | 🫀 Body, 🧠 Mind, ❤️ Heart | Direct / Echo | 1 | 1 | +1 | — | Gentle push. Echo repeats as a small +1 Heaven nudge next round. |
| 🧭 | Purpose | Uncommon | Desperate | 🧠 Mind, 🕰️ Time | Direct / Comeback | 1 | 2 | +1 | — | Desperate: stronger while Heaven is behind, weaker while Heaven is already ahead. |
| 🏰 | Sanctuary | Rare | Heavy, Binding | 🫀 Body, ❤️ Heart | Ongoing / Guard | 3 | 3 | — | — | Heavy Binding guard. Very strong defense, but costs Will and locks that Chamber next round. |
| ☀️ | Dawn | Rare | Delayed | 🕰️ Time, ❤️ Heart | Direct / Delayed | 2 | 2 | +2 | — | Delayed: schedules its Heaven movement for the next round instead of scoring immediately. |
| 🫖 | Tea & Quiet | Common | Narrow | 🧠 Mind | Direct / Narrow | 2 | 1 | +1 | — | Efficient Mind-only calm. Narrow but reliable. |
| 🪷 | Acceptance | Uncommon | Hungry | 🧠 Mind, 🪞 Memory | Disruption / Cleanse | 2 | 2 | +1 | — | Hungry cleanse. Consumes friendly stack if possible; removes a Devil effect/condition here. |
| 🧵 | Silver Thread | Rare | Fragile | ❤️ Heart, 🪞 Memory | Support / Fragile | 1 | 3 | +1 | — | Fragile support. Builds a powerful connection stack and boosts the next Heaven effect. |
| 🕊️ | Grace | Rare | Heavy, Desperate | ❤️ Heart, 🪞 Memory | Direct / Rescue | 2 | 2 | +2 | — | Heavy rescue. Stronger when Heaven is behind; expensive when you are trying to turn the tide. |

## 24.2 Hell Cards

| Glyph | Name | Rarity | Price | Chambers | Type | Power | Stack | Move | Ongoing | Special Summary |
|---|---|---|---|---|---|---:|---:|---:|---:|---|
| 🦷 | Ache | Common | — | 🫀 Body | Direct / Ongoing | 1 | 2 | +1 | +1 | Pushes Hell and leaves ongoing Body pain. |
| 🔊 | Screech | Common | — | 🧠 Mind | Direct | 1 | 2 | +1 | — | Pushes Hell. Stronger if the Soul is already below 0. |
| 🪰 | Fly Swarm | Uncommon | — | 🫀 Body, 🧠 Mind | Ongoing | 1 | 2 | — | +1 | Ongoing: pushes -1 each round. |
| 🕳️ | Void Thought | Uncommon | Desperate | 🧠 Mind, 🕰️ Time | Scaling | 1 | 2 | +1 | — | Pushes Hell. Stronger when Hell is losing. |
| 🥀 | Memory Thorn | Rare | — | 🪞 Memory, ❤️ Heart | Direct / Condition | 2 | 3 | +1 | — | Pushes Hell and leaves Thorn to weaken Loved Memory. |
| ⛓️ | Burden | Uncommon | — | 🫀 Body, 🕰️ Time | Direct / Delay | 1 | 2 | +1 | — | Pushes Hell and reduces opponent Will next round. |
| 🪤 | False Door | Rare | — | 🫀 Body, 🧠 Mind, ❤️ Heart, 🕰️ Time, 🪞 Memory | Trap | 2 | 1 | — | — | Trap: next Angel play in this Chamber loses 3 power, then vanishes. |
| 👁️ | Watching Eye | Rare | — | 🧠 Mind, 🪞 Memory | Control | 2 | 1 | — | — | Gives Hell +1 Will next round and a small Hell push. |
| ⏳ | Endless Wait | Rare | — | 🕰️ Time | Ongoing / Cap | 2 | 2 | — | +1 | Ongoing Hell pressure. Caps Heaven movement next round. |
| 🧊 | Cold Room | Uncommon | — | 🫀 Body, ❤️ Heart | Direct / Condition | 1 | 2 | +1 | — | Pushes Hell and leaves Cold, weakening warmth/comfort here. |
| 🧵 | Tangled Desire | Rare | — | ❤️ Heart | Disruption | 2 | 1 | — | — | Disrupts Heart. Weakens opposing Heart plays and can Sever stack well. |
| 🐍 | Whispering Serpent | Rare | Volatile | 🫀 Body, 🧠 Mind, ❤️ Heart, 🕰️ Time, 🪞 Memory | Wild | 1 | 1 | — | — | Wild: rolls for Will drain or a burst Hell push. |
| 🪨 | Weight | Common | Heavy | 🫀 Body, 🕰️ Time | Direct / Heavy | 2 | 2 | +2 | — | Heavy reliable Hell pressure. Strong for a common, but costs Will or loses force. |
| 🕸️ | Snare | Uncommon | Fragile | 🫀 Body, 🕰️ Time | Trap / Fragile | 2 | 2 | — | — | Fragile trap. Strong tempo catch if uncontested, poor if Angel fights the same Chamber. |
| 🔥 | Fever | Rare | Volatile | 🫀 Body, 🧠 Mind | Direct / Volatile | 2 | 3 | +2 | — | Volatile burst. Weak rolls lose force; surge rolls bite harder. |
| 🧿 | Obsession | Rare | Binding | ❤️ Heart, 🪞 Memory | Ongoing / Binding | 2 | 3 | +1 | +1 | Binding ongoing lock. Good pressure, but you cannot use that Chamber next round. |
| 🪦 | Futility | Uncommon | Delayed | 🧠 Mind, 🕰️ Time | Direct / Delayed | 2 | 2 | +2 | — | Delayed despair pulse. Telegraphs pressure instead of scoring immediately. |
| 🧨 | Rupture | Rare | Hungry | 🫀 Body, 🪞 Memory | Direct / Hungry | 2 | 3 | +2 | — | Hungry burst. Feeds on Hell stack; painful if prepared, blunter if played naked. |
| 🐜 | Crawling Skin | Common | Echo | 🫀 Body, 🧠 Mind | Direct / Echo | 1 | 1 | +1 | — | Low irritation. Echo repeats as a small -1 Hell nudge next round. |
| 🪞 | Shame Mirror | Rare | Narrow, Fragile | 🪞 Memory | Direct / Narrow | 2 | 3 | +2 | — | Memory-only rare. Brutal if protected, fragile if Angel contests Memory. |

¹ Candle of Warmth's type line says "Guard", but the card places no guard. It is a plain push with a Body bonus. See §28.3.

---

## 25. Rules Priorities and Edge Cases

1. **Card text and coded special rules override generic rules.**
2. **Movement type determines legal movement direction.**
3. **Power decides clashes; Amount decides movement/effect size.**
4. **If Power is reduced to 0, the play fizzles.**
5. **If Amount is halved, round down.**
6. **No modifier takes Power or Amount below 0.**
7. **Delayed movement happens at the start of the next round before new reveals.**
8. **Echo only schedules if the card actually moved the Soul.**
9. **Heavy spends Will automatically if available, after rerolls.**
10. **Hungry does not trigger while using Release.**
11. **Binding locks only the Chamber used by that side for the next round.**
12. **If you have no legal play, you must play your first revealed glyph in its first listed Chamber as Press.**
13. **Traps trigger before clash and then vanish.**
14. **Guards persist and keep weakening opposing plays in their Chamber.**
15. **Ongoing effects apply after both active plays, including the round they are placed.**
16. **Movement caps apply to the round's total movement, not to each individual play.**
17. **The Soul cannot move beyond the selected goal.**
18. **One-shot bonuses are used by the next play with movement, even one that then Anchors.**
19. **Only the final roll counts for Strain and Volatile.**
20. **At 9 tokens, a side's stack in a Chamber merges its two oldest tokens.**

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

A gap worth filling: Hell has no way to remove Heaven's board effects (§15.5).

---

## 27. Quick Reference

## Round Sequence

```text
1. Start round
2. Set Will
3. Apply Chamber locks
4. Resolve delayed movement, check victory
5. Discard last round's unplayed reveals; reveal 4 glyphs per side
6. Computer commits blind, if playing vs Computer
7. Each player chooses glyph, Chamber, Intent (Release: how much stack)
8. Each player rolls intensity and may spend 1 Will to reroll once
9. Build pending plays
10. Apply guard/trap/keyword pre-clash modifiers
11. Resolve clash if both chose the same Chamber
12. Apply both active plays (Player 1 first)
13. Apply ongoing effects
14. Sum movement and apply caps
15. Move Soul
16. Resolve Strain events
17. Check victory
18. Begin next round
```

## Intent Formulas

```text
Press bonus   = floor((friendly stack + harmony - floor(enemy stack / 2)) / 3), min 0
                (added to Amount and Power; adds the card's stack value)
Anchor stack  = card stack value + intensity + 1 if harmony exists
                (no movement; Power -1)
Release bonus = min(5, ceil(spent stack × intensity / 4))
                (added to Amount and Power; no new stack)
Sever cut     = card stack value + intensity + 1 if disruptive
                (Amount -1, Power -1; +1 Power if enemy stack is present)
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

## Modifier Sizes

```text
Guard -2 · Trap -3 · Fragile -1 · Calm Breath / Tangled Desire -2
Heavy -1 if unpaid · Desperate ±1 · Volatile -2 / +1 · Hungry ±1
Cold -1 (Candle, Small Comfort, Sweetness) · Thorn -2 (Loved Memory)
```

---

## 28. Rules Versions

## 28.1 Choosing a Version

Every game records its rules version, and the game setup lets you choose one:

- **v1.0.2 (current, default):** the original rules with the fixes in §28.2.
- **v1.0.1 (original):** the original rules exactly, quirks included. The digital engine reproduces the original prototype's games identically under v1.0.1.

## 28.2 Changes in v1.0.2

| Change | v1.0.1 | v1.0.2 |
|---|---|---|
| 🔊 Screech | +1 whenever the Soul was anywhere but exactly 0, so it was also boosted while Heaven was ahead | +1 only while the Soul is below 0, as printed |
| Computer urgency | thresholds fixed at the old ±12 positions whatever the goal | thresholds keep their distance from the goal (§20.2) |
| Computer defence | never counted 🕳️ Void Thought as a defensive card, due to a naming slip | does |
| 🧵 Silver Thread log | said "Small Comfort gives the next Heaven effect +1" | names Silver Thread (the effect was always right) |

At ±12, with no Screech, Void Thought or Silver Thread in either deck, a game plays out identically under both versions.

## 28.3 Open Questions (unchanged in both versions)

These behaviours are in both versions and are described in this rulebook as they actually play. Each may or may not be what was intended:

1. **Hope and Void Thought** get +2 when behind, because their "behind" bonus and Desperate stack (§14.8). Intended identity, or should one replace the other?
2. **⏳ Endless Wait** caps Heaven's movement in the round it's played and every round it stays on the board. The card text says "next round" (§17.2).
3. **🕰️ Stasis** is used up by Delayed or Echo movement at the start of the next round, leaving the main movement uncapped (§18).
4. **One-shot bonuses** are wasted by a play that Anchors (§9.4).
5. **🕯️ Candle of Warmth** is typed "Direct / Guard" but has no guard (§24).
6. **Intensification** always goes to Player 1's play when both plays have movement (§9.4).

Balance observations from the simulator (computer vs computer, many thousands of games) are kept in the project's `docs/RULES_CHANGES.md`.

---

**End of Rulebook.**
