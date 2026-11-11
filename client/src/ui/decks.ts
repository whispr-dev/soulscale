// Deck Builder: click glyphs into a 10-card deck, live legality, save locally,
// import/export the one-emoji-per-line text format the Python editor used.

import { app } from "../app";
import { audio } from "../audio";
import { copyText, h, mount } from "../dom";
import { toast } from "../fx";
import type { Faction } from "../protocol";
import { BUDGET, card, DECK_SIZE, factionCards, legality, loadDecks, MAX_RARES, S, saveDecks, type SavedDeck } from "../store";
import { btn, cardEl, FACTION_GLYPH, FACTION_NAME, modal } from "./components";
import { glyph } from "../glyph";

const ed = { faction: "heaven" as Faction, cards: [] as string[], name: "My Deck", editing: -1 };

function normalize(s: string): string {
  return s.trim().replace(/️/g, "");
}

/** Python `parse_custom_deck`: 10 lines (or legacy 8, padded with commons). */
export function parseDeckText(f: Faction, text: string): { cards?: string[]; error?: string } {
  const lines = text
    .replace(/^﻿/, "")
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l && !l.startsWith("#"));
  if (lines.length !== DECK_SIZE && lines.length !== 8) return { error: `Need exactly ${DECK_SIZE} glyph lines (or a legacy 8); found ${lines.length}.` };
  const pool = factionCards(f);
  const keys: string[] = [];
  const bad: string[] = [];
  lines.forEach((l, i) => {
    const c = pool.find((x) => x.emoji === l || normalize(x.emoji) === normalize(l));
    if (c) keys.push(c.key);
    else bad.push(`line ${i + 1}: ${l}`);
  });
  if (bad.length) return { error: `Not ${FACTION_NAME[f]} glyphs: ${bad.join(", ")}` };
  const commons = pool.filter((c) => c.rarity === "Common").map((c) => c.key);
  while (keys.length < DECK_SIZE) keys.push(commons[(keys.length - 8) % commons.length]);
  const leg = legality(keys);
  if (leg.issues.length) return { error: `Recognised, but not legal: ${leg.issues.join("; ")}` };
  return { cards: keys };
}

