// Discord Embedded App SDK bridge.
//
// Inside Discord the page URL carries `frame_id`; we then run the official flow:
//   ready() → authorize (code) → POST api/token (server swaps code for a token using the
//   client secret) → authenticate(token). The same token goes to our WebSocket `hello`,
//   where the server verifies it with Discord before trusting the user's identity.
// Outside Discord (a normal browser tab) we return null and main.ts uses dev logins.

import { DiscordSDK, Events } from "@discord/embedded-app-sdk";

export type DiscordCtx = {
  sdk: DiscordSDK;
  token: string;
  instanceId: string;
  userId: string;
};

export const inDiscord = (): boolean => new URLSearchParams(location.search).has("frame_id");

type PublicConfig = { client_id: string | null; dev_login: boolean; rules_version: string };

export async function publicConfig(): Promise<PublicConfig> {
  const r = await fetch("api/config");
  if (!r.ok) throw new Error(`server config unavailable (${r.status})`);
  return (await r.json()) as PublicConfig;
}

export async function connectDiscord(onStep: (s: string) => void): Promise<DiscordCtx> {
  const cfg = await publicConfig();
  if (!cfg.client_id) throw new Error("The server has no DISCORD_CLIENT_ID configured.");
  const sdk = new DiscordSDK(cfg.client_id, { disableConsoleLogOverride: false });
  onStep("Handshaking with Discord…");
  await sdk.ready();
  onStep("Asking Discord who you are…");
  const { code } = await sdk.commands.authorize({
    client_id: cfg.client_id,
    response_type: "code",
    state: "",
    prompt: "none",
    scope: ["identify", "applications.commands"],
  });
  const r = await fetch("api/token", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ code }),
  });
  if (!r.ok) throw new Error(`Sign-in failed at the token exchange (${r.status}).`);
  const { access_token } = (await r.json()) as { access_token: string };
  const auth = await sdk.commands.authenticate({ access_token });
  if (!auth) throw new Error("Discord rejected the sign-in.");
  return { sdk, token: access_token, instanceId: sdk.instanceId, userId: auth.user.id };
}

/** Rich presence line under the user's name in Discord. Failures are harmless. */
export function setPresence(ctx: DiscordCtx | null, details: string, state: string): void {
  if (!ctx) return;
  ctx.sdk.commands.setActivity({ activity: { type: 0, details, state } }).catch(() => undefined);
}

export function invite(ctx: DiscordCtx | null): void {
  ctx?.sdk.commands.openInviteDialog().catch(() => undefined);
}

/** Picture-in-picture (small) layout: we switch to a compact view. */
export function watchLayout(ctx: DiscordCtx, onPip: (pip: boolean) => void): void {
  ctx.sdk
    .subscribe(Events.ACTIVITY_LAYOUT_MODE_UPDATE, (e: { layout_mode: number }) => onPip(e.layout_mode === 1))
    .catch(() => undefined);
}

/** The public site that hosts the legal pages. */
export const SITE = "https://soulscale.store";
export const TERMS_URL = `${SITE}/tos/`;
export const PRIVACY_URL = `${SITE}/privacy/`;

/** Open a web page outside the game. Inside Discord a plain link would load inside the
 *  Activity frame (and be blocked), so it has to go through the SDK, which asks first. */
export function openExternal(ctx: DiscordCtx | null, url: string): void {
  if (ctx) ctx.sdk.commands.openExternalLink({ url }).catch(() => undefined);
  else window.open(url, "_blank", "noopener,noreferrer");
}
