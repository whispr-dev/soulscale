// Event sequencer: plays a round's GameEvents as timed animation + sound, in the exact
// order the engine resolved them. The board then settles on the server's snapshot.

import { audio } from "../audio";
import { centerOf, h, sleep } from "../dom";
import { banner, COLORS, edgeGlow, floatText, fx, overlay, shake } from "../fx";
import { glyph } from "../glyph";
import type { Chamber, Faction, GameEvent, Seat, View } from "../protocol";
import { seatIdx } from "../protocol";
import { card, prefs } from "../store";
import { cardEl } from "./components";

export interface AnimCtx {
  view: View | null; // the view currently on screen (before these events)
  next: View; // the snapshot we settle on afterwards
  mine: Seat[];
  chamberEl(ch: Chamber): HTMLElement | null;
  zoneEl(ch: Chamber, f: Faction): HTMLElement | null;
  effectEl(ch: Chamber, f: Faction, key: string): HTMLElement | null;
  orbEl(): HTMLElement | null;
  handCardEl(seat: Seat, key: string): HTMLElement | null;
  seatEl(seat: Seat): HTMLElement | null;
  pillarEl(): HTMLElement | null;
  setSoul(value: number, ms: number): Promise<void>;
  setStrain(value: number): void;
  logAppend(text: string): void;
  dealt(seat: Seat): void;
  diceReroll(seat: Seat, to: number): boolean;
}

export const PURGATORY_TEXT: Record<string, string> = {
  Collapse: "The next Hell push gets +2.",
  Reprieve: "The next Heaven push gets +2.",
  Flashback: "Memory conditions clear; both Memory stacks lose 2.",
  Stasis: "Next round's Soul movement is capped at ±2.",
  Intensification: "The next active effect gets +1.",
  Judgement: "The side behind gains Will next round.",
};

const MOD_LABEL: Record<string, string> = {
  support: "support",
  reprieve: "Reprieve",
  collapse: "Collapse",
  intensification: "Intensified",
  cold: "Cold",
  thorn: "Thorn",
  heavy: "Heavy",
  desperate: "Desperate",
  volatile: "Volatile",
  press: "Press",
  release: "Release",
  hungry: "Hungry",
  fragile: "Fragile",
};

const wait = (ms: number) => sleep(ms / prefs.speed);
const factionOfSeat = (v: View, s: Seat): Faction => v.seats[seatIdx(s)].faction;
const color = (f: Faction) => (f === "heaven" ? COLORS.heaven : COLORS.hell);

