#!/usr/bin/env python3
"""
Record raw footage for the Discord "Video Preview": a few seconds of the lobby, then a whole
vs-Computer game played at a human pace (with a visible cursor), until the finale.

Frames come straight from Chrome's compositor (DevTools screencast) as high-quality JPEGs with
their real timestamps, so make_preview.py can cut and retime them without generation loss.

    python tools/preview/record.py --out preview-raw            # server must be running (dev logins on)
    python tools/preview/make_preview.py preview-raw            # -> soulscale-preview.mp4

Output: <out>/frames/*.jpg, <out>/frames.json (index -> timestamp), <out>/marks.json (what happened when).
"""
from __future__ import annotations

import argparse
import asyncio
import base64
import json
import math
import random
import re
import shutil
import time
from pathlib import Path

from playwright.async_api import async_playwright

VIEW_W, VIEW_H = 960, 720  # 4:3: the preview frames the game in a window with art either side

CURSOR_JS = r"""
(() => {
  const make = () => {
    const c = document.createElement('div');
    c.id = 'pv-cursor';
    c.innerHTML = '<svg width="26" height="26" viewBox="0 0 26 26"><path d="M3 2 L3 21 L8 16.5 L11.5 24 L15 22.5 L11.6 15.2 L18.5 15 Z" fill="#fff" stroke="#140b24" stroke-width="1.8" stroke-linejoin="round"/></svg>';
    Object.assign(c.style, {position:'fixed', left:'-40px', top:'-40px', zIndex: 2147483647, pointerEvents:'none',
      filter:'drop-shadow(0 2px 3px rgba(0,0,0,.6)) drop-shadow(0 0 6px rgba(200,180,255,.55))', transition:'transform .08s'});
    document.documentElement.appendChild(c);
    addEventListener('mousemove', e => { c.style.left = e.clientX - 3 + 'px'; c.style.top = e.clientY - 2 + 'px'; }, true);
    addEventListener('mousedown', () => c.style.transform = 'scale(.85)', true);
    addEventListener('mouseup', () => c.style.transform = '', true);
  };
  if (document.documentElement) make(); else addEventListener('DOMContentLoaded', make);
})();
"""

PREFS = {"master": 0.0, "music": 0.0, "sfx": 0.0, "muted": True, "speed": 1,
         "reducedMotion": False, "showLog": False}


class Recorder:
    def __init__(self, out: Path):
        self.dir = out / "frames"
        if self.dir.exists():
            shutil.rmtree(self.dir)
        self.dir.mkdir(parents=True)
        self.frames: list[tuple[int, float]] = []
        self.marks: list[tuple[float, str]] = []
        self.cdp = None
        self.t0 = None

    async def start(self, page):
        self.cdp = await page.context.new_cdp_session(page)
        self.cdp.on("Page.screencastFrame", lambda p: asyncio.ensure_future(self._frame(p)))
        await self.cdp.send("Page.startScreencast", {"format": "jpeg", "quality": 92,
                                                      "maxWidth": VIEW_W, "maxHeight": VIEW_H, "everyNthFrame": 1})

    async def _frame(self, p):
        ts = p["metadata"]["timestamp"]
        if self.t0 is None:
            self.t0 = ts
        i = len(self.frames)
        (self.dir / f"{i:06d}.jpg").write_bytes(base64.b64decode(p["data"]))
        self.frames.append((i, ts - self.t0))
        try:
            await self.cdp.send("Page.screencastFrameAck", {"sessionId": p["sessionId"]})
        except Exception:
            pass

    def mark(self, what: str):
        t = (time.time() - self.wall0) if hasattr(self, "wall0") else 0.0
        self.marks.append((t, what))
        print(f"  {t:7.2f}s {what}")

    async def stop(self, out: Path):
        await self.cdp.send("Page.stopScreencast")
        await asyncio.sleep(0.3)
        (out / "frames.json").write_text(json.dumps(self.frames))
        (out / "marks.json").write_text(json.dumps(self.marks, indent=1))


