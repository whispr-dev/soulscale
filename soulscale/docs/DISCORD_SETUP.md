# Putting Soulscale in Discord

**Where you are:** the game runs locally in a browser (`tools\run.ps1`).
**Where this gets you:** Soulscale launching from Discord's App Launcher in your own test server,
then (later) hosted for real.

## How it fits together

A Discord Activity is a web page that Discord shows inside a frame. Discord never talks to your
machine directly; it proxies everything through `https://<your-app-id>.discordsays.com/`, using the
**URL Mapping** you set in the Developer Portal to find your server.

```
Discord app ──> <app-id>.discordsays.com ──(URL Mapping "/")──> your server (soulscale-server)
                                                                 ├─ serves the page (client/dist)
                                                                 ├─ /api/token   swaps the login code for a token
                                                                 └─ /ws          the live game (WebSocket)
```

Sign-in happens with no password prompt: the page asks Discord for a one-time code, the server
trades it for a token using your **Client Secret** (which never leaves the server), then asks Discord
who the token belongs to. Everyone launched from the same Activity window shares one room, so a Duel
is "launch it in a voice channel, your mate launches it too, click Accept".

## One-time setup (about 10 minutes)

### 1. Turn on Developer Mode in Discord

Discord → User Settings → **Advanced** → **Developer Mode** on. Without it, Discord won't let you
launch an Activity that's still in development.

### 2. Create the app

1. Go to https://discord.com/developers/applications → **New Application** → name it `Soulscale`.
   If you'll ever have collaborators, create a **Team** first and make it the owner.
2. **General Information**: add an icon and a description. You can change these any time.

### 3. Installation

Left sidebar → **Installation** → under **Installation Contexts** tick both **User Install** and
**Guild Install**. This lets people launch it in servers, DMs and group DMs.

### 4. OAuth2

1. Left sidebar → **OAuth2** → **Redirects** → add `https://127.0.0.1` → **Save Changes**.
   It's a placeholder. Discord's SDK handles the real redirect, but the field can't be empty.
2. Copy the **Client ID**. Click **Reset Secret** and copy the **Client Secret**.
3. In the repo root, copy `.env.example` to `.env` and paste them in:

   ```
   DISCORD_CLIENT_ID=123456789012345678
   DISCORD_CLIENT_SECRET=abc...xyz
   ```

   `.env` is in `.gitignore`. Never commit it and never paste the secret into chat or a screenshot.
   If it leaks, Reset Secret again.

4. While testing, add `SOULSCALE_DEV_LOGIN=on` to `.env` too if you still want the `?name=` browser
   tabs to work alongside Discord. (With a secret set they default to off.)

### 5. Turn on Activities

Left sidebar → **Activities** → **Settings**:

- Tick **Enable Activities**. This also creates the "Launch" entry point command, which is what puts
  Soulscale in the App Launcher.
- **Supported platforms**: tick Web (desktop/browser), iOS and Android. Desktop alone is the safe
  first test; the client already has phone layouts.
- **Default orientation lock** for phones: set **Portrait**. The portrait layout is the one built
  for phones (cards along the bottom, under your thumb). Tablets can stay unlocked.

### 6. Rich Presence pictures

This is the status under a player's name in Discord, e.g. "Playing Soulscale: Purgatory · Duel as
Heaven · Round 7 · Soul +6 of ±18", with a match timer and a Heaven or Hell badge. The game sets the
words itself. Discord only needs the pictures:

1. Left sidebar → **Rich Presence** → **Art Assets** → **Add Image(s)**.
2. Upload `soulscale.png`, `heaven.png` and `hell.png` from `tools/presence/out/`.
3. Make sure each one's key (the name under it) is exactly `soulscale`, `heaven` or `hell`. The game
   asks for them by those names. Save. New uploads can take a few minutes to appear.

The **Visualizer** page next to it is only a preview playground. The C code at the bottom is for
Discord's old library for native games and isn't used by an Activity. Nothing typed there is saved.

### 7. (Optional) Bot token for instance checks

Left sidebar → **Bot** → **Reset Token** → put it in `.env` as `DISCORD_BOT_TOKEN=...`.
The server then asks Discord to confirm each Activity instance really exists before seating anyone
in it. Without it everything still works; the room ID is just taken on trust. The bot needs no
permissions and doesn't need to join any server.

## Every test session

1. **Start the server and a tunnel**:

   ```powershell
   winget install --id Cloudflare.cloudflared     # once
   powershell -ExecutionPolicy Bypass -File tools\run.ps1 -Tunnel -NoBrowser
   ```

   A second window opens with cloudflared. Find the line with `https://something-random.trycloudflare.com`.

2. **Point Discord at it**: Developer Portal → **Activities** → **URL Mappings** → the `/` row →
   **Target** = `something-random.trycloudflare.com` (no `https://`, no trailing slash) → Save.

   A quick tunnel gets a **new random name every time you start it**, so repeat this step each
   session. (A named Cloudflare tunnel on your own domain keeps one name; see "Hosting it for real".)

3. **Launch**: in a server you own with fewer than 25 members, join a voice channel (or open any text
   channel), open the **App Launcher** (the rocket icon), find **Soulscale**, click it. The first
   time, Discord asks you to **Authorize**: that's the sign-in.

