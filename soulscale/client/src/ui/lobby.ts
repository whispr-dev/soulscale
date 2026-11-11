// Lobby / title screen: who's here, what's on the table, and the new-game setup.

import { app } from "../app";
import { audio } from "../audio";
import { h, mount } from "../dom";
import { glyph } from "../glyph";
import type { DeckChoice, Faction, GameMode, RulesVersion, TableInfo } from "../protocol";
import { card, legality, loadDecks, S } from "../store";
import { avatar, btn, chip, FACTION_GLYPH, FACTION_NAME, modal } from "./components";
import { openSettings } from "./settings";
import { legalLinks } from "./legal";

type Setup = {
  side: Faction;
  deck: string; // "builtin:<name>" | "custom:<index>"
  oppDeck: string; // "" (random) | "builtin:<name>" | "custom:<index>"
  goal: number;
  profile: string;
  rules: RulesVersion;
  secret: boolean;
};

const last: Setup = {
  side: "heaven",
  deck: "builtin:Starter Mercy",
  oppDeck: "",
  goal: 18,
  profile: "Balanced / Tactical",
  rules: "1.0.2",
  secret: false,
};

function decksFor(f: Faction): { id: string; label: string; cards: string[] }[] {
  const out: { id: string; label: string; cards: string[] }[] = [];
  for (const d of S.catalog?.decks ?? []) if (d.faction === f) out.push({ id: `builtin:${d.name}`, label: d.name, cards: d.cards });
  loadDecks().forEach((d, i) => {
    if (d.faction === f && legality(d.cards).issues.length === 0) out.push({ id: `custom:${i}`, label: `★ ${d.name}`, cards: d.cards });
  });
  return out;
}

function deckChoice(id: string): DeckChoice {
  if (!id) return { kind: "random" };
  if (id.startsWith("builtin:")) return { kind: "builtin", name: id.slice(8) };
  const d = loadDecks()[parseInt(id.slice(7), 10)];
  return d ? { kind: "custom", name: d.name, cards: d.cards } : { kind: "random" };
}

function deckStrip(cards: string[]): HTMLElement {
  return h("div", { class: "deck-strip" }, cards.map((k) => glyph(card(k).emoji, { size: "sm", faction: card(k).faction, title: card(k).name })));
}

const MODE_TITLE: Record<GameMode, string> = {
  vs_computer: "Play vs Computer",
  duel: "Open a Duel",
  hotseat: "Pass & Play",
  secret_hotseat: "Pass & Play",
};