class Hand:
    """Glides a visible cursor like a person would, rather than teleporting."""

    def __init__(self, page):
        self.page = page
        self.x, self.y = VIEW_W * 0.55, VIEW_H * 0.9

    async def glide(self, x: float, y: float, ms: int = 380):
        steps = max(8, ms // 16)
        x0, y0 = self.x, self.y
        for k in range(1, steps + 1):
            t = k / steps
            e = t * t * (3 - 2 * t)  # smoothstep
            await self.page.mouse.move(x0 + (x - x0) * e, y0 + (y - y0) * e)
            await asyncio.sleep(ms / steps / 1000)
        self.x, self.y = x, y

    async def to(self, locator, ms: int = 380, dx: float = 0.5, dy: float = 0.45):
        try:
            await locator.scroll_into_view_if_needed(timeout=2000)
        except Exception:
            return False
        box = await locator.bounding_box()
        if not box:
            return False
        await self.glide(box["x"] + box["width"] * dx, box["y"] + box["height"] * dy, ms)
        return True

    async def click(self, locator, ms: int = 380, pause: float = 0.12, dx: float = 0.5, dy: float = 0.45):
        if not await self.to(locator, ms, dx, dy):
            return False
        await asyncio.sleep(pause)
        await self.page.mouse.down()
        await asyncio.sleep(0.07)
        await self.page.mouse.up()
        return True


async def best_chamber(page):
    """The glowing Chamber with the biggest Press bonus preview (ties broken randomly)."""
    cells = page.locator(".chamber.valid")
    n = await cells.count()
    best, score = [], -1
    for i in range(n):
        txt = await cells.nth(i).locator(".preview").all_inner_texts()
        m = re.search(r"\+(\d+)", " ".join(txt))
        s = int(m.group(1)) if m else 0
        if s > score:
            best, score = [i], s
        elif s == score:
            best.append(i)
    return cells.nth(random.choice(best)) if best else None


async def play_turn(page, hand: Hand, rec: Recorder, turn: int) -> str:
    """Play one turn the way a person would. Returns 'over', 'played' or 'wait'."""
    if await page.locator(".modal-wrap:not(.closing) .summary-modal").count():
        return "over"
    cards = page.locator(".hand .card.clickable")
    n = await cards.count()
    if n == 0 or await page.locator(".chamber.valid").count():
        return "wait"
    # Browse the hand a little (shows the 3D tilt), then pick.
    order = list(range(n))
    random.shuffle(order)
    for i in order[: 2 if turn % 2 == 0 else 1]:
        await hand.to(cards.nth(i), 420, 0.5, 0.4)
        await hand.glide(hand.x + random.uniform(-14, 14), hand.y + random.uniform(-18, 10), 260)
        await asyncio.sleep(0.15)
    pick = cards.nth(order[0])
    await hand.click(pick, 300)
    rec.mark(f"turn {turn}: card")
    await asyncio.sleep(0.55)
    ch = await best_chamber(page)
    if ch is None:
        return "wait"
    await hand.click(ch, 480, 0.18, 0.5, 0.5)
    rec.mark(f"turn {turn}: chamber")
    try:
        await page.wait_for_selector(".modal-wrap:not(.closing) .intent-modal button.intent-tile", timeout=4000)
    except Exception:
        return "played"
    await asyncio.sleep(0.45)
    tile = page.locator(".modal-wrap:not(.closing) .intent-modal button.intent-tile").first
    await hand.click(tile, 360, 0.2)
    rec.mark(f"turn {turn}: intent")
    try:
        await page.wait_for_selector(".dice-panel .btn.primary:has-text('Keep')", timeout=8000)
    except Exception:
        return "played"
    await asyncio.sleep(0.75)
    reroll = page.locator(".dice-panel .btn.reroll:not([disabled])")
    if turn == 1 and await reroll.count():
        await hand.click(reroll, 380, 0.15)
        rec.mark(f"turn {turn}: reroll")
    else:
        await hand.click(page.locator(".dice-panel .btn.primary:has-text('Keep')"), 380, 0.15)
        rec.mark(f"turn {turn}: keep")
    await hand.glide(VIEW_W * random.uniform(0.35, 0.6), VIEW_H * random.uniform(0.45, 0.6), 700)
    return "played"


async def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="http://127.0.0.1:3001")
    ap.add_argument("--out", default="preview-raw")
    ap.add_argument("--name", default="Player")
    ap.add_argument("--side", default="Heaven")
    ap.add_argument("--goal", default="Quick")
    ap.add_argument("--seed", type=int, default=3)
    ap.add_argument("--chrome", default=None)
    ap.add_argument("--max-minutes", type=float, default=8)
    a = ap.parse_args()
    random.seed(a.seed)
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)

    async with async_playwright() as pw:
        launch = {"headless": True, "args": ["--autoplay-policy=no-user-gesture-required", "--hide-scrollbars"]}
        if a.chrome:
            launch["executable_path"] = a.chrome
        browser = await pw.chromium.launch(**launch)
        ctx = await browser.new_context(viewport={"width": VIEW_W, "height": VIEW_H}, device_scale_factor=1)
        await ctx.add_init_script(f"localStorage.setItem('soulscale.prefs', {json.dumps(json.dumps(PREFS))});")
        await ctx.add_init_script(CURSOR_JS)
        page = await ctx.new_page()
        room = f"preview-{int(time.time())}"
        await page.goto(f"{a.base}/?name={a.name}&room={room}")
        await page.wait_for_selector(".lobby")
        await page.evaluate("document.fonts.ready")
        await asyncio.sleep(1.0)

        rec = Recorder(out)
        hand = Hand(page)
        rec.wall0 = time.time()
        await rec.start(page)
        rec.mark("lobby")
        await page.mouse.move(hand.x, hand.y)
        await asyncio.sleep(2.2)
        await hand.to(page.locator(".mode", has_text="vs Computer"), 900)
        await asyncio.sleep(0.9)
        await hand.click(page.locator(".mode", has_text="vs Computer"), 200)
        await page.wait_for_selector(".setup")
        rec.mark("setup")
        await asyncio.sleep(0.6)
        await hand.click(page.locator(".side", has_text=a.side), 450)
        await asyncio.sleep(0.3)
        await hand.click(page.locator(".setup .seg .btn", has_text=a.goal).first, 450)
        await asyncio.sleep(0.3)
        await hand.click(page.locator(".setup .btn.primary", has_text="Begin"), 500)
        await page.wait_for_selector(".game")
        rec.mark("game")

        turn, deadline = 0, time.time() + a.max_minutes * 60
        while time.time() < deadline:
            r = await play_turn(page, hand, rec, turn + 1)
            if r == "over":
                rec.mark("summary")
                break
            if r == "played":
                turn += 1
            await asyncio.sleep(0.15)
            if await page.locator("body.finale-heaven, body.finale-hell").count() and not any(m == "finale" for _, m in rec.marks):
                rec.mark("finale")
        await asyncio.sleep(2.5)
        rec.mark("end")
        await rec.stop(out)
        await browser.close()
    fr = json.loads((out / "frames.json").read_text())
    dur = fr[-1][1] if fr else 0
    print(f"{len(fr)} frames over {dur:.1f}s ({len(fr) / max(dur, 0.01):.1f} fps), {turn} turns")


if __name__ == "__main__":
    asyncio.run(main())
