// The game screen: Soul pillar, five Chambers, both seats, hands, intent picker, dice,
// chronicle log, end-of-match summary. Rendering is a pure function of the server View;
// animations (anim.ts) play the events that led to it first.

import { app } from "../app";
import { audio } from "../audio";
import { h, mount, sleep } from "../dom";
import { fx, toast } from "../fx";
import { glyph } from "../glyph";
import type { Chamber, ChamberView, Faction, GameEvent, Intent, Seat, SeatView, View } from "../protocol";
import { CHAMBERS, CHAMBER_GLYPH, seatIdx } from "../protocol";
import { card, changed, isDisruptive, mySeats, prefs, pressBonus, releaseBonus, S, savePrefs } from "../store";
import { animateEvents, type AnimCtx } from "./anim";
import { avatar, btn, cardBack, cardEl, chip, confirmBox, FACTION_GLYPH, FACTION_NAME, modal } from "./components";
import { openSettings } from "./settings";

type Sel = { seat: Seat; index: number; key: string };

const G = {
  sel: null as Sel | null,
  pending: false,
  animating: false,
  queue: [] as { view: View; events: GameEvent[] }[],
  shown: null as View | null,
  log: [] as string[],
  dealt: new Set<Seat>(),
  summaryShown: false,
  unveiled: null as Seat | null,
  diceRolling: false,
  diceShownRaw: null as number | null,
};

let root: HTMLElement | null = null;

// ---------------------------------------------------------------- lifecycle

export function resetGameUi(): void {
  G.sel = null;
  G.pending = false;
  G.queue = [];
  G.shown = null;
  G.log = [];
  G.dealt.clear();
  G.summaryShown = false;
  G.unveiled = null;
  G.diceRolling = false;
  G.diceShownRaw = null;
}

export function mountGame(screen: HTMLElement): void {
  root = h(
    "div",
    { class: "game" },
    h("header", { class: "topbar", id: "g-top" }),
    h("section", { class: "seat-area top", id: "g-seat-top" }),
    h(
      "div",
      { class: `table ${prefs.showLog ? "" : "nolog"}`, id: "g-table" },
      h("aside", { class: "pillar", id: "g-pillar" }),
      h("div", { class: "chambers", id: "g-chambers" }),
      h(
        "aside",
        { class: "logpanel", id: "g-log" },
        h("div", { class: "log-head" }, "Chronicle"),
        h("div", { class: "log-lines", id: "g-loglines" }),
      ),
    ),
    h("section", { class: "seat-area bottom", id: "g-seat-bottom" }),
  );
  mount(screen, root);
  for (const line of G.log.slice(-300)) appendLogDom(line);
  if (S.view) render(S.view);
  audio.setScene(S.view?.over ? "endgame" : "match");
}

export function unmountGame(): void {
  root = null;
}

export function gameMounted(): boolean {
  return root !== null && root.isConnected;
}

/** A Game message from the server. */
export function onGame(view: View, events: GameEvent[]): void {
  if (events.some((e) => e.t === "round_start" && e.round === 1)) {
    resetGameUi();
    document.getElementById("g-loglines")?.replaceChildren();
  }
  if (!gameMounted()) {
    // Not watching the board: keep the log and state current without animating.
    for (const e of events) if (e.t === "log") G.log.push(e.text);
    S.view = view;
    G.shown = view;
    changed();
    return;
  }
  G.queue.push({ view, events });
  void pump();
}

async function pump(): Promise<void> {
  if (G.animating) return;
  while (G.queue.length) {
    const { view, events } = G.queue.shift()!;
    G.animating = true;
    if (events.length) clearSelection(false);
    try {
      await animateEvents(events, animCtx(view));
    } catch (e) {
      console.error("animation failed", e);
    }
    G.shown = view;
    S.view = view;
    G.pending = false;
    G.animating = false;
    if (gameMounted()) {
      render(view);
      afterRender(view);
    }
    changed();
  }
}

// ---------------------------------------------------------------- helpers

const sv = (v: View, s: Seat): SeatView => v.seats[seatIdx(s)];
const other = (s: Seat): Seat => (s === "P1" ? "P2" : "P1");

function bottomSeat(v: View): Seat {
  return v.viewer.kind === "seat" ? v.viewer.seat : "P1";
}

function actingSeat(v: View): Seat | null {
  for (const s of mySeats(v)) {
    const st = sv(v, s).status;
    if (st && (st.kind === "choose" || st.kind === "reroll")) return s;
  }
  return null;
}

function seatUser(s: Seat) {
  const t = S.room?.table;
  return t && t.state === "live" ? t.seats[seatIdx(s)] : null;
}

function displayName(v: View, s: Seat): string {
  const seat = sv(v, s);
  if (v.mode === "vs_computer") return s === "P1" ? (seatUser("P1")?.name ?? "You") : "Computer";
  return seat.name;
}

