# How Soulscale is built

## The one rule

**The server owns the game.** Browsers send intentions ("play Hope in Heart as Press"), the server
checks and applies them with the rules engine, and sends each person only what they're allowed to
see. A modified client can't cheat, and the Python-faithful engine lives in exactly one place.

```
┌──────────── client/ (TypeScript, in Discord's frame) ────────────┐
│ main.ts   sign-in, connect, screens                              │
│ ui/game   board, hand, dice, intents   ui/anim  event animations │
│ fx.ts     particles, banners, shake    audio.ts music + SFX      │
└─────────────────────── WebSocket /ws (JSON) ─────────────────────┘
                                   │
┌──────────── soulscale-server (axum + tokio) ─────────────────────┐
│ ws.rs     auth (Discord token or dev name), rate limit, routing  │
│ rooms.rs  one room per Activity instance: lobby, Duel, rematch,  │
│           AFK auto-play, disconnect cover, saving replays        │
│ discord.rs  OAuth code exchange, /users/@me, instance check      │
└──────────────────────────────────────────────────────────────────┘
                                   │
┌──────────── soulscale-core (pure Rust, no I/O) ──────────────────┐
│ game.rs + rules/*  the Python engine, method for method          │
│ ai.rs     choose_ai_play / ai_should_reroll                      │
│ session.rs  turn flow for all four modes, who-sees-what          │
│ view.rs   per-viewer snapshots     events.rs  what happened      │
│ rng.rs    CPython-identical Mersenne Twister                     │
│ data/cards.json  every card, deck and AI personality             │
└──────────────────────────────────────────────────────────────────┘
```

## A round, end to end

1. The server deals: each seat's view lists its 4 revealed glyphs (hidden from the other side in a
   Duel or secret Pass & Play).
2. You click a glyph, a glowing Chamber, then an Intent. The client sends
   `{"t":"choose","seat":"P1","card":"hope","chamber":"Heart","intent":{"kind":"press"}}`.
3. The server rolls your die and sends your roll **to you only**. You Keep or Reroll.
4. When both seats are locked, the engine resolves the round and produces **events**
   (`plays`, `clash`, `stack_added`, `soul_move`, `purgatory`…) plus a fresh **view**.
5. Each connection gets the events it may see and a view built for it. The client animates the
   events in order (cards fly, clashes bump, the orb slides), then redraws from the view. The view is
   the truth; animations are decoration on top of it.

Each event carries a visibility (`Public`, `Only(seat)`, `Except(seat)`). Your reroll is `Only(you)`
until the round resolves, then it's revealed to everyone.

## Where to change things

| You want to… | Edit |
|---|---|
| Change a card's numbers, text, glyph or price | `crates/soulscale-core/data/cards.json`, then `cd client; npm run glyphs` if the glyph changed. Run `cargo test`: the v1.0.1 parity test will fail, which is expected once the cards differ from the Python. |
| Add a new special ability | Add it to `Special` in `model.rs`, implement it in `rules/effects.rs` (and `ai.rs` if the AI should value it). `cargo test -p soulscale-core --features ts --lib` regenerates the TypeScript types. |
| Add a rules revision (v1.0.3) | Add a variant to `RulesVersion` in `model.rs` and branch on it where behaviour changes, like the v1.0.2 fixes do. Old replays keep working. |
| Change which sound plays when | In the game: 🎚 Sound Lab, then Export manifest and save it over `client/public/audio/manifest.json`. Or edit that file directly: `sfx` maps events to files, `cards` and `chambers` override per card or Chamber. |
| Add new audio | Put the .wav in your audio folder, run `tools\convert-audio.ps1 -Source <folder>`, then map it in the Sound Lab. |
| Restyle anything | `client/src/styles.css`. Colours and fonts are CSS variables at the top. |
| Change an animation | `client/src/ui/anim.ts` (per-event choreography) and `client/src/fx.ts` (particles, banners). |
| Tune timeouts | `.env`: `SOULSCALE_TURN_TIMEOUT_SECS`, `SOULSCALE_RECONNECT_GRACE_SECS`. |

## Determinism and replays

Every match has a seed. Given the seed, the settings and the list of inputs, the engine always produces
the same game. That's how replays work (`.replay.json` stores just those), and how the parity test
compares Rust against Python line by line.

The engine never reads the clock or the OS random source mid-game. Time stamps come in through a
`Clock` trait (fixed in tests), and the only randomness is the seeded generator.

## Protocol types

`crates/soulscale-core/src/protocol.rs`, `view.rs` and `events.rs` define the messages. ts-rs
generates matching TypeScript into `client/src/protocol/generated/`:

```powershell
cargo test -p soulscale-core --features ts --lib
```

Run that after changing any message, view or event type. `npm run typecheck` will then point at
every place in the client that needs updating.

## Audio

`client/public/audio/` holds each sound twice: `.ogg` (Opus) for Chrome, desktop Discord and Android,
and `.m4a` (AAC) for Safari and iOS Discord. The client picks whichever the device can play.
`index.json` (written by the converter) records exact sample counts, so music loops land on the right
sample with no gap. In a match the Heaven and Hell themes crossfade according to which way the Soul
leans. `manifest.json` → `dynamic` sets how far it has to lean before the crossfade starts and finishes.

## Security notes

- The Client Secret and Bot token are only ever read by the server.
- Browser test logins (`?name=`) can claim any name, so they're for testing only:
  `SOULSCALE_DEV_LOGIN=off` in production.
- Every client message is size-capped (32 KB) and rate-limited (burst 40, then 10 per second).
- The server log holds connection IDs and Activity session IDs only, never names or Discord IDs, and
  saved matches are deleted after 12 months (`deploy/soulscale-tmpfiles.conf`). The Privacy Policy
  promises both, so keep them that way.
- With `DISCORD_BOT_TOKEN` set, the server confirms that an Activity instance exists before seating
  anyone in its room.
