#!/usr/bin/env python3
"""
Soulscale UI smoke test: drives real Chromium tabs against a running server.

It plays full games through the actual UI (clicking cards, Chambers, intents,
dice), checks every screen opens, watches the WebSocket traffic for leaks in
Duels, and fails on any JavaScript error. Screenshots land in --out.

Needs: a server with dev logins on (the default when no Discord secret is set),
       `pip install playwright` and either `playwright install chromium` or a
       local Chrome (--chrome PATH, or --channel chrome).

Windows:
    py -m pip install playwright
    py -m playwright install chromium
    py tools\\qa\\ui_smoke.py --base http://127.0.0.1:3001
"""
from __future__ import annotations

import argparse
import json
import random
import sys
import time
import uuid
from pathlib import Path

from playwright.sync_api import Browser, BrowserContext, Page, sync_playwright
from playwright.sync_api import TimeoutError as PWTimeout

FAST_PREFS = {
    "master": 0.0,
    "music": 0.0,
    "sfx": 0.0,
    "muted": True,
    "speed": 4,
    "reducedMotion": False,
    "showLog": True,
}


class Failure(Exception):
    pass


class Tab:
    """One browser tab as one named player, with error and frame capture."""

    def __init__(self, ctx: BrowserContext, base: str, name: str, room: str, out: Path, viewport=(1440, 900)):
        self.name = name
        self.out = out
        self.errors: list[str] = []
        self.frames: list[dict] = []
        self.page: Page = ctx.new_page()
        self.page.set_viewport_size({"width": viewport[0], "height": viewport[1]})
        self.page.on("pageerror", lambda e: self.errors.append(f"pageerror: {e}"))
        self.page.on("console", self._console)
        self.page.on("websocket", self._ws)
        self.page.goto(f"{base}/?name={name}&room={room}")
        self.page.wait_for_selector(".lobby", timeout=15000)

    def _console(self, msg) -> None:
        if msg.type == "error":
            text = msg.text
            # Chrome logs these for decoder fallbacks on some builds; not ours.
            if "favicon" in text:
                return
            self.errors.append(f"console: {text}")

    def _ws(self, ws) -> None:
        def got(payload):
            if isinstance(payload, (bytes, bytearray)):
                return
            try:
                self.frames.append(json.loads(payload))
            except json.JSONDecodeError:
                self.errors.append(f"non-JSON frame: {payload[:80]}")

        ws.on("framereceived", got)

    def shot(self, name: str) -> None:
        self.page.screenshot(path=str(self.out / f"{name}.png"))

    def server_errors(self) -> list[str]:
        return [f["message"] for f in self.frames if f.get("t") == "error"]

    def views(self) -> list[dict]:
        return [f["view"] for f in self.frames if f.get("t") == "game"]


# ------------------------------------------------------------------- driving


LIVE = ".modal-wrap:not(.closing)"


def click(loc, timeout=4000) -> bool:
    """Click if it is still there; the UI re-renders under us, so misses are normal."""
    try:
        loc.click(timeout=timeout)
        return True
    except PWTimeout:
        return False


def step(tab: Tab, rng: random.Random, reroll_rate=0.25) -> str:
    """Take one sensible action on this tab. Returns what it did ('' = nothing yet)."""
    p = tab.page
    if p.locator(f"{LIVE} .summary-modal").count():
        return "over"
    intent = p.locator(f"{LIVE} .intent-modal")
    if intent.count():
        opts = intent.locator("button.intent-tile:not(.weak), .btn.spend")
        n = opts.count()
        target = opts.nth(rng.randrange(n)) if n else intent.locator("button.intent-tile").first
        return "intent" if click(target) else ""
    curtain = p.locator(".curtain .btn")
    if curtain.count():
        return "unveil" if click(curtain.first) else ""
    play_it = p.locator("#g-action .btn.primary", has_text="Play it")
    if play_it.count():
        return "forced" if click(play_it) else ""
    keep = p.locator(".dice-panel .btn.primary", has_text="Keep")
    if keep.count():
        rr = p.locator(".dice-panel .btn.reroll:not([disabled])")
        if rr.count() and rng.random() < reroll_rate:
            return "reroll" if click(rr) else ""
        return "keep" if click(keep) else ""
    valid = p.locator(".chamber.valid")
    if valid.count():
        return "chamber" if click(valid.nth(rng.randrange(valid.count()))) else ""
    cards = p.locator(".hand .card.clickable")
    if cards.count():
        return "card" if click(cards.nth(rng.randrange(cards.count()))) else ""
    return ""