function handHidden(v: View, s: Seat): boolean {
  if (v.mode !== "secret_hotseat" || v.viewer.kind !== "both") return false;
  const acting = actingSeat(v);
  return acting !== s || G.unveiled !== s;
}

// ---------------------------------------------------------------- render

function render(v: View): void {
  if (!root) return;
  renderTop(v);
  const bottom = bottomSeat(v);
  renderSeat(v, other(bottom), "top");
  renderSeat(v, bottom, "bottom");
  renderPillar(v);
  renderChambers(v);
  document.getElementById("g-table")?.classList.toggle("nolog", !prefs.showLog || logHiddenForSecret(v));
}

function logHiddenForSecret(v: View): boolean {
  return v.mode === "secret_hotseat" && v.viewer.kind === "both" && actingSeat(v) === "P2";
}

function modeLabel(v: View): string {
  switch (v.mode) {
    case "vs_computer":
      return `vs Computer · ${v.ai_profile}`;
    case "duel":
      return "Duel";
    case "hotseat":
      return "Pass & Play";
    case "secret_hotseat":
      return "Pass & Play · Secret";
  }
}

function pendingChips(v: View): HTMLElement[] {
  const out: HTMLElement[] = [];
  for (const d of v.delayed_next) out.push(chip(`⏳ ${d.label} ${d.amount > 0 ? "+" : ""}${d.amount}`, d.amount >= 0 ? "heaven" : "hell", "Lands at the start of next round"));
  if (v.heaven_next_bonus) out.push(chip(`🌤️ next Heaven push +${v.heaven_next_bonus}`, "heaven", "Reprieve"));
  if (v.hell_next_bonus) out.push(chip(`🕳️ next Hell push +${v.hell_next_bonus}`, "hell", "Collapse"));
  if (v.all_next_bonus) out.push(chip(`🔥 next effect +${v.all_next_bonus}`, "violet", "Intensification"));
  if (v.soul_cap !== null) out.push(chip(`🕰️ Soul capped ±${v.soul_cap}`, "violet", "Stasis"));
  if (v.heaven_cap !== null) out.push(chip(`⏳ Heaven capped at +${v.heaven_cap}`, "hell", "Endless Wait"));
  if (v.support_bonus[0]) out.push(chip(`🧸 next Heaven effect +${v.support_bonus[0]}`, "heaven", "Support"));
  return out;
}

function renderTop(v: View): void {
  const el = document.getElementById("g-top");
  if (!el) return;
  const strain = h("div", { class: "strain", id: "g-strain", title: "Strain: at 3, a Purgatory Event fires" }, strainPips(v.strain));
  const iAmPlayer = mySeats(v).length > 0;
  mount(
    el,
    h("div", { class: "top-left" }, h("span", { class: "brand" }, "SOULSCALE"), chip(modeLabel(v), "mode"), chip(`rules v${v.rules}`, "muted", "Rules version")),
    h("div", { class: "top-mid" }, h("span", { class: "round" }, `Round ${v.round}`), strain, pendingChips(v)),
    h(
      "div",
      { class: "top-right" },
      btn(prefs.speed === 1 ? "▶ 1×" : prefs.speed === 2 ? "⏩ 2×" : "⏭ 4×", () => {
        prefs.speed = prefs.speed === 1 ? 2 : prefs.speed === 2 ? 4 : 1;
        savePrefs();
        renderTop(v);
      }, "ghost small", "Animation speed"),
      btn("📜", () => {
        prefs.showLog = !prefs.showLog;
        savePrefs();
        render(v);
      }, "ghost small icon", "Show/hide the Chronicle"),
      btn(prefs.muted ? "🔇" : "🔊", () => {
        prefs.muted = !prefs.muted;
        savePrefs();
        audio.applyVolumes();
        renderTop(v);
      }, "ghost small icon", "Mute"),
      btn("⚙", () => openSettings(), "ghost small icon", "Settings"),
      btn("⬇", () => app.send({ t: "export" }), "ghost small icon", "Export the playtest log"),
      iAmPlayer && !v.over
        ? btn("⚑", () => confirmBox("Resign this match? It ends for everyone at the table.", "Resign", () => app.send({ t: "resign" })), "ghost small icon", "Resign")
        : null,
      btn("⌂", () => {
        S.spectating = false;
        app.go("lobby");
      }, "ghost small icon", "Back to the lobby"),
    ),
  );
}

function strainPips(n: number): HTMLElement[] {
  return [h("span", { class: "strain-label" }, "Strain"), ...[0, 1, 2].map((i) => h("span", { class: `pip ${i < n ? "on" : ""}` }))];
}

function willGems(s: SeatView): HTMLElement {
  const gems = Array.from({ length: Math.max(s.will, 0) }, () => h("span", { class: "gem on" }, "◆"));
  return h(
    "span",
    { class: "will", title: "Will: spend 1 to reroll; Heavy glyphs spend it automatically" },
    "Will ",
    gems.length ? gems : h("span", { class: "gem" }, "◇"),
    s.next_will ? h("span", { class: `next ${s.next_will > 0 ? "up" : "down"}` }, ` ${s.next_will > 0 ? "+" : ""}${s.next_will} next`) : null,
  );
}

