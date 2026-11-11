// Glyphs are Twemoji SVGs (the emoji art Discord itself uses) so every device sees the
// same piece. CSS gives them the "fattened" sticker outline, faction glow and depth.
// If an SVG is missing we fall back to the system emoji font.

import { h } from "./dom";
import type { Faction } from "./protocol";

export function glyphFile(g: string): string {
  return [...g]
    .map((c) => c.codePointAt(0)!.toString(16))
    .filter((x) => x !== "fe0f")
    .join("-");
}

export const glyphUrl = (g: string) => `glyphs/${glyphFile(g)}.svg`;

export type GlyphOpts = { faction?: Faction | null; size?: "xs" | "sm" | "md" | "lg" | "xl"; plain?: boolean; title?: string };

export function glyph(g: string, o: GlyphOpts = {}): HTMLElement {
  const cls = ["glyph", `g-${o.size ?? "md"}`, o.plain ? "plain" : "fat", o.faction ? `g-${o.faction}` : ""].join(" ");
  const img = h("img", { class: cls, src: glyphUrl(g), alt: g, draggable: "false", title: o.title, decoding: "async" });
  img.addEventListener(
    "error",
    () => {
      const span = h("span", { class: `${cls} glyph-text`, title: o.title }, g);
      img.replaceWith(span);
    },
    { once: true },
  );
  return img;
}

// Cached bitmaps for the particle canvas.
const bitmapCache = new Map<string, HTMLImageElement>();
export function glyphImage(g: string): HTMLImageElement {
  let img = bitmapCache.get(g);
  if (!img) {
    img = new Image();
    img.src = glyphUrl(g);
    bitmapCache.set(g, img);
  }
  return img;
}
