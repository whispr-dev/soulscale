// How to Play: condensed rulebook, keyword glossary, Purgatory events, full card gallery,
// and credits/licences.

import { app } from "../app";
import { h, mount } from "../dom";
import { glyph } from "../glyph";
import type { Faction } from "../protocol";
import { CHAMBER_GLYPH, CHAMBERS } from "../protocol";
import { factionCards } from "../store";
import { btn, cardEl, FACTION_GLYPH } from "./components";
import { PURGATORY_TEXT } from "./anim";
import { legalLinks } from "./legal";

const KEYWORDS: [string, string][] = [
  ["Heavy", "Spends 1 Will automatically. With no Will left, the glyph loses 1 force."],
  ["Fragile", "Loses 1 force if the opponent plays the same Chamber."],
  ["Volatile", "Raw 1–2: −2 force and +1 Strain. Raw 5–6: +1 force."],
  ["Delayed", "Its Soul movement lands at the start of next round."],
  ["Binding", "You can't use that Chamber next round."],
  ["Hungry", "Not with Release: eats 1 friendly stack for +1 force, or −1 force if there's none."],
  ["Echo", "If it moved the Soul, a ±1 echo lands next round."],
  ["Desperate", "+1 while your side is behind, −1 while ahead."],
  ["Narrow", "Few legal Chambers. The restriction is the price."],
];

const EVENTS: [string, string, string][] = [
  ["🕳️", "Collapse", PURGATORY_TEXT.Collapse],
  ["🌤️", "Reprieve", PURGATORY_TEXT.Reprieve],
  ["🪞", "Flashback", PURGATORY_TEXT.Flashback],
  ["🕰️", "Stasis", PURGATORY_TEXT.Stasis],
  ["🔥", "Intensification", PURGATORY_TEXT.Intensification],
  ["⚖️", "Judgement", "Side behind +2 Will next round; if level, both +1."],
];

let tab: "rules" | "cards" | "credits" = "rules";
let galleryFaction: Faction = "heaven";

export function renderHelp(screen: HTMLElement): void {
  const redraw = () => renderHelp(screen);
  const tabs = h(
    "div",
    { class: "seg" },
    btn("Rules", () => ((tab = "rules"), redraw()), tab === "rules" ? "on" : ""),
    btn("Glyph gallery", () => ((tab = "cards"), redraw()), tab === "cards" ? "on" : ""),
    btn("Credits", () => ((tab = "credits"), redraw()), tab === "credits" ? "on" : ""),
  );
  let body: HTMLElement;
  if (tab === "rules") body = rules();
  else if (tab === "cards") body = gallery(redraw);
  else body = credits();
  mount(screen, h("div", { class: "page help-page" }, h("header", { class: "page-head" }, btn("← Lobby", () => app.go("lobby"), "ghost"), h("h2", null, "How to Play"), tabs), body));
}

