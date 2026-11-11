// Generated from the Rust types by `cargo test -p soulscale-core --features ts`.
// Never edit generated/; change the Rust structs and regenerate instead.
export type { CardDef } from "./generated/CardDef";
export type { Catalog } from "./generated/Catalog";
export type { Chamber } from "./generated/Chamber";
export type { ChamberView } from "./generated/ChamberView";
export type { ClientMsg } from "./generated/ClientMsg";
export type { Condition } from "./generated/Condition";
export type { DeckChoice } from "./generated/DeckChoice";
export type { DeckView } from "./generated/DeckView";
export type { Faction } from "./generated/Faction";
export type { GameEvent } from "./generated/GameEvent";
export type { GameMode } from "./generated/GameMode";
export type { Intent } from "./generated/Intent";
export type { NewGame } from "./generated/NewGame";
export type { PlayInfo } from "./generated/PlayInfo";
export type { Price } from "./generated/Price";
export type { Rarity } from "./generated/Rarity";
export type { RoomInfo } from "./generated/RoomInfo";
export type { RulesVersion } from "./generated/RulesVersion";
export type { Seat } from "./generated/Seat";
export type { SeatStatus } from "./generated/SeatStatus";
export type { SeatView } from "./generated/SeatView";
export type { ServerMsg } from "./generated/ServerMsg";
export type { TableInfo } from "./generated/TableInfo";
export type { UserInfo } from "./generated/UserInfo";
export type { View } from "./generated/View";
export type { AiProfile } from "./generated/AiProfile";

import type { Chamber } from "./generated/Chamber";
import type { Seat } from "./generated/Seat";

export const CHAMBERS: Chamber[] = ["Body", "Mind", "Heart", "Time", "Memory"];
export const CHAMBER_GLYPH: Record<Chamber, string> = {
  Body: "🫀",
  Mind: "🧠",
  Heart: "❤️",
  Time: "🕰️",
  Memory: "🪞",
};
export const SEATS: Seat[] = ["P1", "P2"];
export const seatIdx = (s: Seat): 0 | 1 => (s === "P1" ? 0 : 1);