def play_until_over(tabs: list[Tab], rng: random.Random, max_secs: float, shots: dict[int, str] | None = None, label="") -> int:
    """Round-robin the tabs until a summary appears on the first one. Returns rounds seen."""
    t0 = time.time()
    taken = set()
    last_round = 0
    while time.time() - t0 < max_secs:
        acted = False
        for tab in tabs:
            what = step(tab, rng)
            if what == "over":
                return last_round
            if what:
                acted = True
                tab.page.wait_for_timeout(60)
        rnd = current_round(tabs[0])
        if rnd != last_round:
            last_round = rnd
            if shots and rnd in shots and rnd not in taken:
                taken.add(rnd)
                tabs[0].page.wait_for_timeout(250)
                tabs[0].shot(shots[rnd])
        if not acted:
            tabs[0].page.wait_for_timeout(120)
    raise Failure(f"{label}: no game over after {max_secs:.0f}s (round {last_round})")


def current_round(tab: Tab) -> int:
    vs = tab.views()
    return vs[-1]["round"] if vs else 0


def assert_no_errors(tabs: list[Tab]) -> None:
    bad = [f"[{t.name}] {e}" for t in tabs for e in t.errors]
    bad += [f"[{t.name}] server rejected: {m}" for t in tabs for m in t.server_errors()]
    if bad:
        raise Failure("JavaScript errors:\n  " + "\n  ".join(bad[:20]))


def open_mode(tab: Tab, title: str) -> None:
    tab.page.locator(".mode", has_text=title).click()
    tab.page.wait_for_selector(".setup")


def begin(tab: Tab, label: str) -> None:
    tab.page.locator(".setup .btn.primary", has_text=label).click()


def back_to_lobby(tab: Tab) -> None:
    p = tab.page
    if p.locator(f"{LIVE} .summary-modal").count():
        p.locator(f"{LIVE} .summary-modal .btn", has_text="Lobby").click()
    else:
        p.locator(".topbar .btn[title='Back to the lobby']").click()
    p.wait_for_selector(".lobby", timeout=10000)


# ----------------------------------------------------------------- scenarios


def scenario_screens(ctx: BrowserContext, base: str, out: Path) -> list[Tab]:
    tab = Tab(ctx, base, "Wofl", f"qa-screens-{uuid.uuid4().hex[:6]}", out)
    p = tab.page
    p.wait_for_timeout(600)
    tab.shot("01_lobby")
    links = p.locator(".lobby .legal-links a")
    hrefs = [links.nth(i).get_attribute("href") for i in range(links.count())]
    if hrefs != ["https://soulscale.store/tos/", "https://soulscale.store/privacy/"]:
        raise Failure(f"lobby legal links wrong: {hrefs}")
    # Outside Discord a click opens a new tab (never navigates the game away).
    with p.context.expect_page() as popup:
        links.nth(0).click()
    popup.value.close()
    if not p.locator(".lobby").count():
        raise Failure("clicking Terms navigated the game away")
    for nav, sel, name in [
        ("How to Play", ".help-page", "02_help"),
        ("Deck Builder", ".decks", "03_decks"),
        ("Sound Lab", ".soundlab", "04_soundlab"),
    ]:
        p.locator(".lobby-nav .btn", has_text=nav).click()
        p.wait_for_selector(sel, timeout=8000)
        p.wait_for_timeout(300)
        tab.shot(name)
        if name == "03_decks":
            # Build a deck by clicking the first ten library cards that fit.
            lib = p.locator(".lib-grid .card.clickable")
            for i in range(14):
                if i >= lib.count():
                    break
                lib.nth(i).click()
            p.wait_for_timeout(200)
            tab.shot("03b_decks_building")
        if name == "02_help":
            tabs = p.locator(".page-head .btn, .tabs .btn")
            for i in range(tabs.count()):
                label = tabs.nth(i).inner_text()
                if "Glyph" in label:
                    tabs.nth(i).click()
                    p.wait_for_timeout(300)
                    tab.shot("02b_glyphs")
        p.locator(".page-head .btn", has_text="Lobby").first.click()
        p.wait_for_selector(".lobby", timeout=8000)
    p.locator(".lobby-nav .btn", has_text="Settings").click()
    p.wait_for_selector(".settings")
    tab.shot("05_settings")
    p.locator(".modal .btn[title='Close']").click()
    p.wait_for_timeout(250)
    return [tab]