function statusChip(v: View, s: SeatView): HTMLElement {
  const map: Record<string, [string, string]> = {
    choose: ["choosing…", "busy"],
    reroll: ["rolling…", "busy"],
    locked: ["locked in ✓", "ok"],
    waiting: ["waiting", "muted"],
    done: [v.winner === s.faction ? "victorious" : "defeated", v.winner === s.faction ? "ok" : "muted"],
  };
  const [t, c] = map[s.public_status] ?? [s.public_status, "muted"];
  return chip(s.controller === "ai" && v.mode === "duel" ? `AI holding seat · ${t}` : t, c);
}

function renderSeat(v: View, seat: Seat, pos: "top" | "bottom"): void {
  const el = document.getElementById(pos === "top" ? "g-seat-top" : "g-seat-bottom");
  if (!el) return;
  const s = sv(v, seat);
  const mine = mySeats(v).includes(seat);
  const acting = actingSeat(v) === seat;
  const panel = h(
    "div",
    { class: `seat-panel faction-${s.faction} ${acting ? "active" : ""}`, data: { seat } },
    avatar(seatUser(seat), displayName(v, seat), s.faction),
    h(
      "div",
      { class: "seat-meta" },
      h("div", { class: "seat-name" }, glyph(FACTION_GLYPH[s.faction], { size: "sm", faction: s.faction }), displayName(v, seat), mine && v.viewer.kind === "seat" ? chip("you", "you") : null),
      h("div", { class: "seat-sub" }, `${FACTION_NAME[s.faction]} · ${s.deck_name}`),
      h("div", { class: "seat-stats" }, willGems(s), statusChip(v, s), h("span", { class: "deckcount", title: "Draw pile · discard" }, `🂠 ${s.draw_count} · ⟲ ${s.discard_count}`)),
    ),
  );
  const hand = renderHand(v, seat, pos);
  const parts: HTMLElement[] = [panel, hand];
  if (pos === "bottom") parts.push(renderAction(v));
  mount(el, parts);
}

function renderHand(v: View, seat: Seat, pos: "top" | "bottom"): HTMLElement {
  const s = sv(v, seat);
  const size = pos === "bottom" ? "hand" : "small";
  const wrap = h("div", { class: `hand hand-${pos}`, data: { seat } });
  const hidden = handHidden(v, seat);
  if (!s.hand || hidden) {
    for (let i = 0; i < s.hand_count; i++) wrap.appendChild(cardBack(s.faction, size === "hand" ? "small" : "small"));
    return wrap;
  }
  const mine = mySeats(v).includes(seat);
  const canChoose = mine && s.status?.kind === "choose" && !G.pending && !G.animating;
  const committed = s.choice?.card;
  let committedMarked = false;
  s.hand.forEach((key, index) => {
    const forcedOnly = s.forced && s.forced.card !== key;
    const isCommitted = !!committed && !committedMarked && key === committed && s.status?.kind !== "choose";
    if (isCommitted) committedMarked = true;
    const el = cardEl(key, {
      size,
      selected: (G.sel?.seat === seat && G.sel.index === index) || isCommitted,
      disabled: !canChoose || !!forcedOnly,
      dim: !!committed && !isCommitted && s.status?.kind !== "choose",
      onClick: canChoose && !forcedOnly ? () => selectCard({ seat, index, key }) : undefined,
    });
    el.dataset.index = String(index);
    if (isCommitted) el.appendChild(h("div", { class: "lock-badge" }, "🔒"));
    if (G.dealt.has(seat)) {
      el.classList.add("dealing");
      el.style.animationDelay = `${index * 90}ms`;
    }
    wrap.appendChild(el);
  });
  return wrap;
}

