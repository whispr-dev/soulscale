// Visual effects: a full-screen particle canvas (sparks, glyph shards, rings, beams),
// floating numbers, banners and screen shake. The canvas only animates while something
// is alive, so an idle board costs nothing.

import { byId, h } from "./dom";
import { glyphImage } from "./glyph";
import { prefs } from "./store";

type P = {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  max: number;
  size: number;
  color: string;
  kind: "spark" | "glyph" | "ring" | "mote";
  glyph?: string;
  spin?: number;
  rot?: number;
  gravity?: number;
};
type Beam = { x1: number; y1: number; x2: number; y2: number; life: number; max: number; color: string; width: number };

export const COLORS = {
  heaven: "#ffd98a",
  heavenHot: "#fff4cf",
  hell: "#ff5a36",
  hellHot: "#ffb199",
  violet: "#a98bff",
  frost: "#bfe8ff",
};

class Fx {
  private canvas!: HTMLCanvasElement;
  private g!: CanvasRenderingContext2D;
  private ps: P[] = [];
  private beams: Beam[] = [];
  private running = false;
  private dpr = 1;
  private last = 0;

  init(): void {
    this.canvas = byId("fx") as HTMLCanvasElement;
    this.g = this.canvas.getContext("2d")!;
    const resize = () => {
      this.dpr = Math.min(2, window.devicePixelRatio || 1);
      this.canvas.width = Math.floor(innerWidth * this.dpr);
      this.canvas.height = Math.floor(innerHeight * this.dpr);
      this.canvas.style.width = `${innerWidth}px`;
      this.canvas.style.height = `${innerHeight}px`;
    };
    resize();
    addEventListener("resize", resize);
  }

  get calm(): boolean {
    return prefs.reducedMotion;
  }

  private kick(): void {
    if (this.running) return;
    this.running = true;
    this.last = performance.now();
    requestAnimationFrame((t) => this.frame(t));
  }

  private frame(t: number): void {
    const dt = Math.min(0.05, (t - this.last) / 1000);
    this.last = t;
    const g = this.g;
    g.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    g.clearRect(0, 0, innerWidth, innerHeight);
    g.globalCompositeOperation = "lighter";
    for (const b of this.beams) {
      b.life -= dt;
      const a = Math.max(0, b.life / b.max);
      const grad = g.createLinearGradient(b.x1, b.y1, b.x2, b.y2);
      grad.addColorStop(0, "rgba(255,255,255,0)");
      grad.addColorStop(0.5, b.color);
      grad.addColorStop(1, "rgba(255,255,255,0.9)");
      g.globalAlpha = a;
      g.strokeStyle = grad;
      g.lineWidth = b.width * (0.4 + a);
      g.lineCap = "round";
      g.beginPath();
      g.moveTo(b.x1, b.y1);
      g.lineTo(b.x2, b.y2);
      g.stroke();
    }
    this.beams = this.beams.filter((b) => b.life > 0);
    for (const p of this.ps) {
      p.life -= dt;
      p.vy += (p.gravity ?? 0) * dt;
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      p.vx *= 0.985;
      p.vy *= 0.985;
      const a = Math.max(0, p.life / p.max);
      g.globalAlpha = a;
      if (p.kind === "spark" || p.kind === "mote") {
        const r = p.size * (0.5 + a * 0.5);
        const grd = g.createRadialGradient(p.x, p.y, 0, p.x, p.y, r * 3);
        grd.addColorStop(0, p.color);
        grd.addColorStop(1, "rgba(0,0,0,0)");
        g.fillStyle = grd;
        g.beginPath();
        g.arc(p.x, p.y, r * 3, 0, Math.PI * 2);
        g.fill();
      } else if (p.kind === "ring") {
        const r = p.size * (1 - a) + 4;
        g.strokeStyle = p.color;
        g.lineWidth = 4 * a + 1;
        g.beginPath();
        g.arc(p.x, p.y, r, 0, Math.PI * 2);
        g.stroke();
      } else if (p.kind === "glyph" && p.glyph) {
        const img = glyphImage(p.glyph);
        if (img.complete && img.naturalWidth) {
          p.rot = (p.rot ?? 0) + (p.spin ?? 0) * dt;
          g.save();
          g.globalCompositeOperation = "source-over";
          g.translate(p.x, p.y);
          g.rotate(p.rot);
          g.drawImage(img, -p.size / 2, -p.size / 2, p.size, p.size);
          g.restore();
          g.globalCompositeOperation = "lighter";
        }
      }
    }
    this.ps = this.ps.filter((p) => p.life > 0);
    g.globalAlpha = 1;
    if (this.ps.length || this.beams.length) requestAnimationFrame((n) => this.frame(n));
    else {
      g.clearRect(0, 0, innerWidth, innerHeight);
      this.running = false;
    }
  }

  burst(x: number, y: number, color: string, n = 24, speed = 220, size = 3): void {
    const count = this.calm ? Math.ceil(n / 4) : n;
    for (let i = 0; i < count; i++) {
      const a = Math.random() * Math.PI * 2;
      const s = speed * (0.3 + Math.random());
      this.ps.push({ x, y, vx: Math.cos(a) * s, vy: Math.sin(a) * s, life: 0.6 + Math.random() * 0.5, max: 1.1, size: size * (0.6 + Math.random()), color, kind: "spark", gravity: 60 });
    }
    this.kick();
  }

  ring(x: number, y: number, color: string, radius = 90): void {
    this.ps.push({ x, y, vx: 0, vy: 0, life: 0.55, max: 0.55, size: radius, color, kind: "ring" });
    this.kick();
  }

