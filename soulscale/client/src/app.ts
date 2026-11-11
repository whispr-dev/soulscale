// App-wide handles shared by screens (set up by main.ts), kept here to avoid import cycles.

import type { ClientMsg } from "./protocol";
import type { DiscordCtx } from "./discord";
import type { Screen } from "./store";

export const app = {
  discord: null as DiscordCtx | null,
  send: (_m: ClientMsg): boolean => false,
  go: (_s: Screen): void => undefined,
  invite: (): void => undefined,
  presence: (): void => undefined,
};