function renderAction(v: View): HTMLElement {
  const wrap = h("div", { class: "action", id: "g-action" });
  const mine = mySeats(v);
  if (v.over) {
    wrap.append(
      h("div", { class: "prompt" }, `${v.winner === "heaven" ? "Heaven" : "Hell"} wins.`),
      btn("Summary", () => showSummary(v, true), "primary"),
    );
    return wrap;
  }
  if (!mine.length) {
    wrap.append(h("div", { class: "prompt" }, `Watching ${displayName(v, "P1")} vs ${displayName(v, "P2")}`));
    return wrap;
  }
  const acting = actingSeat(v);
  if (acting) {
    const s = sv(v, acting);
    if (v.mode === "secret_hotseat" && G.unveiled !== acting) {
      wrap.append(curtain(acting, v));
      return wrap;
    }
    const who = mine.length > 1 ? `${displayName(v, acting)}: ` : "";
    // Pending first: until the server answers our last action, the shown view still
    // says "reroll" or "choose", and its buttons must not be live a second time.
    if (G.pending) {
      if (s.status?.kind === "reroll" && !G.diceRolling) {
        // Kept the roll: show it settled while the round resolves.
        wrap.append(dieFace(s.status.raw, "small"), h("div", { class: "prompt waiting" }, "Kept. Resolving…"));
      } else {
        wrap.append(dicePanel(v, acting, null, s.will));
      }
      return wrap;
    }
    if (s.status?.kind === "reroll") {
      wrap.append(dicePanel(v, acting, s.status.raw, s.will));
      return wrap;
    }
    if (s.forced) {
      const f = s.forced;
      wrap.append(
        h("div", { class: "prompt warn" }, `${who}no glyph has an open Chamber. The only play is ${card(f.card).emoji} ${card(f.card).name} → ${f.chamber} (Press).`),
        btn("Play it", () => submitChoice(acting, f.card, f.chamber, { kind: "press" }), "primary"),
      );
      return wrap;
    }
    if (G.sel) {
      const c = card(G.sel.key);
      wrap.append(
        h("div", { class: "prompt" }, `${who}${c.emoji} ${c.name}: pick a glowing Chamber.`, h("div", { class: "help" }, c.help_text)),
        btn("Cancel", () => clearSelection(true), "ghost"),
      );
    } else {
      wrap.append(h("div", { class: "prompt" }, `${who}choose a glyph to play.`));
    }
    return wrap;
  }
  // Locked in, waiting.
  const me = mine[0];
  const s = sv(v, me);
  if (s.roll !== null && s.status?.kind === "locked") wrap.append(dieFace(s.roll, "small"));
  const opp = other(me);
  const waitingFor = mine.length > 1 ? "resolution" : displayName(v, opp);
  wrap.append(h("div", { class: "prompt waiting" }, mine.length > 1 ? "Resolving…" : `Locked in. Waiting for ${waitingFor}…`));
  if (v.mode === "duel" && app.discord) wrap.append(btn("Invite", () => app.invite(), "ghost small"));
  return wrap;
}

function curtain(seat: Seat, v: View): HTMLElement {
  return h(
    "div",
    { class: "curtain" },
    h("div", { class: "prompt" }, `Pass to ${displayName(v, seat)} (${FACTION_NAME[sv(v, seat).faction]}). Everyone else, look away.`),
    btn(`I'm ${displayName(v, seat)}: show my glyphs`, () => {
      G.unveiled = seat;
      render(v);
    }, "primary"),
  );
}

// ---------------------------------------------------------------- dice

const INTENSITY = ["", "Weak ×1", "Steady ×2", "Surge ×3"];
const intensityOf = (raw: number) => (raw <= 2 ? 1 : raw <= 4 ? 2 : 3);

function dieFace(raw: number | null, cls = ""): HTMLElement {
  const pips: Record<number, number[]> = { 1: [4], 2: [0, 8], 3: [0, 4, 8], 4: [0, 2, 6, 8], 5: [0, 2, 4, 6, 8], 6: [0, 2, 3, 5, 6, 8] };
  const on = raw ? pips[raw] : [];
  return h("div", { class: `die ${cls} ${raw ? "" : "rolling"}`, id: cls ? undefined : "g-die" }, Array.from({ length: 9 }, (_, i) => h("span", { class: on.includes(i) ? "pip on" : "pip" })));
}

function dicePanel(v: View, seat: Seat, raw: number | null, will: number): HTMLElement {
  const wrap = h("div", { class: "dice-panel" });
  const face = dieFace(raw);
  wrap.appendChild(face);
  if (raw === null) {
    wrap.appendChild(h("div", { class: "prompt" }, "Rolling…"));
    return wrap;
  }
  if (G.diceShownRaw !== raw) {
    G.diceShownRaw = raw;
    face.animate([{ transform: "rotate(-200deg) scale(0.6)" }, { transform: "rotate(10deg) scale(1.1)", offset: 0.8 }, { transform: "rotate(0) scale(1)" }], { duration: 420, easing: "ease-out" });
    audio.play("dice_roll");
  }
  const i = intensityOf(raw);
  const reroll = btn(`Reroll · 1 Will (${will})`, () => {
    // A reroll is final (the new roll stands), so nothing is left to decide: freeze
    // the panel on a tumbling die until the server's answer animates the new face.
    G.diceRolling = true;
    G.pending = true;
    audio.play("dice_reroll");
    app.send({ t: "reroll", seat, reroll: true });
    render(v);
  }, "reroll", will > 0 ? "Spend 1 Will to roll again; the new roll stands" : "No Will left");
  reroll.disabled = will <= 0;
  wrap.append(
    h("div", { class: "dice-info" }, h("div", { class: `intensity i${i}` }, INTENSITY[i]), h("div", { class: "help" }, "1–2 weak · 3–4 steady · 5–6 surge (a 6 adds Strain)")),
    reroll,
    btn("Keep", () => {
      audio.play("lock_in");
      app.send({ t: "reroll", seat, reroll: false });
      G.pending = true;
      render(v);
    }, "primary"),
  );
  return wrap;
}