export async function animateEvents(events: GameEvent[], ctx: AnimCtx): Promise<void> {
  const ghosts = new Map<Seat, HTMLElement>();
  const v = ctx.next;
  const fast = prefs.speed >= 4;

  const ghostAt = (seat: Seat): { x: number; y: number } | null => {
    const g = ghosts.get(seat);
    return g ? centerOf(g) : null;
  };
  const orb = () => {
    const o = ctx.orbEl();
    return o ? centerOf(o) : { x: innerWidth / 2, y: innerHeight / 2 };
  };

  for (const e of events) {
    switch (e.t) {
      case "log":
        ctx.logAppend(e.text);
        break;

      case "round_start":
        if (e.round > 1 && !fast) {
          audio.play("round_start");
          void banner(`Round ${e.round}`, "", "round", 900 / prefs.speed);
        }
        break;

      case "reveal":
        ctx.dealt(e.seat);
        break;

      case "delayed_resolve": {
        const p = ctx.pillarEl();
        if (p) {
          const pc = centerOf(p);
          const o = orb();
          fx.beam(pc.x, pc.y, o.x, o.y, e.total >= 0 ? COLORS.heaven : COLORS.hell, 5);
          floatText(pc.x + 30, pc.y, `⏳ ${e.total > 0 ? "+" : ""}${e.total}`, e.total >= 0 ? "heaven" : "hell");
        }
        audio.play(e.total >= 0 ? "delayed_heaven" : "delayed_hell");
        await ctx.setSoul(e.to, 600);
        audio.setLean(e.to / v.goal);
        break;
      }

      case "plays": {
        audio.play("reveal");
        const sameChamber = e.plays[0].chamber === e.plays[1].chamber;
        const flights: Promise<void>[] = [];
        e.plays.forEach((p, i) => {
          const target = ctx.chamberEl(p.chamber);
          if (!target) return;
          const src = ctx.handCardEl(p.seat, p.card) ?? ctx.seatEl(p.seat);
          const from = src ? src.getBoundingClientRect() : new DOMRect(innerWidth / 2, innerHeight, 0, 0);
          if (src?.classList.contains("card")) src.style.visibility = "hidden";
          const tr = target.getBoundingClientRect();
          const ghost = cardEl(p.card, { size: "ghost" });
          ghost.classList.add("fly");
          // Two plays in one Chamber sit side by side inside it; one play gets most of its width.
          const gw = Math.min(118, tr.width * (sameChamber ? 0.47 : 0.86));
          const gh = gw * 1.4;
          const tx = tr.left + tr.width / 2 + (sameChamber ? (i === 0 ? -gw * 0.53 : gw * 0.53) : 0) - gw / 2;
          const ty = tr.top + tr.height / 2 - gh / 2;
          Object.assign(ghost.style, { left: `${tx}px`, top: `${ty}px` });
          ghost.style.setProperty("--card-w", `${gw}px`); // scales the art and text with the card
          overlay().appendChild(ghost);
          ghosts.set(p.seat, ghost);
          const hidden = !ctx.mine.includes(p.seat) && !(ctx.view && ctx.view.seats[seatIdx(p.seat)].hand);
          const dx = from.left + from.width / 2 - (tx + gw / 2);
          const dy = from.top + from.height / 2 - (ty + gh / 2);
          const anim = ghost.animate(
            [
              { transform: `translate(${dx}px,${dy}px) scale(0.7) rotateY(${hidden ? 180 : 0}deg)`, opacity: 0.4 },
              { transform: `translate(${dx * 0.3}px,${dy * 0.3 - 40}px) scale(1.12) rotateY(${hidden ? 90 : 0}deg)`, opacity: 1, offset: 0.6 },
              { transform: "translate(0,0) scale(1) rotateY(0deg)", opacity: 1 },
            ],
            { duration: 620 / prefs.speed, easing: "cubic-bezier(.2,.8,.2,1)", fill: "forwards" },
          );
          const f = card(p.card).faction;
          audio.playCard(p.card, f, p.chamber, i * 90);
          flights.push(
            anim.finished.then(
              () => {
                const c = centerOf(ghost);
                fx.burst(c.x, c.y, color(f), 14, 160);
                floatText(c.x, c.y + gh * 0.45, `${p.intent.kind === "release" ? `Release ${p.intent.spend}` : cap(p.intent.kind)} · 🎲${p.raw}`, "small");
              },
              () => undefined,
            ),
          );
        });
        await Promise.all(flights);
        await wait(250);
        break;
      }

      case "modifier": {
        if (fast) break;
        const at = ghostAt(e.seat);
        if (at) {
          const label = MOD_LABEL[e.source] ?? (S_cardName(e.source) ?? e.source);
          floatText(at.x, at.y - 30, `${e.delta > 0 ? "+" : ""}${e.delta} ${label}`, e.delta > 0 ? "buff" : "debuff", 900);
          await wait(160);
        }
        break;
      }

      case "trap_sprung": {
        const ch = ctx.chamberEl(e.chamber);
        if (ch) {
          const c = centerOf(ch);
          fx.shards(c.x, c.y, card(e.card).emoji, 7);
          fx.ring(c.x, c.y, COLORS.hell, 110);
          floatText(c.x, c.y - 50, `${card(e.card).emoji} springs! −3`, "debuff big");
        }
        ghosts.get(e.victim)?.animate([{ transform: "translateX(-8px)" }, { transform: "translateX(8px)" }, { transform: "translateX(0)" }], { duration: 260, iterations: 2 });
        audio.play("trap");
        await wait(450);
        break;
      }

      case "clash": {
        const g1 = ghosts.get("P1");
        const g2 = ghosts.get("P2");
        if (g1 && g2) {
          const bump = (g: HTMLElement, dir: number) =>
            g.animate([{ translate: "0 0" }, { translate: `${dir * 22}px 0` }, { translate: "0 0" }], { duration: 300 / prefs.speed, easing: "ease-in-out" }).finished.catch(() => undefined);
          await Promise.all([bump(g1, 1), bump(g2, -1)]);
        }
        const ch = ctx.chamberEl(e.chamber);
        if (ch) {
          const c = centerOf(ch);
          fx.ring(c.x, c.y, "#ffffff", 140);
          fx.burst(c.x, c.y, COLORS.violet, 30, 320);
          floatText(c.x, c.y - 70, `Clash ${e.power[0]} vs ${e.power[1]}`, "big");
        }
        shake(5);
        audio.play(e.winner ? "clash" : "clash_tie");
        for (const s of ["P1", "P2"] as Seat[]) {
          if (e.winner === null || e.winner !== s) ghosts.get(s)?.classList.add("halved");
        }
        await wait(600);
        break;
      }

      case "fizzle": {
        const g = ghosts.get(e.seat);
        if (g) {
          g.classList.add("fizzled");
          const c = centerOf(g);
          fx.burst(c.x, c.y, "#8a8a9a", 16, 120);
          floatText(c.x, c.y, "fizzles", "debuff");
        }
        audio.play("fizzle");
        await wait(380);
        break;
      }

      case "stack_spent": {
        const z = ctx.zoneEl(e.chamber, e.faction);
        if (z) {
          const a = centerOf(z);
          const o = orb();
          fx.stream(a.x, a.y, o.x, o.y, color(e.faction), 10 + e.amount * 3);
          floatText(a.x, a.y, `−${e.amount} ◈ released`, e.faction);
        }
        audio.play(`release_${e.faction}`);
        await wait(450);
        break;
      }

      case "stack_cut": {
        const z = ctx.zoneEl(e.chamber, e.faction);
        if (z) {
          const r = z.getBoundingClientRect();
          const slash = h("div", { class: "slash", style: `left:${r.left - 10}px;top:${r.top + r.height / 2}px;width:${r.width + 20}px` });
          overlay().appendChild(slash);
          slash.animate([{ clipPath: "inset(0 100% 0 0)" }, { clipPath: "inset(0 0 0 0)", offset: 0.4 }, { clipPath: "inset(0 0 0 0)", opacity: 0 }], { duration: 520 }).finished.then(() => slash.remove(), () => slash.remove());
          const c = centerOf(z);
          fx.burst(c.x, c.y, color(e.faction), 22, 260);
          floatText(c.x, c.y, `✂ −${e.amount} ◈`, "debuff");
        }
        audio.play("sever");
        await wait(480);
        break;
      }

      case "stack_added": {
        const z = ctx.zoneEl(e.chamber, e.faction);
        if (z) {
          z.animate([{ boxShadow: `0 0 0 0 ${color(e.faction)}` }, { boxShadow: `0 0 26px 6px ${color(e.faction)}` }, { boxShadow: "0 0 0 0 transparent" }], { duration: 600 });
          const c = centerOf(z);
          if (!fast) floatText(c.x, c.y, `+${e.value} ◈`, e.faction);
        }
        audio.play(`stack_${e.faction}`);
        await wait(220);
        break;
      }

      case "effect_removed": {
        const el = ctx.effectEl(e.chamber, e.faction, e.card);
        const c = el ? centerOf(el) : ctx.chamberEl(e.chamber) ? centerOf(ctx.chamberEl(e.chamber)!) : null;
        if (el) el.animate([{ opacity: 1, transform: "scale(1)" }, { opacity: 0, transform: "scale(1.8)", filter: "blur(6px)" }], { duration: 450, fill: "forwards" });
        if (c) {
          fx.burst(c.x, c.y, COLORS.heavenHot, 26, 200);
          floatText(c.x, c.y - 20, `${card(e.by).emoji} cleanses ${card(e.card).emoji}`, "heaven");
        }
        audio.play("cleanse");
        await wait(420);
        break;
      }

      case "effect_placed": {
        const ch = ctx.chamberEl(e.chamber);
        if (ch && !e.refreshed) {
          const c = centerOf(ch);
          fx.ring(c.x, c.y, color(e.faction), 70);
          floatText(c.x, c.y, `${card(e.card).emoji} takes hold`, e.faction);
        }
        audio.play(`place_${e.faction}`);
        await wait(e.refreshed ? 120 : 300);
        break;
      }

      case "condition_added": {
        const ch = ctx.chamberEl(e.chamber);
        if (ch) {
          const c = centerOf(ch);
          fx.burst(c.x, c.y, e.condition === "Cold" ? COLORS.frost : "#d0406a", 26, 180);
          floatText(c.x, c.y, e.condition === "Cold" ? "❄️ Cold" : "🥀 Thorn", "hell");
        }
        audio.play(e.condition === "Cold" ? "condition_cold" : "condition_thorn");
        await wait(320);
        break;
      }

      case "condition_removed":
        audio.play("cleanse");
        await wait(120);
        break;

      case "wild": {
        const at = ghostAt(e.seat);
        if (at) floatText(at.x, at.y - 46, `🎲 ${e.roll}`, "big");
        audio.play("wild");
        await wait(450);
        break;
      }

      case "scheduled": {
        const p = ctx.pillarEl();
        if (p) {
          const c = centerOf(p);
          floatText(c.x + 40, c.y - 40, `⏳ ${e.amount > 0 ? "+" : ""}${e.amount} next round`, e.amount >= 0 ? "heaven" : "hell", 1300);
        }
        audio.play(e.amount >= 0 ? "delayed_heaven" : "delayed_hell", { gain: 0.6 });
        await wait(300);
        break;
      }

      case "contribution": {
        if (e.amount === 0) break;
        const at = ghostAt(e.seat);
        const o = orb();
        if (at) {
          fx.beam(at.x, at.y, o.x, o.y, e.amount > 0 ? COLORS.heaven : COLORS.hell, 7);
          floatText(at.x, at.y - 10, `${e.amount > 0 ? "+" : ""}${e.amount}`, `${e.amount > 0 ? "heaven" : "hell"} big`);
        }
        await wait(380);
        break;
      }

      case "ongoing": {
        const el = ctx.effectEl(e.chamber, e.faction, e.card);
        const c = el ? centerOf(el) : null;
        if (el) el.animate([{ transform: "scale(1)" }, { transform: "scale(1.4)" }, { transform: "scale(1)" }], { duration: 380 });
        if (c && e.amount) {
          const o = orb();
          fx.beam(c.x, c.y, o.x, o.y, color(e.faction), 4, 0.5);
          if (!fast) floatText(c.x, c.y - 16, `${e.amount > 0 ? "+" : ""}${e.amount}`, e.faction);
        }
        audio.play(`ongoing_${e.faction}`);
        await wait(280);
        break;
      }

      case "will_next": {
        const s = ctx.next.seats.find((x) => x.faction === e.faction);
        const el = s ? ctx.seatEl(s.seat) : null;
        if (el && !fast) {
          const c = centerOf(el);
          floatText(c.x, c.y, `${e.delta > 0 ? "+" : ""}${e.delta} Will next round`, e.delta > 0 ? "buff" : "debuff", 1000);
        }
        audio.play(e.delta > 0 ? "will_gain" : "will_loss");
        await wait(150);
        break;
      }

      case "reroll": {
        if (ctx.diceReroll(e.seat, e.to)) break;
        const el = ctx.seatEl(e.seat);
        if (el) {
          const c = centerOf(el);
          floatText(c.x, c.y, `🎲 rerolled ${e.from} → ${e.to}`, "small");
        }
        audio.play("dice_reroll");
        await wait(200);
        break;
      }

      case "soul_move": {
        const delta = e.to - e.from;
        if (delta !== 0) {
          const side = delta > 0 ? "heaven" : "hell";
          audio.play(`soul_${side}`);
          if (Math.abs(delta) >= 4) {
            audio.play(`big_${side}`);
            shake(Math.min(14, 4 + Math.abs(delta)));
          }
          edgeGlow(side);
        }
        if (e.uncapped !== delta && delta !== 0) {
          const o = orb();
          floatText(o.x + 40, o.y, `capped (${e.uncapped > 0 ? "+" : ""}${e.uncapped})`, "small");
        }
        await ctx.setSoul(e.to, 750);
        audio.setLean(e.to / v.goal);
        break;
      }

      case "strain":
        ctx.setStrain(e.value);
        if (e.value >= 3) audio.play("strain");
        break;

      case "purgatory": {
        audio.play("purgatory");
        await banner(`Purgatory Event: ${e.name}`, PURGATORY_TEXT[e.name] ?? "", "purgatory", 1700 / prefs.speed, glyph(e.emoji, { size: "xl" }));
        break;
      }

      case "game_over": {
        await wait(300);
        const mineF = ctx.mine.length === 1 ? factionOfSeat(v, ctx.mine[0]) : null;
        audio.play(mineF === null || mineF === e.winner ? "victory" : "defeat");
        audio.play(e.winner === "heaven" ? "ascent" : "descent", { delay: 600 });
        if (e.winner === "heaven") {
          fx.rise(COLORS.heavenHot, 90, true);
          document.body.classList.add("finale-heaven");
          await banner("Heaven wins", "The Soul ascends.", "finale heaven", 2600, glyph("😇", { size: "xl", faction: "heaven" }));
        } else {
          fx.rise(COLORS.hell, 90, false);
          document.body.classList.add("finale-hell");
          await banner("Hell wins", "The Soul descends.", "finale hell", 2600, glyph("😈", { size: "xl", faction: "hell" }));
        }
        document.body.classList.remove("finale-heaven", "finale-hell");
        audio.setScene("endgame");
        break;
      }

      case "locked":
        break;
    }
  }

  for (const g of ghosts.values()) {
    g.animate([{ opacity: 1 }, { opacity: 0, transform: "scale(0.9)" }], { duration: 220, fill: "forwards" }).finished.then(
      () => g.remove(),
      () => g.remove(),
    );
  }
  if (ghosts.size) await wait(200);
}

function cap(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function S_cardName(key: string): string | null {
  try {
    return card(key).name;
  } catch {
    return null;
  }
}