4. **Play someone**: they join the same voice channel and launch Soulscale too (or click the
   Activity's join prompt). You both land in the same room. One of you opens a Duel; the other
   clicks **Accept**. Anyone else who joins can **Watch**.

### Hot reload while you tweak the client

Run the server (`tools\run.ps1 -NoBrowser`), then in another terminal `cd client; npm run dev`, and
tunnel to Vite instead: `cloudflared tunnel --url http://localhost:5173`. Vite forwards `/api` and
`/ws` to the server and reloads the page in Discord when you save a file.

## Letting friends test before release

Until Discord verifies the app it's **unverified**. Only you, your team and up to 50 **App Testers**
can launch it, and only in servers with fewer than 25 members.

Developer Portal → your app → **App Testers** → add them by username. Each tester needs Developer
Mode on (step 1). Testers should use servers that have fewer than 25 members.

What to try with them (also in [TESTING.md](TESTING.md)):

- A full Duel on desktop, then one on a phone.
- Close Discord mid-Duel and reopen it within 30 seconds: you should get your seat back. After 30
  seconds the AI covers the seat; reconnecting hands it back.
- Leave a turn for 2 minutes: the server plays for the idle player so the table never stalls.
- A third person watching: they must never see either hand or a chosen card before the reveal.
- Picture-in-picture: start a game, then click away to a text channel. The small floating window
  should show just the scale and round.

## Hosting it for real (NYC1, soulscale.store)

A tunnel on your PC is fine for testing, but the game stops when your PC sleeps. For launch, the game
runs on NYC1 at `https://soulscale.store`, behind the Caddy that's already there.

**1. The legal pages and the front page** (do this first, it doesn't need the game):
follow [site/README.md](../site/README.md). That puts `/tos/` and `/privacy/` live and shows a
"coming soon" page at `/`.

**2. The game server.** No GitHub, Rust or Node needed on NYC1: you copy a ready-built bundle over
and run its installer. Build the bundle on any Linux box or WSL with `./tools/package-linux.sh`
(it makes `soulscale-linux.tar.gz`), then from PowerShell on your PC:

```powershell
cd $HOME\Downloads                       # wherever soulscale-linux.tar.gz is
scp soulscale-linux.tar.gz nyc1:~/
ssh nyc1
```

then on NYC1:

```bash
tar xzf soulscale-linux.tar.gz
sudo bash soulscale/install.sh          # asks for the Discord keys the first time; Enter skips
```

The installer checks the server can run it, creates the `soulscale` service account, installs to
`/opt/soulscale`, writes `.env` (browser test logins off), installs the systemd service and the
12-month match clean-up rule, starts it and checks `/api/health`. It stops at the first problem.
Re-running it updates the server and keeps `.env` and saved matches. If port 3001 is taken:
`sudo BIND=127.0.0.1:3005 bash soulscale/install.sh`.

**3. Switch the front page to the game:** run `sudo bash ~/soulscale-site/install.sh` again (the
server installer prints the exact command). It sees the game answering and sends everything except
`/tos/` and `/privacy/` to it.

**4. Point Discord at it:** Developer Portal → Activities → URL Mappings → target for `/` =
`soulscale.store`. No tunnel, no PC needed.

Updating the game later: build a new bundle, copy it over, `tar xzf` it and run
`sudo bash soulscale/install.sh` again.

## Going public

When it's ready for everyone: Developer Portal → **Verification** (the app needs to be
team-owned) and then **Discovery**. The portal shows a checklist of what it wants, which includes
Terms of Service and Privacy Policy links (`https://soulscale.store/tos/` and
`https://soulscale.store/privacy/`, under **General Information**) and the Activity art (cover image,
etc.). Expect the checklist to change over time; follow what it says.

The hover **Video Preview** (640x360 MP4, 10 s, under 0.5 MB) is made by `tools/preview/`; see its
README. The current one is `soulscale-preview.mp4`.

Selling things later (cards, packs): Discord requires that anything you sell elsewhere, such as through
Stripe on soulscale.store, can also be bought inside Discord, at a price on Discord no higher than
anywhere else. Monetization also needs a verified, team-owned app. Update the policies' "card shop"
sections before the first sale.

The published Privacy Policy (`site/www/privacy/index.html`) describes exactly what the game
collects and logs today. If you change what the game collects or keeps, change the policy in the same
commit.

## When something's wrong

| Symptom | Likely cause |
|---|---|
| Blank or grey Activity window | URL Mapping target wrong or tunnel stopped. Open `https://<target>/api/health` in a browser: it should say `ok`. |
| "Discord sign-in failed" | Client Secret missing/wrong in `.env`, or the OAuth2 redirect wasn't saved. Check the server log. |
| Stuck on "Connecting…" | The WebSocket can't get through. Confirm the URL Mapping is for prefix `/` (not `/api`). |
| No sound | Browsers block audio until the first tap or click. Tap anything once. |
| Works on desktop, not phone | iOS/Android not ticked in Activities → Settings, or the tester's Discord app needs updating. |
| Friend can't see the Activity | Not added as an App Tester, Developer Mode off, or the server has 25+ members. |

Server log detail: set `RUST_LOG=debug` in `.env` and restart.
