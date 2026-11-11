#!/usr/bin/env python3
"""
Build the Discord "Video Preview" (640x360, H.264 MP4, 10 s, under 0.5 MB) from footage made by
record.py: the game framed in a 4:3 window with the angel/demon artwork (art.html) either side.

    python tools/preview/make_preview.py sheet preview-raw 140 165      # contact sheet to choose moments
    python tools/preview/make_preview.py build preview-raw --cuts tools/preview/cuts.json

cuts.json lists the moments to use, in order: source start/end seconds and a 4:3 crop (in the
960x720 recording) at the start and end of each cut, which the build eases between (a slow zoom).
Cuts crossfade into each other. The total is fitted to exactly 10.0 s.
Needs: Playwright + Chromium (to render art.html), Pillow, ffmpeg with libx264.
"""
from __future__ import annotations

import argparse
import asyncio
import bisect
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw

W, H, FPS, SECONDS = 640, 360, 30, 10.0
WIN = (100, 15, 440, 330)  # game window inside the artwork: x, y, w, h (4:3)
XFADE = 0.3                # seconds of crossfade between cuts
MAX_BYTES = 480_000        # Discord says 0.5 MB; stay under it whichever MB they mean
HERE = Path(__file__).resolve().parent


class Footage:
    def __init__(self, raw: Path):
        self.raw = raw
        fr = json.loads((raw / "frames.json").read_text())
        self.idx = [i for i, _ in fr]
        self.ts = [t for _, t in fr]
        self.cache: dict[int, Image.Image] = {}

    def at(self, t: float) -> Image.Image:
        k = self.idx[max(0, bisect.bisect_right(self.ts, t) - 1)]
        if k not in self.cache:
            if len(self.cache) > 64:
                self.cache.clear()
            self.cache[k] = Image.open(self.raw / "frames" / f"{k:06d}.jpg").convert("RGB")
        return self.cache[k]


def ease(u: float) -> float:
    return u * u * (3 - 2 * u)


def render_art(out_png: Path, chrome: str | None) -> Image.Image:
    from playwright.async_api import async_playwright

    async def go():
        async with async_playwright() as pw:
            kw = {"executable_path": chrome} if chrome else {}
            b = await pw.chromium.launch(**kw)
            p = await b.new_page(viewport={"width": W, "height": H}, device_scale_factor=4)
            await p.goto((HERE / "art.html").as_uri())
            await p.locator("#art").screenshot(path=str(out_png), omit_background=True)
            await b.close()

    asyncio.run(go())
    return Image.open(out_png).convert("RGBA").resize((W, H), Image.LANCZOS)