export function openSetup(mode: GameMode, joinDuel?: { side: Faction }): void {
  audio.unlock();
  const s = { ...last };
  if (joinDuel) s.side = joinDuel.side;
  const body = h("div", { class: "setup" });
  const m = modal(joinDuel ? "Accept the Duel" : MODE_TITLE[mode], body, { wide: true, cls: "setup-modal" });

  const draw = () => {
    const mine = decksFor(s.side);
    if (!mine.some((d) => d.id === s.deck)) s.deck = mine[0]?.id ?? "";
    const opp = decksFor(s.side === "heaven" ? "hell" : "heaven");
    if (s.oppDeck && !opp.some((d) => d.id === s.oppDeck)) s.oppDeck = "";
    const deckSel = select(mine.map((d) => [d.id, d.label]), s.deck, (v) => {
      s.deck = v;
      draw();
    });
    const chosen = mine.find((d) => d.id === s.deck);
    const sections: HTMLElement[] = [];
    if (!joinDuel) {
      sections.push(
        field(
          "Your side",
          h(
            "div",
            { class: "side-pick" },
            (["heaven", "hell"] as Faction[]).map((f) =>
              h(
                "button",
                {
                  class: `side ${f} ${s.side === f ? "on" : ""}`,
                  type: "button",
                  on: {
                    click: () => {
                      audio.play("ui_select");
                      s.side = f;
                      draw();
                    },
                  },
                },
                glyph(FACTION_GLYPH[f], { size: "xl", faction: f }),
                h("b", null, FACTION_NAME[f]),
                h("small", null, f === "heaven" ? "Comfort, mercy, memory. Push the Soul up." : "Pain, dread, despair. Drag the Soul down."),
              ),
            ),
          ),
        ),
      );
    } else {
      sections.push(h("p", { class: "lead" }, `You'll play ${FACTION_NAME[s.side]}. Pick your deck:`));
    }
    sections.push(field("Your deck", h("div", null, deckSel, chosen ? deckStrip(chosen.cards) : null)));
    if (mode === "vs_computer" || mode === "hotseat" || mode === "secret_hotseat") {
      const oppSel = select([["", mode === "vs_computer" ? "Personality's favourite / random" : "Random"], ...opp.map((d) => [d.id, d.label] as [string, string])], s.oppDeck, (v) => {
        s.oppDeck = v;
        draw();
      });
      const oc = opp.find((d) => d.id === s.oppDeck);
      sections.push(field(mode === "vs_computer" ? "Computer's deck" : "Player 2's deck", h("div", null, oppSel, oc ? deckStrip(oc.cards) : null)));
    }
    if (mode === "vs_computer") {
      sections.push(
        field(
          "Opponent personality",
          h(
            "div",
            { class: "profiles" },
            (S.catalog?.ai_profiles ?? []).map((p) =>
              h(
                "button",
                {
                  class: `profile ${s.profile === p.label ? "on" : ""}`,
                  type: "button",
                  on: {
                    click: () => {
                      audio.play("ui_select");
                      s.profile = p.label;
                      draw();
                    },
                  },
                },
                h("b", null, p.label),
                h("small", null, p.description),
              ),
            ),
          ),
        ),
      );
    }
    if (!joinDuel) {
      sections.push(
        field(
          "Game length",
          h(
            "div",
            { class: "seg" },
            ([
              [12, "Quick ±12"],
              [18, "Standard ±18"],
              [24, "Deep ±24"],
            ] as [number, string][]).map(([g, l]) =>
              btn(l, () => {
                s.goal = g;
                draw();
              }, s.goal === g ? "on" : ""),
            ),
          ),
        ),
      );
      if (mode === "hotseat" || mode === "secret_hotseat") {
        const cb = h("input", { type: "checkbox", checked: s.secret });
        cb.addEventListener("change", () => (s.secret = cb.checked));
        sections.push(field("Privacy", h("label", { class: "setting check" }, cb, h("span", null, "Secret: hide each player's glyphs and choices from the other"))));
      }
      sections.push(
        field(
          "Rules",
          h(
            "div",
            { class: "seg" },
            btn("v1.0.2 (fixed)", () => {
              s.rules = "1.0.2";
              draw();
            }, s.rules === "1.0.2" ? "on" : "", "v1.0.1 plus fixes: Screech only below 0, AI thresholds scale with game length, Void Thought counts as a defender, Silver Thread log text"),
            btn("v1.0.1 (original)", () => {
              s.rules = "1.0.1";
              draw();
            }, s.rules === "1.0.1" ? "on" : "", "Exactly the original Python engine, quirks included"),
          ),
        ),
      );
    }
    const go = btn(joinDuel ? "Take the seat" : mode === "duel" ? "Open the Duel" : "Begin", () => start(), "primary big");
    mount(body, sections, h("div", { class: "row end" }, btn("Cancel", () => m.close(), "ghost"), go));
  };

  const start = () => {
    Object.assign(last, s);
    if (joinDuel) {
      app.send({ t: "join_duel", deck: deckChoice(s.deck) });
    } else {
      const realMode: GameMode = mode === "hotseat" || mode === "secret_hotseat" ? (s.secret ? "secret_hotseat" : "hotseat") : mode;
      app.send({
        t: "new_game",
        game: {
          mode: realMode,
          side: s.side,
          deck: deckChoice(s.deck),
          opponent_deck: realMode === "duel" ? null : deckChoice(s.oppDeck),
          goal: s.goal,
          ai_profile: realMode === "vs_computer" ? s.profile : null,
          rules: s.rules,
        },
      });
    }
    audio.play("ui_confirm");
    m.close();
  };
  draw();
}

function field(label: string, control: HTMLElement): HTMLElement {
  return h("div", { class: "field" }, h("div", { class: "field-label" }, label), control);
}

function select(options: [string, string][], value: string, onChange: (v: string) => void): HTMLSelectElement {
  const el = h("select", null, options.map(([v, l]) => h("option", { value: v, selected: v === value }, l)));
  el.addEventListener("change", () => {
    audio.play("ui_click");
    onChange(el.value);
  });
  return el;
}

// ---------------------------------------------------------------- lobby screen

function sigil(): HTMLElement {
  const glyphs = (S.catalog?.cards ?? []).map((c) => c.emoji);
  const ring = h("div", { class: "sigil-ring" });
  glyphs.forEach((g, i) => {
    const a = (i / glyphs.length) * 360;
    const el = h("span", { class: "sigil-glyph", style: `transform: rotate(${a}deg) translateY(calc(var(--sigil-r) * -1)) rotate(${-a}deg)` }, glyph(g, { size: "sm", plain: true }));
    ring.appendChild(el);
  });
  return h("div", { class: "sigil" }, ring, h("div", { class: "sigil-core" }, glyph("⚖️", { size: "xl" })));
}