export function renderDecks(screen: HTMLElement): void {
  const saved = loadDecks();
  const lib = factionCards(ed.faction);
  const leg = legality(ed.cards);
  const counts = new Map<string, number>();
  for (const k of ed.cards) counts.set(k, (counts.get(k) ?? 0) + 1);

  const nameInput = h("input", { class: "deck-name", value: ed.name, maxlength: 40, "aria-label": "Deck name" });
  nameInput.addEventListener("input", () => (ed.name = nameInput.value));

  const redraw = () => renderDecks(screen);
  const meter = (label: string, val: number, max: number, exact = false) =>
    h("div", { class: `meter ${exact ? (val === max ? "ok" : "bad") : val <= max ? "ok" : "bad"}` }, h("span", null, label), h("b", null, `${val}/${max}`), h("div", { class: "bar" }, h("i", { style: `width:${Math.min(100, (val / max) * 100)}%` })));

  mount(
    screen,
    h(
      "div",
      { class: "decks" },
      h(
        "header",
        { class: "page-head" },
        btn("← Lobby", () => app.go("lobby"), "ghost"),
        h("h2", null, "Deck Builder"),
        h(
          "div",
          { class: "seg" },
          (["heaven", "hell"] as Faction[]).map((f) =>
            btn([glyph(FACTION_GLYPH[f], { size: "sm", faction: f }), ` ${FACTION_NAME[f]}`], () => {
              if (ed.faction !== f) {
                ed.faction = f;
                ed.cards = [];
                ed.editing = -1;
              }
              redraw();
            }, ed.faction === f ? "on" : ""),
          ),
        ),
      ),
      h(
        "div",
        { class: "decks-body" },
        h(
          "section",
          { class: "library" },
          h("h3", null, "Glyphs: click to add"),
          h(
            "div",
            { class: "lib-grid" },
            lib.map((c) =>
              cardEl(c.key, {
                size: "lib",
                count: counts.get(c.key),
                disabled: ed.cards.length >= DECK_SIZE,
                onClick: () => {
                  if (ed.cards.length < DECK_SIZE) {
                    ed.cards.push(c.key);
                    redraw();
                  }
                },
              }),
            ),
          ),
        ),
        h(
          "section",
          { class: "deck-pane" },
          h("h3", null, "Your deck"),
          nameInput,
          meter("Glyphs", leg.size, DECK_SIZE, true),
          meter("Rarity budget", leg.cost, BUDGET),
          meter("Rares", leg.rares, MAX_RARES),
          h(
            "div",
            { class: "deck-slots" },
            Array.from({ length: DECK_SIZE }, (_, i) => {
              const k = ed.cards[i];
              if (!k) return h("div", { class: "slot empty" }, String(i + 1));
              const c = card(k);
              const s = h("button", { class: `slot rarity-${c.rarity.toLowerCase()}`, type: "button", title: `Remove ${c.name}` }, glyph(c.emoji, { size: "sm", faction: c.faction }), h("span", null, c.name));
              s.addEventListener("click", () => {
                audio.play("ui_back");
                ed.cards.splice(i, 1);
                redraw();
              });
              return s;
            }),
          ),
          leg.issues.length ? h("div", { class: "issues" }, leg.issues.join(" · ")) : h("div", { class: "issues ok" }, "Legal deck ✓"),
          h(
            "div",
            { class: "row wrap" },
            btn("Save", () => {
              if (leg.issues.length) return toast("Fix the deck first: " + leg.issues.join("; "), "error");
              const list = loadDecks();
              const d: SavedDeck = { name: ed.name.trim() || "My Deck", faction: ed.faction, cards: [...ed.cards] };
              if (ed.editing >= 0 && list[ed.editing]?.faction === ed.faction) list[ed.editing] = d;
              else {
                list.push(d);
                ed.editing = list.length - 1;
              }
              saveDecks(list);
              toast(`Saved “${d.name}”. It's now in the setup deck lists.`, "notice");
              redraw();
            }, "primary"),
            btn("Clear", () => {
              ed.cards = [];
              ed.editing = -1;
              redraw();
            }, "ghost"),
            btn("Export text", async () => {
              const text = `# ${ed.name} (${FACTION_NAME[ed.faction]})\n${ed.cards.map((k) => card(k).emoji).join("\n")}\n`;
              toast((await copyText(text)) ? "Deck copied (one glyph per line)." : "Copy failed; your browser blocked the clipboard.", "notice");
            }, "ghost"),
            btn("Import text", () => importDialog(redraw), "ghost"),
          ),
          h("h3", null, "Start from…"),
          h(
            "div",
            { class: "starters" },
            (S.catalog?.decks ?? [])
              .filter((d) => d.faction === ed.faction)
              .map((d) =>
                btn(d.name, () => {
                  ed.cards = [...d.cards];
                  ed.name = `${d.name} (mine)`;
                  ed.editing = -1;
                  redraw();
                }, "ghost small"),
              ),
          ),
          saved.length
            ? h(
                "div",
                { class: "saved" },
                h("h3", null, "Saved decks"),
                saved.map((d, i) =>
                  h(
                    "div",
                    { class: "saved-row" },
                    glyph(FACTION_GLYPH[d.faction], { size: "sm", faction: d.faction }),
                    h("span", null, d.name),
                    btn("Edit", () => {
                      ed.faction = d.faction;
                      ed.cards = [...d.cards];
                      ed.name = d.name;
                      ed.editing = i;
                      redraw();
                    }, "ghost small"),
                    btn("Delete", () => {
                      const list = loadDecks();
                      list.splice(i, 1);
                      saveDecks(list);
                      if (ed.editing === i) ed.editing = -1;
                      redraw();
                    }, "ghost small danger"),
                  ),
                ),
              )
            : null,
        ),
      ),
    ),
  );
}

function importDialog(redraw: () => void): void {
  const ta = h("textarea", { rows: 12, placeholder: "🕯️\n🤲\n🛏️\n…one glyph per line (lines starting with # are ignored)" });
  const m = modal(
    `Import a ${FACTION_NAME[ed.faction]} deck`,
    h(
      "div",
      { class: "import" },
      ta,
      h(
        "div",
        { class: "row end" },
        btn("Import", () => {
          const r = parseDeckText(ed.faction, ta.value);
          if (r.error) return toast(r.error, "error", 6000);
          ed.cards = r.cards!;
          ed.editing = -1;
          m.close();
          redraw();
        }, "primary"),
      ),
    ),
  );
}