function rules(): HTMLElement {
  return h(
    "div",
    { class: "rules" },
    h(
      "section",
      null,
      h("h3", null, "The goal"),
      h("p", null, "One Soul sits on a scale at 0. Heaven wins when it reaches the top (+12, +18 or +24); Hell wins at the bottom. Every round both sides play one glyph at the same time, then everything resolves together."),
    ),
    h(
      "section",
      null,
      h("h3", null, "A round"),
      h(
        "ol",
        null,
        h("li", null, "You get 3 Will (plus or minus bonuses), and 4 glyphs are dealt from your 10-glyph deck."),
        h("li", null, "Pick a glyph, a Chamber it can enter, and an Intent."),
        h("li", null, "Roll the intensity die: 1–2 weak (×1), 3–4 steady (×2), 5–6 surge (×3). You may spend 1 Will to reroll once. Each 6 adds Strain."),
        h("li", null, "Both plays are revealed. Guards, traps and keywords adjust them."),
        h("li", null, "Same Chamber? They clash: higher power wins and the loser's effect is halved (tie: both halved)."),
        h("li", null, "Plays apply, ongoing effects pulse, the Soul moves. At 3 Strain a Purgatory Event fires."),
      ),
    ),
    h(
      "section",
      null,
      h("h3", null, "The five Chambers"),
      h("div", { class: "chamber-list" }, CHAMBERS.map((c) => h("span", { class: "chip" }, glyph(CHAMBER_GLYPH[c], { size: "sm", plain: true }), ` ${c}`))),
      h("p", null, "Each Chamber holds Heaven and Hell Resonance stacks, board effects (ongoing, guard, trap) and conditions (❄️ Cold, 🥀 Thorn)."),
    ),
    h(
      "section",
      null,
      h("h3", null, "Intents"),
      h(
        "table",
        { class: "rtable" },
        h("tr", null, h("th", null, "Press"), h("td", null, "Score now. Bonus = ⌊(your stack + same-aspect harmony − ⌊enemy stack ÷ 2⌋) ÷ 3⌋. Adds the glyph's stack value.")),
        h("tr", null, h("th", null, "Anchor"), h("td", null, "No Soul movement now, −1 power. Adds stack value + intensity (+1 with harmony). Build for later.")),
        h("tr", null, h("th", null, "Release"), h("td", null, "Spend friendly stack (chosen before the roll) for a burst: min(5, ⌈spent × intensity ÷ 4⌉) to movement and power.")),
        h("tr", null, h("th", null, "Sever"), h("td", null, "−1 effect and power (+1 power if enemy stack is present). Cuts enemy stack by stack value + intensity (+1 if disruptive).")),
      ),
    ),
    h("section", null, h("h3", null, "Price keywords"), h("table", { class: "rtable" }, KEYWORDS.map(([k, t]) => h("tr", null, h("th", null, k), h("td", null, t))))),
    h(
      "section",
      null,
      h("h3", null, "Purgatory Events (at 3 Strain)"),
      h("table", { class: "rtable" }, EVENTS.map(([g, n, t]) => h("tr", null, h("th", null, glyph(g, { size: "sm" }), ` ${n}`), h("td", null, t)))),
    ),
    h(
      "section",
      null,
      h("h3", null, "Deck rules"),
      h("p", null, "Exactly 10 glyphs. Rarity budget 24 or less (Common 1, Uncommon 2, Rare 3). At most 4 Rares. Duplicates allowed."),
    ),
  );
}

function gallery(redraw: () => void): HTMLElement {
  return h(
    "div",
    { class: "gallery" },
    h(
      "div",
      { class: "seg" },
      (["heaven", "hell"] as Faction[]).map((f) =>
        btn([glyph(FACTION_GLYPH[f], { size: "sm", faction: f }), f === "heaven" ? " Heaven" : " Hell"], () => {
          galleryFaction = f;
          redraw();
        }, galleryFaction === f ? "on" : ""),
      ),
    ),
    h(
      "div",
      { class: "gallery-grid" },
      factionCards(galleryFaction).map((c) => h("div", { class: "gallery-item" }, cardEl(c.key, { size: "lib" }), h("p", null, c.help_text))),
    ),
  );
}

function credits(): HTMLElement {
  return h(
    "div",
    { class: "credits" },
    h("h3", null, "Soulscale · Purgatory"),
    h("p", null, "Game design, rules, music and sound by Wofl. Soulscale is run by whispr.dev."),
    legalLinks(),
    h("h3", null, "Third-party"),
    h(
      "ul",
      null,
      h("li", null, "Glyph art: Twemoji, © Twitter, Inc and other contributors, licensed CC-BY 4.0 (creativecommons.org/licenses/by/4.0)."),
      h("li", null, "Fonts: Cinzel and Cormorant Garamond, SIL Open Font License 1.1."),
      h("li", null, "Discord Embedded App SDK, MIT License."),
    ),
  );
}