function tableCard(t: TableInfo | undefined): HTMLElement {
  const me = S.me;
  const wrap = h("section", { class: "table-card" });
  if (!t || t.state === "empty") {
    mount(
      wrap,
      h("h3", null, "The table is empty"),
      h("p", { class: "lead" }, "Start something:"),
      h(
        "div",
        { class: "mode-grid" },
        modeButton("🤖", "Play vs Computer", "Five personalities, from Novice to Cruel Accountant.", () => openSetup("vs_computer")),
        modeButton("⚔️", "Open a Duel", S.inDiscord ? "Challenge someone in this channel. Simultaneous secret picks." : "Challenge another tab or player in this room.", () => openSetup("duel")),
        modeButton("🔁", "Pass & Play", "Two people, one screen. Optional secret mode.", () => openSetup("hotseat")),
      ),
    );
    return wrap;
  }
  if (t.state === "open") {
    const mine = me && t.host.id === me.id;
    const opp: Faction = t.side === "heaven" ? "hell" : "heaven";
    mount(
      wrap,
      h("h3", null, mine ? "Waiting for a challenger…" : `${t.host.name} calls for a Duel`),
      h(
        "div",
        { class: "duel-card" },
        avatar(t.host, t.host.name, t.side),
        h("div", null, h("b", null, `${t.host.name} as ${FACTION_NAME[t.side]}`), h("div", { class: "muted" }, `${t.deck} · goal ±${t.goal} · rules v${t.rules}`)),
      ),
      mine
        ? h("div", { class: "row" }, S.inDiscord ? btn("Invite friends", () => app.invite(), "primary") : h("p", { class: "muted" }, "Open this page in another tab with ?name=Someone to accept."), btn("Cancel", () => app.send({ t: "cancel_open" }), "ghost"))
        : h("div", { class: "row" }, btn(`Accept as ${FACTION_NAME[opp]}`, () => openSetup("duel", { side: opp }), "primary big")),
    );
    return wrap;
  }
  const seated = me && t.seats.some((u) => u?.id === me.id);
  const names = t.seats.map((u, i) => u?.name ?? (t.mode === "vs_computer" && i === 1 ? "Computer" : "—"));
  mount(
    wrap,
    h("h3", null, t.over ? "Match finished" : seated ? "Your match is on" : "A match is in progress"),
    h("p", { class: "lead" }, `${names[0]} vs ${names[1]} · Round ${t.round} · rules v${t.rules}`),
    h(
      "div",
      { class: "row" },
      btn(seated ? "Return to the table" : t.over ? "View the board" : "Watch", () => {
        S.spectating = !seated;
        app.go("game");
      }, "primary big"),
      t.over ? btn("New game", () => openSetup("vs_computer"), "ghost") : null,
    ),
  );
  return wrap;
}

function modeButton(icon: string, title: string, text: string, onClick: () => void): HTMLElement {
  const b = h("button", { class: "mode", type: "button" }, h("span", { class: "mode-icon" }, glyph(icon, { size: "lg" })), h("b", null, title), h("small", null, text));
  b.addEventListener("click", () => {
    audio.unlock();
    audio.play("ui_click");
    onClick();
  });
  b.addEventListener("pointerenter", () => audio.play("ui_hover"));
  return b;
}

export function renderLobby(screen: HTMLElement): void {
  const room = S.room;
  const people = h(
    "section",
    { class: "people" },
    h("h3", null, "In this room"),
    (room?.participants ?? []).map((p) =>
      h(
        "div",
        { class: `person ${p.connected ? "" : "away"}` },
        avatar(p.user, p.user.name),
        h("span", null, p.user.name),
        p.role !== "spectator" ? chip(p.role === "both" ? "pass & play" : p.role === "host" ? "duel host" : p.role, "muted") : null,
        p.user.id === S.me?.id ? chip("you", "you") : null,
      ),
    ),
  );
  mount(
    screen,
    h(
      "div",
      { class: "lobby" },
      h(
        "div",
        { class: "title-block" },
        sigil(),
        h("h1", { class: "title" }, "SOULSCALE"),
        h("div", { class: "subtitle" }, "· Purgatory ·"),
        h("p", { class: "tagline" }, "A single Soul hangs between Heaven and Hell. Play glyphs into its Chambers and pull it home."),
      ),
      h("div", { class: "lobby-main" }, tableCard(room?.table), people),
      h(
        "nav",
        { class: "lobby-nav" },
        btn("📖 How to Play", () => app.go("help"), "ghost"),
        btn("🃏 Deck Builder", () => app.go("decks"), "ghost"),
        btn("🎚 Sound Lab", () => app.go("soundlab"), "ghost"),
        btn("⚙ Settings", () => openSettings(), "ghost"),
      ),
      legalLinks(),
      S.dev ? h("div", { class: "dev-banner" }, `Browser test mode as “${S.me?.name}”. Room: ${new URLSearchParams(location.search).get("room") ?? "local"}. Add ?name=Bob in another tab to duel yourself.`) : null,
    ),
  );
}