// ---------------------------------------------------------------- selection

function selectCard(sel: Sel): void {
  audio.unlock();
  if (G.sel && G.sel.seat === sel.seat && G.sel.index === sel.index) {
    clearSelection(true);
    return;
  }
  G.sel = sel;
  if (S.view) render(S.view);
}

function clearSelection(rerender: boolean): void {
  G.sel = null;
  if (rerender && S.view) render(S.view);
}

function chamberState(v: View, ch: Chamber): "valid" | "invalid" | "locked" | "" {
  if (!G.sel) return "";
  const s = sv(v, G.sel.seat);
  const c = card(G.sel.key);
  if (!c.valid_chambers.includes(ch)) return "invalid";
  if (s.locked_chambers.includes(ch)) return "locked";
  return "valid";
}

function submitChoice(seat: Seat, key: string, ch: Chamber, intent: Intent): void {
  G.pending = true;
  G.diceShownRaw = null;
  G.sel = null;
  audio.play("lock_in");
  if (!app.send({ t: "choose", seat, card: key, chamber: ch, intent })) {
    G.pending = false;
    toast("Not connected. Reconnecting…", "error");
  }
  if (S.view) render(S.view);
}

function openIntent(v: View, ch: Chamber): void {
  if (!G.sel) return;
  const { seat, key } = G.sel;
  const c = card(key);
  const s = sv(v, seat);
  const cv = v.chambers.find((x) => x.chamber === ch)!;
  const f = c.faction;
  const own = f === "heaven" ? cv.heaven_total : cv.hell_total;
  const enemy = f === "heaven" ? cv.hell_total : cv.heaven_total;
  const harmony = (f === "heaven" ? cv.heaven : cv.hell).filter((t) => t.aspect === c.aspect).length;
  const pb = pressBonus(own, harmony, enemy);
  const d = isDisruptive(c) ? 1 : 0;
  const hb = harmony > 0 ? 1 : 0;
  const notes: string[] = [];
  if (c.prices.includes("Heavy")) notes.push(`Heavy: spends 1 Will automatically (you have ${s.will}); without Will it loses 1 force.`);
  if (c.prices.includes("Hungry")) notes.push(own > 0 ? "Hungry: eats 1 friendly stack here for +1 force (not with Release)." : "Hungry: no friendly stack here, so −1 force (not with Release).");
  if (c.prices.includes("Volatile")) notes.push("Volatile: a 1–2 backfires (−2 force, +1 Strain); a 5–6 adds +1.");
  if (c.prices.includes("Delayed")) notes.push("Delayed: its movement lands at the start of next round.");
  if (c.prices.includes("Echo")) notes.push("Echo: if it moves the Soul, a ±1 echo lands next round.");
  if (c.prices.includes("Binding")) notes.push(`Binding: you can't use ${ch} next round.`);
  if (c.prices.includes("Fragile")) notes.push("Fragile: −1 force if your opponent plays the same Chamber.");
  if (c.prices.includes("Desperate")) {
    const behind = (f === "heaven" && v.soul < 0) || (f === "hell" && v.soul > 0);
    const ahead = (f === "heaven" && v.soul > 0) || (f === "hell" && v.soul < 0);
    notes.push(`Desperate: ${behind ? "+1 now (you're behind)" : ahead ? "−1 now (you're ahead)" : "neutral at 0"}.`);
  }
  const m = modal(
    `${c.name} → ${ch}`,
    h(
      "div",
      { class: "intent" },
      h(
        "div",
        { class: "intent-head" },
        glyph(c.emoji, { size: "xl", faction: f }),
        h("span", { class: "arrow" }, "→"),
        glyph(CHAMBER_GLYPH[ch], { size: "lg", plain: true }),
        h(
          "div",
          { class: "intent-stats" },
          h("div", null, `Your stack here ◈${own} · enemy ◈${enemy} · harmony ${harmony}`),
          h("div", { class: "help" }, c.help_text),
        ),
      ),
      h(
        "div",
        { class: "intent-grid" },
        intentTile("Press", `Move now${pb ? ` with +${pb} Resonance` : ""}; add ◈${c.stack_value} stack.`, `+${pb}`, () => pick({ kind: "press" })),
        intentTile("Anchor", `No Soul move now. Invest ◈${c.stack_value + 1 + hb}–${c.stack_value + 3 + hb} stack for later.`, "invest", () => pick({ kind: "anchor" })),
        intentTile(
          "Sever",
          enemy > 0 ? `Cut enemy stack by ${c.stack_value + 1 + d}–${c.stack_value + 3 + d}; your effect −1, add ◈${Math.max(1, c.stack_value - 1)}.` : "No enemy stack here to cut. Effect −1.",
          enemy > 0 ? "cut" : "—",
          () => pick({ kind: "sever" }),
          enemy === 0,
        ),
        releaseTile(own, (spend) => pick({ kind: "release", spend })),
      ),
      notes.length ? h("ul", { class: "notes" }, notes.map((n) => h("li", null, n))) : null,
    ),
    { cls: `intent-modal faction-${f}` },
  );
  function pick(intent: Intent) {
    m.close();
    submitChoice(seat, key, ch, intent);
  }
}

