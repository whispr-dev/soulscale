#!/usr/bin/env python3
"""Render the Rich Presence art to soulscale.png, heaven.png and hell.png (1024x1024) in --out."""
import argparse, asyncio
from pathlib import Path
from playwright.async_api import async_playwright

HERE = Path(__file__).resolve().parent

async def main(out: Path, chrome: str | None):
    out.mkdir(parents=True, exist_ok=True)
    async with async_playwright() as pw:
        b = await pw.chromium.launch(**({"executable_path": chrome} if chrome else {}))
        p = await b.new_page(viewport={"width": 1024, "height": 3200}, device_scale_factor=1)
        await p.goto((HERE / "art.html").as_uri())
        await p.wait_for_load_state("networkidle")
        for key in ["soulscale", "heaven", "hell"]:
            # The badges are circles: keep the corners transparent so nothing shows outside Discord's mask.
            await p.locator(f"#{key}").screenshot(path=str(out / f"{key}.png"), omit_background=True)
            print(f"wrote {out / (key + '.png')}")
        await b.close()

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, default=Path("presence-art"))
    ap.add_argument("--chrome", default=None)
    a = ap.parse_args()
    asyncio.run(main(a.out, a.chrome))