  shards(x: number, y: number, g: string, n = 6): void {
    for (let i = 0; i < (this.calm ? 2 : n); i++) {
      const a = Math.random() * Math.PI * 2;
      const s = 120 + Math.random() * 200;
      this.ps.push({ x, y, vx: Math.cos(a) * s, vy: Math.sin(a) * s - 120, life: 0.9, max: 0.9, size: 14 + Math.random() * 10, color: "#fff", kind: "glyph", glyph: g, spin: (Math.random() - 0.5) * 10, gravity: 520 });
    }
    this.kick();
  }

  /** Particles that travel from a to b (Release draining stack into the Soul, etc). */
  stream(ax: number, ay: number, bx: number, by: number, color: string, n = 18, secs = 0.7): void {
    for (let i = 0; i < (this.calm ? 4 : n); i++) {
      const jitter = 30;
      const sx = ax + (Math.random() - 0.5) * jitter;
      const sy = ay + (Math.random() - 0.5) * jitter;
      const t = secs * (0.7 + Math.random() * 0.4);
      this.ps.push({ x: sx, y: sy, vx: (bx - sx) / t, vy: (by - sy) / t, life: t, max: t, size: 2.5, color, kind: "spark" });
    }
    this.kick();
  }

  beam(x1: number, y1: number, x2: number, y2: number, color: string, width = 6, secs = 0.6): void {
    this.beams.push({ x1, y1, x2, y2, life: secs, max: secs, color, width });
    this.kick();
  }

  rise(color: string, n = 60, up = true): void {
    for (let i = 0; i < (this.calm ? 12 : n); i++) {
      const x = Math.random() * innerWidth;
      const y = up ? innerHeight + 10 : -10;
      this.ps.push({ x, y, vx: (Math.random() - 0.5) * 30, vy: (up ? -1 : 1) * (80 + Math.random() * 160), life: 3 + Math.random() * 2, max: 5, size: 2 + Math.random() * 3, color, kind: "mote" });
    }
    this.kick();
  }
}

export const fx = new Fx();

// ---------------------------------------------------------------- DOM overlays

export function overlay(): HTMLElement {
  return byId("overlay");
}

export function floatText(x: number, y: number, text: string, cls = "", ms = 1100): void {
  const el = h("div", { class: `float-text ${cls}`, style: `left:${x}px;top:${y}px` }, text);
  overlay().appendChild(el);
  el.animate(
    [
      { transform: "translate(-50%,-50%) scale(0.6)", opacity: 0 },
      { transform: "translate(-50%,-90%) scale(1.1)", opacity: 1, offset: 0.2 },
      { transform: "translate(-50%,-180%) scale(1)", opacity: 0 },
    ],
    { duration: prefs.reducedMotion ? 600 : ms, easing: "cubic-bezier(.2,.7,.2,1)" },
  ).finished.then(() => el.remove(), () => el.remove());
}

export async function banner(title: string, sub = "", cls = "", ms = 1500, glyphEl?: HTMLElement): Promise<void> {
  const el = h("div", { class: `banner ${cls}` }, glyphEl ?? null, h("div", { class: "banner-title" }, title), sub ? h("div", { class: "banner-sub" }, sub) : null);
  overlay().appendChild(el);
  const anim = el.animate(
    [
      { opacity: 0, transform: "translate(-50%,-50%) scale(0.85)", filter: "blur(6px)" },
      { opacity: 1, transform: "translate(-50%,-50%) scale(1)", filter: "blur(0)", offset: 0.15 },
      { opacity: 1, transform: "translate(-50%,-50%) scale(1.02)", offset: 0.8 },
      { opacity: 0, transform: "translate(-50%,-50%) scale(1.08)", filter: "blur(4px)" },
    ],
    { duration: ms, easing: "ease-out" },
  );
  await anim.finished.catch(() => undefined);
  el.remove();
}

export function shake(intensity = 6, ms = 380): void {
  if (prefs.reducedMotion) return;
  const el = document.getElementById("screen");
  el?.animate(
    Array.from({ length: 8 }, (_, i) => ({
      transform: i === 7 ? "translate(0,0)" : `translate(${(Math.random() - 0.5) * intensity * 2}px,${(Math.random() - 0.5) * intensity * 2}px)`,
    })),
    { duration: ms },
  );
}

export function edgeGlow(side: "heaven" | "hell"): void {
  const el = h("div", { class: `edge-glow edge-${side}` });
  overlay().appendChild(el);
  el.animate([{ opacity: 0 }, { opacity: 1, offset: 0.25 }, { opacity: 0 }], { duration: 1100 }).finished.then(
    () => el.remove(),
    () => el.remove(),
  );
}

export function toast(text: string, kind: "info" | "error" | "notice" = "info", ms = 3800): void {
  const root = byId("toasts");
  // The same message again just restarts its timer instead of stacking copies.
  for (const el of Array.from(root.children) as HTMLElement[]) {
    if (el.textContent === text && !el.classList.contains("out")) {
      clearTimeout(Number(el.dataset.timer));
      el.dataset.timer = String(expire(el, ms));
      el.animate([{ transform: "scale(1.06)" }, { transform: "scale(1)" }], { duration: 180 });
      return;
    }
  }
  while (root.children.length >= 4) root.firstElementChild?.remove();
  const el = h("div", { class: `toast toast-${kind}` }, text);
  root.appendChild(el);
  el.dataset.timer = String(expire(el, ms));
}

function expire(el: HTMLElement, ms: number): number {
  return window.setTimeout(() => {
    el.classList.add("out");
    setTimeout(() => el.remove(), 400);
  }, ms);
}