function intentTile(name: string, text: string, badge: string, onPick: () => void, weak = false): HTMLElement {
  const t = h("button", { class: `intent-tile ${weak ? "weak" : ""}`, type: "button" }, h("div", { class: "it-name" }, name, h("span", { class: "it-badge" }, badge)), h("div", { class: "it-text" }, text));
  t.addEventListener("click", onPick);
  t.addEventListener("pointerenter", () => audio.play("ui_hover"));
  return t;
}

function releaseTile(own: number, onPick: (spend: number) => void): HTMLElement {
  if (own <= 0) return h("div", { class: "intent-tile disabled" }, h("div", { class: "it-name" }, "Release"), h("div", { class: "it-text" }, "No friendly stack here to release."));
  const spends: number[] = [];
  for (let i = 1; i <= Math.min(own, 6); i++) spends.push(i);
  if (own > 6) spends.push(own);
  return h(
    "div",
    { class: "intent-tile release" },
    h("div", { class: "it-name" }, "Release"),
    h("div", { class: "it-text" }, "Spend stack now for a burst (chosen before the roll):"),
    h(
      "div",
      { class: "spends" },
      spends.map((n) =>
        btn(
          h("span", null, h("b", null, n === own && own > 6 ? `All ${n}` : String(n)), h("small", null, ` +${releaseBonus(n, 1)}–${releaseBonus(n, 3)}`)),
          () => onPick(n),
          "spend",
        ),
      ),
    ),
  );
}

// ---------------------------------------------------------------- pillar

function soulPct(v: View, soul: number): number {
  return ((v.goal - soul) / (2 * v.goal)) * 100;
}

function renderPillar(v: View): void {
  const el = document.getElementById("g-pillar");
  if (!el) return;
  const ticks: HTMLElement[] = [];
  for (let val = -v.goal; val <= v.goal; val += 2) {
    const major = val % 6 === 0;
    ticks.push(h("div", { class: `tick ${major ? "major" : ""} ${val === 0 ? "zero" : ""}`, style: `top:${soulPct(v, val)}%` }, major && Math.abs(val) !== v.goal ? h("span", null, val > 0 ? `+${val}` : String(val)) : null));
  }
  const ghosts = v.delayed_next.map((d) => {
    const at = Math.max(-v.goal, Math.min(v.goal, v.soul + d.amount));
    return h("div", { class: `ghost-mark ${d.amount >= 0 ? "heaven" : "hell"}`, style: `top:${soulPct(v, at)}%`, title: `${d.label} ${d.amount > 0 ? "+" : ""}${d.amount} next round` }, "⏳");
  });
  mount(
    el,
    h("div", { class: "pillar-end heaven" }, glyph("😇", { size: "sm", faction: "heaven" }), h("span", null, `+${v.goal}`)),
    h(
      "div",
      { class: "track", id: "g-track" },
      h("div", { class: "track-glow" }),
      ticks,
      h("div", { class: "fill", id: "g-fill" }),
      ghosts,
      h("div", { class: "orb", id: "g-orb", style: `top:${soulPct(v, v.soul)}%` }, glyph("⚖️", { size: "md", plain: true }), h("span", { class: "soul-num", id: "g-soulnum" }, v.soul > 0 ? `+${v.soul}` : String(v.soul))),
    ),
    h("div", { class: "pillar-end hell" }, h("span", null, `−${v.goal}`), glyph("😈", { size: "sm", faction: "hell" })),
  );
  setFill(v, v.soul);
}

function setFill(v: View, soul: number): void {
  const fill = document.getElementById("g-fill");
  if (!fill) return;
  const p = soulPct(v, soul);
  if (soul >= 0) {
    fill.style.top = `${p}%`;
    fill.style.height = `${50 - p}%`;
    fill.className = "fill heaven";
  } else {
    fill.style.top = "50%";
    fill.style.height = `${p - 50}%`;
    fill.className = "fill hell";
  }
}

