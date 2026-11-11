# Soulscale: Purgatory

A single Soul hangs between Heaven and Hell. Two sides play glyphs into five Chambers and pull it home.

This is the Rust rewrite of the Python v1.0.1 prototype, built to run as a **Discord Activity**: a web
page inside Discord, backed by a small Rust server that owns the rules. It also runs in a plain browser
for testing.

## Status (November 2026)

- **Done:** full Rust rules engine (identical to the Python), server with Discord sign-in, all four
  modes (vs Computer, Duel, Pass & Play, secret Pass & Play), spectators, reconnects, AFK cover,
  saved replays, the new client (animated board, fattened glyphs, your music and SFX, Deck Builder,
  Sound Lab, How to Play), phone layouts, and the test suites below. All passing.
- **Not done yet:** a run inside real Discord. That needs your Developer Portal account, so it's
  the next step.
- **Next moves:** 1) [docs/DISCORD_SETUP.md](docs/DISCORD_SETUP.md) up to "Launch". 2) Play a Duel
  with a friend added as an App Tester. 3) Decide the balance questions in
  [docs/RULES_CHANGES.md](docs/RULES_CHANGES.md).

## Where things are

| Path | What it is |
|---|---|
| `crates/soulscale-core` | The game rules. Pure Rust, no I/O. Bit-for-bit identical to the Python under rules v1.0.1 (300 test games, 214,033 log lines). |
| `crates/soulscale-server` | Web server: Discord sign-in, matches over WebSocket, serves the client, saves logs and replays. |
| `crates/soulscale-sim` | Command-line balance simulator and replay checker. |
| `client/` | The page Discord shows: TypeScript + Vite. Board, cards, animations, sound, Deck Builder, Sound Lab. |
| `client/public/audio` | Your music and SFX as Ogg Opus + AAC, plus `manifest.json` (which sound plays when). |
| `tools/` | Audio conversion, the Python parity harness, the UI smoke test, Windows run scripts. |
| `docs/` | Setup, testing, rules changes, architecture. |
| `site/` | soulscale.store: Terms of Service, Privacy Policy, front page, and the NYC1 installer. |
| `deploy/` | systemd service and the 12-month match-record clean-up rule for the server. |

## Run it on Windows (browser test mode)

Needs: Rust 1.88+ (`winget install Rustlang.Rustup`; when rustup asks, let it install the Visual
Studio C++ build tools), Node 22+ (`winget install OpenJS.NodeJS.LTS`). Open a new terminal after
installing.

```powershell
cd soulscale
powershell -ExecutionPolicy Bypass -File tools\run.ps1
```

That builds the client, starts the server and opens `http://127.0.0.1:3001/?name=Wofl`.
To duel yourself, open a second tab at `http://127.0.0.1:3001/?name=Bob`.

Same thing by hand:

```powershell
cd client; npm ci; npm run build; cd ..
cargo run --release -p soulscale-server
```

## Put it in Discord

Follow **[docs/DISCORD_SETUP.md](docs/DISCORD_SETUP.md)**. In short: create the app in the Developer
Portal, put the Client ID and Secret in `.env`, run the server, open a Cloudflare tunnel to it, and point
the Activity's URL Mapping at the tunnel.

## Test it

**[docs/TESTING.md](docs/TESTING.md)** covers the Rust tests, the Python parity check, the browser
smoke test (plays full games through the real UI), the balance simulator and replays.

```powershell
cargo test --workspace
```

## Rules versions

New games default to **v1.0.2**, which fixes four bugs found in the Python. **v1.0.1** is still
selectable in the setup screen and is exactly the original. See [docs/RULES_CHANGES.md](docs/RULES_CHANGES.md),
including balance numbers from the simulator.

## Credits

Game design, music and sound: Wofl. Glyph art: Twemoji (CC-BY 4.0). Fonts: Cinzel and Cormorant
Garamond (SIL OFL 1.1). Details in [THIRD_PARTY.md](THIRD_PARTY.md).
