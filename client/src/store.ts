// Client state, card catalog helpers, rule helpers mirrored from the Rust engine
// (for previews only: the server is authoritative), and persisted preferences.

import type { CardDef, Catalog, Faction, RoomInfo, Seat, UserInfo, View } from "./protocol";
import { seatIdx } from "./protocol";

export type Screen = "boot" | "lobby" | "game" | "decks" | "soundlab" | "help";

export const S = {
  screen: "boot" as Screen,
  me: null as UserInfo | null,
  dev: false,
  inDiscord: false,
  catalog: null as Catalog | null,
  cards: new Map<string, CardDef>(),
  room: null as RoomInfo | null,
  view: null as View | null,
  spectating: false,
  /** Hot-seat secret mode: which seat's turn screen has been "unveiled". */
  unveiled: null as Seat | null,
};

export function setCatalog(c: Catalog): void {
  S.catalog = c;
  S.cards = new Map(c.cards.map((x) => [x.key, x]));
}

export function card(key: string): CardDef {
  const c = S.cards.get(key);
  if (!c) throw new Error(`unknown card ${key}`);
  return c;
}

export function factionCards(f: Faction): CardDef[] {
  return (S.catalog?.cards ?? []).filter((c) => c.faction === f);
}

/** Seats this client controls in the current view. */
export function mySeats(v: View | null = S.view): Seat[] {
  if (!v) return [];
  switch (v.viewer.kind) {
    case "seat":
      return [v.viewer.seat];
    case "both":
      return ["P1", "P2"];
    default:
      return [];
  }
}

export function seatOf(v: View, s: Seat) {
  return v.seats[seatIdx(s)];
}

// ---------------------------------------------------------------- rule mirrors

export const RARITY_COST = { Common: 1, Uncommon: 2, Rare: 3 } as const;
export const DECK_SIZE = 10;
export const BUDGET = 24;
export const MAX_RARES = 4;

export function isDisruptive(c: CardDef): boolean {
  return (
    c.type_label.includes("Disruption") ||
    c.type_label.includes("Trap") ||
    c.type_label.includes("Control") ||
    ["calm", "bell", "false_door", "watching_eye", "tangle", "serpent", "acceptance"].includes(c.special)
  );
}

export const pressBonus = (own: number, harmony: number, enemy: number) =>
  Math.max(0, Math.floor((own + harmony - Math.floor(enemy / 2)) / 3));
export const releaseBonus = (spent: number, intensity: number) => Math.min(5, Math.ceil((spent * intensity) / 4));

export function legality(keys: string[]): { size: number; cost: number; rares: number; issues: string[] } {
  const defs = keys.map(card);
  const cost = defs.reduce((a, c) => a + RARITY_COST[c.rarity], 0);
  const rares = defs.filter((c) => c.rarity === "Rare").length;
  const issues: string[] = [];
  if (keys.length !== DECK_SIZE) issues.push(`needs exactly ${DECK_SIZE} glyphs; has ${keys.length}`);
  if (cost > BUDGET) issues.push(`rarity budget ${cost}/${BUDGET} is too high`);
  if (rares > MAX_RARES) issues.push(`rare limit ${rares}/${MAX_RARES} is too high`);
  return { size: keys.length, cost, rares, issues };
}

// ----------------------------------------------------------------- persistence

function readJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? { ...fallback, ...(JSON.parse(raw) as T) } : fallback;
  } catch {
    return fallback;
  }
}

function writeJson(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* storage can be unavailable in some embeds; settings just won't persist */
  }
}

export type Prefs = {
  master: number;
  music: number;
  sfx: number;
  muted: boolean;
  speed: 1 | 2 | 4;
  reducedMotion: boolean;
  showLog: boolean;
};

const reduce = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
export const prefs: Prefs = readJson<Prefs>("soulscale.prefs", {
  master: 0.8,
  music: 0.6,
  sfx: 0.9,
  muted: false,
  speed: 1,
  reducedMotion: reduce,
  showLog: true,
});
export const savePrefs = () => writeJson("soulscale.prefs", prefs);

export type SavedDeck = { name: string; faction: Faction; cards: string[] };
export function loadDecks(): SavedDeck[] {
  return readJson<{ decks: SavedDeck[] }>("soulscale.decks", { decks: [] }).decks;
}
export function saveDecks(decks: SavedDeck[]): void {
  writeJson("soulscale.decks", { decks });
}

// ------------------------------------------------------------------ change bus

const listeners = new Set<() => void>();
export function onChange(fn: () => void): void {
  listeners.add(fn);
}
export function changed(): void {
  for (const fn of listeners) fn();
}