def scenario_vs_computer(ctx: BrowserContext, base: str, out: Path, rng: random.Random, profile: str | None, goal_label: str | None) -> list[Tab]:
    tab = Tab(ctx, base, "Wofl", f"qa-ai-{uuid.uuid4().hex[:6]}", out)
    p = tab.page
    open_mode(tab, "vs Computer")
    if profile:
        p.locator(".profile", has_text=profile).first.click()
    if goal_label:
        p.locator(".setup .seg .btn", has_text=goal_label).first.click()
    tab.shot("10_setup_vs_ai")
    begin(tab, "Begin")
    p.wait_for_selector(".game", timeout=10000)
    p.wait_for_timeout(900)
    tab.shot("11_game_start")
    # Show the selection states once for the record.
    p.locator(".hand .card.clickable").first.wait_for(timeout=10000)
    p.locator(".hand .card.clickable").first.click()
    p.wait_for_timeout(250)
    tab.shot("12_card_selected")
    valid = p.locator(".chamber.valid")
    if valid.count():
        valid.first.click()
        p.wait_for_selector(".intent-modal", timeout=5000)
        p.wait_for_timeout(250)
        tab.shot("13_intent_modal")
        p.locator(".intent-modal button.intent-tile").first.click()
        p.wait_for_selector(".dice-panel", timeout=8000)
        p.wait_for_timeout(500)
        tab.shot("14_dice")
    rounds = play_until_over([tab], rng, 600, shots={3: "15_round3", 6: "16_round6", 12: "17_round12"}, label="vs computer")
    p.wait_for_timeout(1200)
    tab.shot("18_summary")
    # Export through the summary.
    p.locator(f"{LIVE} .summary-modal .btn", has_text="Export").click()
    p.wait_for_selector(".report", timeout=8000)
    tab.shot("19_export")
    p.locator(".modal .btn[title='Close']").last.click()
    p.wait_for_timeout(250)
    print(f"  vs computer finished after {rounds} rounds")
    final = tab.views()[-1]
    if not final["over"] or final["winner"] not in ("heaven", "hell"):
        raise Failure("vs computer: last view is not a finished game")
    back_to_lobby(tab)
    return [tab]


