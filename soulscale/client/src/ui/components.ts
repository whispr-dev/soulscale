// Shared UI pieces: glyph cards (with 3D tilt + shine), card backs, modals, chips.

import { byId, h, type Child } from "../dom";
import { glyph } from "../glyph";
import type { CardDef, Faction, UserInfo } from "../protocol";
import { CHAMBER_GLYPH } from "../protocol";
import { audio } from "../audio";
import { card, prefs } from "../store";

export const FACTION_GLYPH: Record<Faction, string> = { heaven: "😇", hell: "😈" };
export const FACTION_NAME: Record<Faction, string> = { heaven: "Heaven", hell: "Hell" };

export type CardSize = "hand" | "small" | "lib" | "ghost";

/** Pointer-follow tilt + specular shine (skipped for reduced motion). */
function tilt(el: HTMLElement): void {
  if (prefs.reducedMotion) return;
  el.addEventListener("pointermove", (e) => {
    const r = el.getBoundingClientRect();
    const px = (e.clientX - r.left) / r.width;
    const py = (e.clientY - r.top) / r.height;
    el.style.setProperty("--rx", `${(0.5 - py) * 14}deg`);
    el.style.setProperty("--ry", `${(px - 0.5) * 18}deg`);
    el.style.setProperty("--mx", `${px * 100}%`);
    el.style.setProperty("--my", `${py * 100}%`);
  });
  el.addEventListener("pointerleave", () => {
    el.style.setProperty("--rx", "0deg");
    el.style.setProperty("--ry", "0deg");
  });
}

function moveLine(c: CardDef): string {
  const parts: string[] = [];
  if (c.base_move) parts.push(`move +${c.base_move}`);
  if (c.ongoing_move) parts.push(`ongoing +${c.ongoing_move}/rnd`);
  if (c.guard) parts.push("guard −2");
  if (c.trap) parts.push("trap −3");
  if (!parts.length) parts.push("special");
  return parts.join(" · ");
}

export function cardEl(
  key: string,
  o: { size?: CardSize; selected?: boolean; disabled?: boolean; onClick?: () => void; count?: number; dim?: boolean } = {},
): HTMLElement {
  const c = card(key);
  const size = o.size ?? "hand";
  const cls = [
    "card",
    `card-${c.faction}`,
    `rarity-${c.rarity.toLowerCase()}`,
    `size-${size}`,
    o.selected ? "selected" : "",
    o.disabled ? "disabled" : "",
    o.onClick && !o.disabled ? "clickable" : "",
    o.dim ? "dim" : "",
  ].join(" ");
  const el = h(
    "div",
    {
      class: cls,
      data: { key },
      title: `${c.name} (${c.rarity}) — ${c.help_text}`,
      tabindex: o.onClick && !o.disabled ? "0" : undefined,
      role: o.onClick ? "button" : undefined,
    },
    h(
      "div",
      { class: "card-inner" },
      h("div", { class: "card-shine" }),
      h(
        "div",
        { class: "card-top" },
        h("span", { class: "rarity-gem", title: c.rarity }, c.rarity[0]),
        h("span", { class: "card-stack", title: "Stack value (Resonance added on Press)" }, `◈${c.stack_value}`),
      ),
      h("div", { class: "card-art" }, glyph(c.emoji, { faction: c.faction, size: size === "hand" ? "xl" : size === "ghost" ? "xl" : "lg" })),
      h("div", { class: "card-name" }, c.name),
      size !== "small" ? h("div", { class: "card-type" }, c.type_label) : null,
      size !== "small" ? h("div", { class: "card-stats" }, `${moveLine(c)} · power ${c.base_power}`) : null,
      c.prices.length && size !== "small" ? h("div", { class: "card-prices" }, c.prices.map((p) => h("span", { class: "price" }, p))) : null,
      h(
        "div",
        { class: "card-chambers" },
        c.valid_chambers.map((ch) => glyph(CHAMBER_GLYPH[ch], { size: "xs", plain: true, title: ch })),
      ),
      o.count && o.count > 1 ? h("div", { class: "card-count" }, `×${o.count}`) : null,
    ),
  );
  if (o.onClick && !o.disabled) {
    el.addEventListener("click", () => {
      audio.play("ui_select");
      o.onClick!();
    });
    el.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        o.onClick!();
      }
    });
    el.addEventListener("pointerenter", () => audio.play("ui_hover"));
  }
  if (size === "hand" || size === "lib") tilt(el);
  return el;
}

export function cardBack(faction: Faction, size: CardSize = "small"): HTMLElement {
  return h(
    "div",
    { class: `card back back-${faction} size-${size}` },
    h("div", { class: "card-inner" }, h("div", { class: "back-sigil" }, glyph(FACTION_GLYPH[faction], { size: "lg", faction }))),
  );
}

export function avatar(u: UserInfo | null | undefined, fallback: string, faction?: Faction): HTMLElement {
  const ring = faction ? ` ring-${faction}` : "";
  if (u?.avatar) {
    return h("img", { class: `avatar${ring}`, src: u.avatar, alt: "", referrerpolicy: "no-referrer" });
  }
  const name = u?.name ?? fallback;
  return h("div", { class: `avatar avatar-letter${ring}` }, (name.trim()[0] ?? "?").toUpperCase());
}

export function btn(label: Child, onClick: () => void, cls = "", title?: string): HTMLButtonElement {
  const b = h("button", { class: `btn ${cls}`, title, type: "button" }, label);
  b.addEventListener("click", () => {
    audio.unlock();
    audio.play("ui_click");
    onClick();
  });
  b.addEventListener("pointerenter", () => audio.play("ui_hover"));
  return b;
}

export function chip(text: Child, cls = "", title?: string): HTMLElement {
  return h("span", { class: `chip ${cls}`, title }, text);
}

let openModals = 0;
export function modal(
  title: string,
  body: Child,
  o: { wide?: boolean; onClose?: () => void; dismissable?: boolean; cls?: string } = {},
): { el: HTMLElement; close: () => void } {
  const root = byId("modal-root");
  const dismissable = o.dismissable !== false;
  let closed = false;
  const close = () => {
    if (closed) return;
    closed = true;
    openModals--;
    wrap.classList.add("closing");
    setTimeout(() => wrap.remove(), 180);
    o.onClose?.();
  };
  const panel = h(
    "div",
    { class: `modal ${o.wide ? "wide" : ""} ${o.cls ?? ""}`, role: "dialog", "aria-modal": "true", "aria-label": title },
    h(
      "div",
      { class: "modal-head" },
      h("h2", null, title),
      dismissable ? btn("✕", () => {
        audio.play("ui_back");
        close();
      }, "ghost icon", "Close") : null,
    ),
    h("div", { class: "modal-body" }, body),
  );
  const wrap = h("div", { class: "modal-wrap" }, panel);
  if (dismissable) {
    wrap.addEventListener("pointerdown", (e) => {
      if (e.target === wrap) close();
    });
  }
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape" && dismissable && !closed) {
      close();
      removeEventListener("keydown", onKey);
    }
  };
  addEventListener("keydown", onKey);
  root.appendChild(wrap);
  openModals++;
  return { el: panel, close };
}

export const anyModalOpen = () => openModals > 0;

export function confirmBox(text: string, yes: string, onYes: () => void): void {
  const m = modal(
    "Are you sure?",
    h(
      "div",
      { class: "confirm" },
      h("p", null, text),
      h(
        "div",
        { class: "row end" },
        btn("Cancel", () => m.close(), "ghost"),
        btn(yes, () => {
          m.close();
          onYes();
        }, "danger"),
      ),
    ),
  );
}