async function animateSoul(v: View, to: number, ms: number): Promise<void> {
  const orb = document.getElementById("g-orb");
  const num = document.getElementById("g-soulnum");
  if (!orb) return;
  const from = parseFloat(orb.style.top) || soulPct(v, 0);
  const target = soulPct(v, to);
  const dur = ms / prefs.speed;
  const a = orb.animate([{ top: `${from}%` }, { top: `${target}%` }], { duration: dur, easing: "cubic-bezier(.3,1.4,.4,1)", fill: "forwards" });
  if (!prefs.reducedMotion) {
    const trail = setInterval(() => {
      const r = orb.getBoundingClientRect();
      fx.burst(r.left + r.width / 2, r.top + r.height / 2, to >= 0 ? "#ffe6a8" : "#ff7a55", 2, 40, 2);
    }, 40);
    setTimeout(() => clearInterval(trail), dur);
  }
  await a.finished.catch(() => undefined);
  orb.style.top = `${target}%`;
  a.cancel();
  if (num) num.textContent = to > 0 ? `+${to}` : String(to);
  setFill(v, to);
}

// ---------------------------------------------------------------- chambers

function tokenEl(t: { card: string; value: number; aspect: string }, f: Faction): HTMLElement {
  return h("span", { class: `token ${f}`, title: `${card(t.card).name} · ${t.aspect} · ${t.value}` }, glyph(card(t.card).emoji, { size: "xs", plain: true }), h("b", null, String(t.value)));
}

function renderChambers(v: View): void {
  const el = document.getElementById("g-chambers");
  if (!el) return;
  const cols = CHAMBERS.map((ch) => chamberColumn(v, v.chambers.find((c) => c.chamber === ch)!));
  mount(el, cols);
}

function chamberColumn(v: View, cv: ChamberView): HTMLElement {
  const ch = cv.chamber;
  const state = chamberState(v, ch);
  const forced = G.sel && sv(v, G.sel.seat).forced;
  const effective = forced ? (forced.chamber === ch && forced.card === G.sel!.key ? "valid" : "invalid") : state;
  const lockedFor = v.seats.filter((s) => s.locked_chambers.includes(ch)).map((s) => s.faction);
  let preview: string | null = null;
  if (effective === "valid" && G.sel) {
    const c = card(G.sel.key);
    const own = c.faction === "heaven" ? cv.heaven_total : cv.hell_total;
    const enemy = c.faction === "heaven" ? cv.hell_total : cv.heaven_total;
    const harmony = (c.faction === "heaven" ? cv.heaven : cv.hell).filter((t) => t.aspect === c.aspect).length;
    const pb = pressBonus(own, harmony, enemy);
    preview = pb ? `Press +${pb}` : "Play here";
  }
  const col = h(
    "div",
    { class: `chamber ${effective} ${cv.conditions.includes("Cold") ? "cold" : ""} ${cv.conditions.includes("Thorn") ? "thorn" : ""}`, data: { chamber: ch }, tabindex: effective === "valid" ? "0" : undefined },
    h(
      "div",
      { class: "ch-head" },
      glyph(CHAMBER_GLYPH[ch], { size: "md", plain: true }),
      h("span", { class: "ch-name" }, ch),
      lockedFor.map((f) => h("span", { class: `lockmark ${f}`, title: `${FACTION_NAME[f]} can't use ${ch} this round (Binding)` }, "🔒")),
    ),
    zone(cv, "heaven"),
    h(
      "div",
      { class: "ch-mid" },
      cv.effects.map((e) =>
        h(
          "span",
          { class: `effect ${e.faction}`, data: { card: e.card, faction: e.faction }, title: `${card(e.card).name}: ${card(e.card).help_text}` },
          glyph(card(e.card).emoji, { size: "sm", faction: e.faction }),
        ),
      ),
      cv.conditions.map((c) => h("span", { class: `condition ${c.toLowerCase()}`, title: c === "Cold" ? "Cold: weakens Candle, Small Comfort and Sweetness here by 1" : "Thorn: weakens Loved Memory here by 2, then fades" }, glyph(c === "Cold" ? "❄️" : "🥀", { size: "sm" }))),
      preview ? h("span", { class: "preview" }, preview) : null,
    ),
    zone(cv, "hell"),
  );
  if (effective === "valid") {
    const go = () => {
      audio.play("ui_confirm");
      openIntent(v, ch);
    };
    col.addEventListener("click", go);
    col.addEventListener("keydown", (e) => {
      if (e.key === "Enter") go();
    });
    col.addEventListener("pointerenter", () => audio.play("ui_hover"));
  } else if (effective === "locked" || effective === "invalid") {
    col.addEventListener("click", () => {
      audio.play("ui_error");
      toast(effective === "locked" ? `${ch} is Binding-locked for you this round.` : `${card(G.sel!.key).name} can't be played in ${ch}.`, "error", 2200);
    });
  }
  return col;
}

function zone(cv: ChamberView, f: Faction): HTMLElement {
  const tokens = f === "heaven" ? cv.heaven : cv.hell;
  const total = f === "heaven" ? cv.heaven_total : cv.hell_total;
  return h(
    "div",
    { class: `zone ${f}`, data: { faction: f } },
    h("div", { class: "tokens" }, tokens.map((t) => tokenEl(t, f))),
    total ? h("span", { class: "total", title: `${FACTION_NAME[f]} Resonance here` }, `◈${total}`) : null,
  );
}

