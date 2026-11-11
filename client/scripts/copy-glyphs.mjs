// Copy just the Twemoji SVGs Soulscale uses into public/glyphs/.
// Card glyphs come from the Rust card data, so new cards are picked up automatically.
//   npm run glyphs
// Twemoji graphics: (c) Twitter, Inc and other contributors, CC-BY 4.0
// (https://creativecommons.org/licenses/by/4.0/). Attribution is shown in the in-game Credits.
import { readFileSync, mkdirSync, copyFileSync, existsSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const cardsPath = join(here, "../../crates/soulscale-core/data/cards.json");
const src = join(here, "../node_modules/@twemoji/svg");
const out = join(here, "../public/glyphs");

// Chambers, conditions, Purgatory events, UI.
const extra = "🫀 🧠 ❤️ 🕰️ 🪞 ❄️ 🥀 🕳️ 🌤️ 🔥 ⚖️ 🔒 😇 😈 🎲 ✨ 🕯️ 👁️ 🌙 ☁️ 🪽 ⛓️".split(" ");

export function glyphFile(g) {
  return [...g].map((c) => c.codePointAt(0).toString(16)).filter((h) => h !== "fe0f").join("-");
}

const cards = JSON.parse(readFileSync(cardsPath, "utf8")).cards;
const glyphs = new Set([...cards.map((c) => c.emoji), ...extra]);
mkdirSync(out, { recursive: true });
let n = 0;
const missing = [];
for (const g of glyphs) {
  const f = glyphFile(g);
  const from = join(src, `${f}.svg`);
  if (!existsSync(from)) {
    missing.push(`${g} (${f})`);
    continue;
  }
  copyFileSync(from, join(out, `${f}.svg`));
  n++;
}
writeFileSync(join(out, "LICENSE.txt"), "Twemoji graphics (c) Twitter, Inc and other contributors.\nLicensed under CC-BY 4.0: https://creativecommons.org/licenses/by/4.0/\n");
console.log(`copied ${n} glyph SVGs to public/glyphs`);
if (missing.length) {
  console.error(`missing from @twemoji/svg (will fall back to system emoji): ${missing.join(", ")}`);
  process.exitCode = 1;
}