def scenario_duel(browser: Browser, base: str, out: Path, rng: random.Random) -> list[Tab]:
    room = f"qa-duel-{uuid.uuid4().hex[:6]}"
    ctxs = [new_ctx(browser) for _ in range(3)]
    host = Tab(ctxs[0], base, "Wofl", room, out)
    guest = Tab(ctxs[1], base, "Bob", room, out)
    open_mode(host, "Duel")
    begin(host, "Open the Duel")
    guest.page.wait_for_selector(".duel-card", timeout=8000)
    guest.shot("20_duel_open_guest")
    guest.page.locator(".table-card .btn.primary", has_text="Accept").click()
    guest.page.wait_for_selector(".setup", timeout=5000)
    begin(guest, "Take the seat")
    host.page.wait_for_selector(".game", timeout=10000)
    guest.page.wait_for_selector(".game", timeout=10000)
    spec = Tab(ctxs[2], base, "Carol", room, out)
    spec.page.locator(".table-card .btn", has_text="Watch").click()
    spec.page.wait_for_selector(".game", timeout=10000)
    host.page.wait_for_timeout(800)
    host.shot("21_duel_host")
    guest.shot("22_duel_guest")
    # Host locks in first, then look at the guest's screen while it decides.
    for _ in range(6):
        if step(host, rng) in ("keep",):
            break
        host.page.wait_for_timeout(120)
    host.page.wait_for_timeout(400)
    host.shot("23_duel_host_waiting")
    spec.shot("24_duel_spectator")
    rounds = play_until_over([host, guest], rng, 600, label="duel")
    host.page.wait_for_timeout(1200)
    host.shot("25_duel_summary_host")
    guest.shot("26_duel_summary_guest")
    print(f"  duel finished after {rounds} rounds")
    check_duel_secrecy(host, my_idx=0)
    check_duel_secrecy(guest, my_idx=1)
    check_spectator(spec)
    return [host, guest, spec]


def scenario_secret_hotseat(ctx: BrowserContext, base: str, out: Path, rng: random.Random) -> list[Tab]:
    tab = Tab(ctx, base, "Wofl", f"qa-hot-{uuid.uuid4().hex[:6]}", out)
    p = tab.page
    open_mode(tab, "Pass & Play")
    cb = p.locator(".setup input[type=checkbox]")
    if cb.count() and not cb.first.is_checked():
        cb.first.check()
    begin(tab, "Begin")
    p.wait_for_selector(".game", timeout=10000)
    p.wait_for_timeout(700)
    tab.shot("30_secret_curtain")
    rounds = play_until_over([tab], rng, 600, shots={2: "31_secret_round2"}, label="secret hotseat")
    print(f"  secret hotseat finished after {rounds} rounds")
    return [tab]


def scenario_mobile(ctx: BrowserContext, base: str, out: Path, rng: random.Random) -> list[Tab]:
    tabs = []
    for label, size in [("portrait", (390, 844)), ("landscape", (844, 390))]:
        tab = Tab(ctx, base, f"Phone-{label}", f"qa-mob-{uuid.uuid4().hex[:6]}", out, viewport=size)
        tab.shot(f"40_mobile_{label}_lobby")
        open_mode(tab, "vs Computer")
        tab.shot(f"41_mobile_{label}_setup")
        begin(tab, "Begin")
        tab.page.wait_for_selector(".game", timeout=10000)
        tab.page.wait_for_timeout(900)
        tab.shot(f"42_mobile_{label}_game")
        did_intent = False
        for _ in range(40):
            what = step(tab, rng)
            if what == "chamber" and not did_intent:
                tab.page.wait_for_timeout(300)
                tab.shot(f"43_mobile_{label}_intent")
                did_intent = True
            if what == "over":
                break
            tab.page.wait_for_timeout(150)
            check_no_sideways_scroll(tab)
        tab.page.wait_for_timeout(500)
        tab.shot(f"44_mobile_{label}_later")
        tabs.append(tab)
    return tabs


def check_no_sideways_scroll(tab: Tab) -> None:
    w = tab.page.evaluate("""() => {
        const s = document.getElementById('screen');
        const g = document.querySelector('.game') || s;
        return [s.scrollWidth, s.clientWidth, g.scrollWidth, g.clientWidth];
    }""")
    if w[0] > w[1] + 1 or w[2] > w[3] + 1:
        raise Failure(f"{tab.name}: content is wider than the screen {w}")


# -------------------------------------------------------------------- checks