def sheet(raw: Path, t0: float, t1: float, step: float, out: Path):
    foot = Footage(raw)
    times, t = [], t0
    while t <= t1 + 1e-9:
        times.append(t)
        t += step
    cols, tw, th = 4, 320, 240
    rows = (len(times) + cols - 1) // cols
    img = Image.new("RGB", (cols * tw, rows * (th + 18)))
    d = ImageDraw.Draw(img)
    for k, t in enumerate(times):
        x, y = (k % cols) * tw, (k // cols) * (th + 18)
        img.paste(foot.at(t).resize((tw, th)), (x, y + 18))
        d.text((x + 4, y + 3), f"{t:.2f}s", fill=(255, 255, 0))
    img.save(out)
    print(f"wrote {out}")


def build(raw: Path, cuts_file: Path, out: Path, chrome: str | None, keep_frames: bool):
    cuts = json.loads(cuts_file.read_text())["cuts"]
    foot = Footage(raw)
    work = Path(tempfile.mkdtemp(prefix="ssprev-"))
    art = render_art(work / "art_big.png", chrome)
    art.save(work / "art.png")

    # Fit the cuts to exactly SECONDS, keeping their proportions.
    total_src = sum(c["end"] - c["start"] for c in cuts)
    budget = SECONDS + XFADE * (len(cuts) - 1)
    rate = total_src / budget  # >1 means a touch faster than real time
    lens = [(c["end"] - c["start"]) / rate for c in cuts]
    starts, t = [], 0.0
    for L in lens:
        starts.append(t)
        t += L - XFADE
    print(f"{len(cuts)} cuts, playback rate x{rate:.3f}")

    def cut_frame(ci: int, local: float) -> Image.Image:
        c = cuts[ci]
        u = min(max(local / lens[ci], 0.0), 1.0)
        src_t = c["start"] + local * rate
        a, b = c["crop_from"], c["crop_to"]
        e = ease(u)
        x, y, w = (a[0] + (b[0] - a[0]) * e, a[1] + (b[1] - a[1]) * e, a[2] + (b[2] - a[2]) * e)
        h = w * 3 / 4
        return foot.at(src_t).resize((WIN[2], WIN[3]), Image.LANCZOS, box=(x, y, x + w, y + h))

    frames_dir = work / "frames"
    frames_dir.mkdir()
    n_frames = int(round(SECONDS * FPS))
    for n in range(n_frames):
        T = n / FPS
        active = [ci for ci in range(len(cuts)) if starts[ci] <= T < starts[ci] + lens[ci]] or [len(cuts) - 1]
        game = cut_frame(active[0], T - starts[active[0]])
        if len(active) > 1:
            ci = active[1]
            mix = min(max((T - starts[ci]) / XFADE, 0.0), 1.0)
            game = Image.blend(game, cut_frame(ci, T - starts[ci]), mix)
        canvas = Image.new("RGBA", (W, H), (0, 0, 0, 255))
        canvas.paste(game, (WIN[0], WIN[1]))
        canvas.alpha_composite(art)
        canvas.convert("RGB").save(frames_dir / f"{n:04d}.png")
    print(f"composed {n_frames} frames")

    # Two-pass H.264 aimed just under the size cap. No audio: Discord plays previews muted.
    kbps = 360
    while True:
        common = ["-framerate", str(FPS), "-i", str(frames_dir / "%04d.png"), "-c:v", "libx264",
                  "-preset", "veryslow", "-tune", "animation", "-profile:v", "high", "-level", "3.1",
                  "-pix_fmt", "yuv420p", "-b:v", f"{kbps}k", "-maxrate", f"{kbps * 2}k", "-bufsize", f"{kbps * 2}k",
                  "-g", str(FPS * 2), "-an"]
        subprocess.run(["ffmpeg", "-y", "-loglevel", "error", *common, "-pass", "1",
                        "-passlogfile", str(work / "x264"), "-f", "mp4", "/dev/null"], check=True)
        subprocess.run(["ffmpeg", "-y", "-loglevel", "error", *common, "-pass", "2",
                        "-passlogfile", str(work / "x264"), "-movflags", "+faststart", str(out)], check=True)
        size = out.stat().st_size
        print(f"{out}: {size:,} bytes at {kbps} kb/s")
        if size <= MAX_BYTES or kbps <= 150:
            break
        kbps -= 30
    if keep_frames:
        print(f"frames kept in {frames_dir}")
    else:
        shutil.rmtree(work)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("sheet")
    s.add_argument("raw", type=Path)
    s.add_argument("t0", type=float)
    s.add_argument("t1", type=float)
    s.add_argument("--step", type=float, default=0.75)
    s.add_argument("--out", type=Path, default=Path("sheet.png"))
    b = sub.add_parser("build")
    b.add_argument("raw", type=Path)
    b.add_argument("--cuts", type=Path, default=HERE / "cuts.json")
    b.add_argument("--out", type=Path, default=Path("soulscale-preview.mp4"))
    b.add_argument("--chrome", default=None)
    b.add_argument("--keep-frames", action="store_true")
    a = ap.parse_args()
    if a.cmd == "sheet":
        sheet(a.raw, a.t0, a.t1, a.step, a.out)
    else:
        if not shutil.which("ffmpeg"):
            sys.exit("ffmpeg not found")
        build(a.raw, a.cuts, a.out, a.chrome, a.keep_frames)


if __name__ == "__main__":
    main()