// ---------------------------------------------------------------- log

function appendLogDom(text: string): void {
  const box = document.getElementById("g-loglines");
  if (!box) return;
  let cls = "line";
  if (text.startsWith("=== Round")) cls += " round";
  else if (text.startsWith("Purgatory Event")) cls += " purgatory";
  else if (text.includes("WINS")) cls += " win";
  else if (text.startsWith("Clash in")) cls += " clash";
  else if (/toward Heaven|Heaven adds|→ \+/.test(text)) cls += " heaven";
  else if (/toward Hell|Hell adds/.test(text)) cls += " hell";
  const stick = box.scrollTop + box.clientHeight >= box.scrollHeight - 30;
  box.appendChild(h("div", { class: cls }, text || " "));
  while (box.childElementCount > 600) box.firstElementChild?.remove();
  if (stick) box.scrollTop = box.scrollHeight;
}

// ---------------------------------------------------------------- after render

function afterRender(v: View): void {
  if (G.dealt.size) {
    audio.play("deal");
    setTimeout(() => G.dealt.clear(), 50);
  }
  G.diceRolling = false;
  audio.setLean(v.soul / v.goal);
  if (v.over && !G.summaryShown) {
    G.summaryShown = true;
    void sleep(400).then(() => showSummary(v, false));
  }
  // Secret pass-and-play: a new acting seat must be unveiled again.
  if (v.mode === "secret_hotseat" && actingSeat(v) !== G.unveiled) G.unveiled = null;
  app.presence();
}

export function showSummary(v: View, again: boolean): void {
  const iPlay = mySeats(v).length > 0;
  const winner = v.winner ?? "heaven";
  const m = modal(
    winner === "heaven" ? "Heaven wins: the Soul ascends" : "Hell wins: the Soul descends",
    h(
      "div",
      { class: `summary ${winner}` },
      h("div", { class: "summary-glyph" }, glyph(winner === "heaven" ? "😇" : "😈", { size: "xl", faction: winner })),
      h("pre", { class: "summary-text" }, v.summary ?? ""),
      h(
        "div",
        { class: "row end" },
        btn("Export logs", () => app.send({ t: "export" }), "ghost"),
        btn("Lobby", () => {
          m.close();
          app.go("lobby");
        }, "ghost"),
        iPlay ? btn("Rematch", () => {
          m.close();
          app.send({ t: "rematch" });
        }, "primary") : null,
      ),
    ),
    { wide: true, cls: "summary-modal" },
  );
  if (!again) audio.setScene("endgame");
}

// ---------------------------------------------------------------- anim ctx

function animCtx(next: View): AnimCtx {
  const v = G.shown ?? next;
  return {
    view: G.shown,
    next,
    mine: mySeats(next),
    chamberEl: (ch) => document.querySelector<HTMLElement>(`.chamber[data-chamber="${ch}"]`),
    zoneEl: (ch, f) => document.querySelector<HTMLElement>(`.chamber[data-chamber="${ch}"] .zone.${f}`),
    effectEl: (ch, f, key) => document.querySelector<HTMLElement>(`.chamber[data-chamber="${ch}"] .effect[data-card="${key}"][data-faction="${f}"]`),
    orbEl: () => document.getElementById("g-orb"),
    handCardEl: (seat, key) => document.querySelector<HTMLElement>(`.hand[data-seat="${seat}"] .card[data-key="${key}"]:not([style*="hidden"])`),
    seatEl: (seat) => document.querySelector<HTMLElement>(`.seat-panel[data-seat="${seat}"]`),
    pillarEl: () => document.getElementById("g-track"),
    setSoul: (value, ms) => animateSoul(v, value, ms),
    setStrain: (n) => {
      const el = document.getElementById("g-strain");
      if (el) mount(el, strainPips(n));
    },
    logAppend: (t) => {
      G.log.push(t);
      if (G.log.length > 2000) G.log.splice(0, G.log.length - 2000);
      if (!logHiddenForSecret(next)) appendLogDom(t);
    },
    dealt: (seat) => G.dealt.add(seat),
    diceReroll: (seat, to) => {
      const die = document.getElementById("g-die");
      if (!die || !mySeats(next).includes(seat)) return false;
      const face = dieFace(to);
      die.replaceWith(face);
      face.animate([{ transform: "rotate(-320deg) scale(0.5)" }, { transform: "rotate(12deg) scale(1.15)", offset: 0.8 }, { transform: "rotate(0) scale(1)" }], { duration: 520, easing: "ease-out" });
      audio.play("dice_roll");
      G.diceShownRaw = to;
      return true;
    },
  };
}

/** The server rejected an action: unfreeze the controls. */
export function onServerError(): void {
  G.pending = false;
  G.diceShownRaw = null;
  if (gameMounted() && S.view && !G.animating) render(S.view);
}