def check_duel_secrecy(tab: Tab, my_idx: int) -> None:
    opp = 1 - my_idx
    views = tab.views()
    if not views:
        raise Failure(f"{tab.name}: no game frames captured")
    for v in views:
        if v["viewer"] != {"kind": "seat", "seat": "P1" if my_idx == 0 else "P2"}:
            raise Failure(f"{tab.name}: unexpected viewer {v['viewer']}")
        o = v["seats"][opp]
        if o["hand"] is not None:
            raise Failure(f"{tab.name}: saw the opponent's hand in round {v['round']}")
        if o["status"] is not None or o["roll"] is not None:
            raise Failure(f"{tab.name}: saw the opponent's private status/roll in round {v['round']}")
        if o["public_status"] in ("choose", "reroll", "locked") and o["choice"] is not None:
            raise Failure(f"{tab.name}: saw the opponent's choice before the reveal (round {v['round']})")
    print(f"  {tab.name}: {len(views)} views, no leaks")


def check_spectator(tab: Tab) -> None:
    for v in tab.views():
        for s in v["seats"]:
            if s["hand"] is not None or s["status"] is not None or s["roll"] is not None:
                raise Failure(f"spectator saw private data in round {v['round']}")
            if s["public_status"] in ("choose", "reroll", "locked") and s["choice"] is not None:
                raise Failure(f"spectator saw a choice before the reveal (round {v['round']})")
    print(f"  {tab.name} (spectator): {len(tab.views())} views, no leaks")


# ---------------------------------------------------------------------- main


def new_ctx(browser: Browser) -> BrowserContext:
    ctx = browser.new_context(device_scale_factor=1)
    ctx.add_init_script(f"localStorage.setItem('soulscale.prefs', {json.dumps(json.dumps(FAST_PREFS))});")
    return ctx


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", default="http://127.0.0.1:3001")
    ap.add_argument("--out", default="qa-shots")
    ap.add_argument("--seed", type=int, default=7)
    ap.add_argument("--chrome", help="path to a Chrome/Chromium executable")
    ap.add_argument("--channel", help="Playwright browser channel, e.g. chrome or msedge")
    ap.add_argument("--headed", action="store_true")
    ap.add_argument("--only", nargs="*", default=None, help="screens ai duel hotseat mobile")
    ap.add_argument("--games", type=int, default=1, help="vs-computer games to play")
    a = ap.parse_args()

    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    rng = random.Random(a.seed)
    want = set(a.only or ["screens", "ai", "duel", "hotseat", "mobile"])
    all_tabs: list[Tab] = []
    with sync_playwright() as pw:
        launch = {
            "headless": not a.headed,
            "args": ["--autoplay-policy=no-user-gesture-required", "--disable-gpu-sandbox"],
        }
        if a.chrome:
            launch["executable_path"] = a.chrome
        elif a.channel:
            launch["channel"] = a.channel
        browser = pw.chromium.launch(**launch)
        try:
            if "screens" in want:
                print("screens…")
                all_tabs += scenario_screens(new_ctx(browser), a.base, out)
            if "ai" in want:
                profiles = ["Balanced", "Cruel Accountant", "Novice", "Architect", "Chaos Imp"]
                for g in range(a.games):
                    print(f"vs computer #{g + 1}…")
                    all_tabs += scenario_vs_computer(new_ctx(browser), a.base, out, rng, profiles[g % len(profiles)] if g else None, None)
            if "duel" in want:
                print("duel…")
                all_tabs += scenario_duel(browser, a.base, out, rng)
            if "hotseat" in want:
                print("secret hotseat…")
                all_tabs += scenario_secret_hotseat(new_ctx(browser), a.base, out, rng)
            if "mobile" in want:
                print("mobile…")
                all_tabs += scenario_mobile(new_ctx(browser), a.base, out, rng)
            assert_no_errors(all_tabs)
        except Failure as e:
            for t in all_tabs:
                try:
                    t.shot(f"zz_fail_{t.name}")
                except Exception:
                    pass
            print(f"FAIL: {e}")
            return 1
        finally:
            browser.close()
    print(f"PASS. Screenshots in {out.resolve()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
