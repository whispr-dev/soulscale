// Tiny DOM builder and helpers (no framework).

export type Child = Node | string | number | null | undefined | false | Child[];
type Handler = (e: Event) => void;
export type Attrs = {
  class?: string;
  style?: string;
  on?: Record<string, Handler>;
  data?: Record<string, string | number>;
  [key: string]: unknown;
};

const PROPS = new Set(["value", "checked", "disabled", "selected", "hidden", "title", "type", "min", "max", "step"]);

export function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs?: Attrs | null,
  ...children: Child[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  if (attrs) {
    for (const [k, v] of Object.entries(attrs)) {
      if (v === undefined || v === null || v === false) continue;
      if (k === "class") el.className = String(v);
      else if (k === "style") el.style.cssText = String(v);
      else if (k === "on") {
        for (const [ev, fn] of Object.entries(v as Record<string, Handler>)) el.addEventListener(ev, fn);
      } else if (k === "data") {
        for (const [dk, dv] of Object.entries(v as Record<string, string | number>)) el.dataset[dk] = String(dv);
      } else if (PROPS.has(k)) {
        (el as unknown as Record<string, unknown>)[k] = v;
      } else if (v === true) el.setAttribute(k, "");
      else el.setAttribute(k, String(v));
    }
  }
  append(el, children);
  return el;
}

export function append(el: Node, children: Child[]): void {
  for (const c of children) {
    if (c === null || c === undefined || c === false) continue;
    if (Array.isArray(c)) append(el, c);
    else if (c instanceof Node) el.appendChild(c);
    else el.appendChild(document.createTextNode(String(c)));
  }
}

export function clear(el: Element): void {
  while (el.firstChild) el.removeChild(el.firstChild);
}

export function mount(el: Element, ...children: Child[]): void {
  clear(el);
  append(el, children);
}

export function $(sel: string, root: ParentNode = document): HTMLElement | null {
  return root.querySelector<HTMLElement>(sel);
}

export function byId(id: string): HTMLElement {
  const el = document.getElementById(id);
  if (!el) throw new Error(`#${id} missing from index.html`);
  return el;
}

export const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, Math.max(0, ms)));

export function centerOf(el: Element): { x: number; y: number } {
  const r = el.getBoundingClientRect();
  return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
}

export function signed(n: number): string {
  return n > 0 ? `+${n}` : String(n);
}

/** Copy text to the clipboard, with a fallback for iframes that block the async API. */
export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const ta = h("textarea", { style: "position:fixed;left:-9999px;top:0" }, text);
    document.body.appendChild(ta);
    ta.select();
    let ok = false;
    try {
      ok = document.execCommand("copy");
    } catch {
      ok = false;
    }
    ta.remove();
    return ok;
  }
}

/** Offer a text file download (may be blocked inside some Discord clients; callers also offer copy). */
export function download(name: string, text: string, type = "text/plain"): void {
  const url = URL.createObjectURL(new Blob([text], { type: `${type};charset=utf-8` }));
  const a = h("a", { href: url, download: name, style: "display:none" });
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 5000);
}
