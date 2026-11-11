// Boot: Discord sign-in (or browser test mode), connect, route between screens.

import "@fontsource/cinzel/500.css";
import "@fontsource/cinzel/700.css";
import "@fontsource/cormorant-garamond/500.css";
import "@fontsource/cormorant-garamond/500-italic.css";
import "./styles.css";

import { app } from "./app";
import { audio } from "./audio";
import { connectDiscord, inDiscord, invite, setPresence, watchLayout } from "./discord";
import { byId, copyText, download, h, mount } from "./dom";
import { fx, toast } from "./fx";
import { glyph } from "./glyph";
import { Net, type Hello } from "./net";
import type { ServerMsg } from "./protocol";
import { onChange, prefs, S, setCatalog, type Screen } from "./store";
import { btn, modal } from "./ui/components";
import { renderDecks } from "./ui/decks";
import { mountGame, onGame, onServerError, unmountGame } from "./ui/game";
import { renderHelp } from "./ui/help";
import { renderLobby } from "./ui/lobby";
import { renderSoundLab } from "./ui/soundlab";

const screen = byId("screen");

function bootScreen(status: string, error = false): void {
  mount(
    screen,
    h(
      "div",
      { class: "boot" },
      glyph("⚖️", { size: "xl" }),
      h("h1", { class: "title small" }, "SOULSCALE"),
      h("p", { class: error ? "boot-status error" : "boot-status" }, status),
      error ? btn("Try again", () => location.reload(), "primary") : null,
    ),
  );
}

function go(s: Screen): void {
  if (S.screen === "game" && s !== "game") unmountGame();
  S.screen = s;
  document.body.dataset.screen = s;
  switch (s) {
    case "lobby":
      renderLobby(screen);
      audio.setScene("menu");
      break;
    case "game":
      mountGame(screen);
      break;
    case "decks":
      renderDecks(screen);
      audio.setScene("menu");
      break;
    case "soundlab":
      renderSoundLab(screen);
      break;
    case "help":
      renderHelp(screen);
      audio.setScene("menu");
      break;
    case "boot":
      break;
  }
  app.presence();
}

function presence(): void {
  const v = S.view;
  const t = S.room?.table;
  if (S.screen === "game" && v && !v.over) {
    const mode = v.mode === "vs_computer" ? `vs Computer (${v.ai_profile})` : v.mode === "duel" ? "Dueling" : "Pass & Play";
    setPresence(app.discord, mode, `Round ${v.round} · Soul ${v.soul > 0 ? "+" : ""}${v.soul} of ±${v.goal}`);
  } else if (t?.state === "open") {
    setPresence(app.discord, "Waiting for a Duel", `${t.host.name} awaits a challenger`);
  } else {
    setPresence(app.discord, "In the lobby", "Between Heaven and Hell");
  }
}

function showExport(m: Extract<ServerMsg, { t: "export" }>): void {
  modal(
    "Playtest export",
    h(
      "div",
      { class: "export" },
      h("p", { class: "muted" }, "The server also saves these to data/replays/ automatically when a match ends."),
      h(
        "div",
        { class: "row wrap" },
        btn("Report .txt", () => download(`${m.basename}.txt`, m.report), "primary"),
        btn("Rounds .csv", () => download(`${m.basename}.rounds.csv`, m.csv, "text/csv")),
        btn("Replay .json", () => download(`${m.basename}.replay.json`, m.replay, "application/json")),
        btn("Copy report", async () => toast((await copyText(m.report)) ? "Report copied." : "Clipboard blocked here; use the download buttons.", "notice")),
      ),
      h("pre", { class: "report" }, m.report),
    ),
    { wide: true },
  );
}

function onMessage(m: ServerMsg): void {
  switch (m.t) {
    case "welcome":
      S.me = m.user;
      S.dev = m.dev;
      setCatalog(m.catalog);
      go("lobby");
      break;
    case "room": {
      const before = S.room?.table;
      S.room = m.room;
      const t = m.room.table;
      const seated = t.state === "live" && t.seats.some((u) => u?.id === S.me?.id);
      const fresh = t.state === "live" && (before?.state !== "live" || before.round > t.round);
      if (seated && fresh && S.screen !== "game") go("game");
      if (S.screen === "lobby") renderLobby(screen);
      if (S.screen === "game" && t.state !== "live" && !S.spectating) go("lobby");
      app.presence();
      break;
    }
    case "game": {
      onGame(m.view, m.events);
      const seated = m.view.viewer.kind !== "spectator";
      if (seated && S.screen !== "game" && !m.view.over && m.events.some((e) => e.t === "round_start" && e.round === 1)) go("game");
      break;
    }
    case "error":
      toast(m.message, "error");
      audio.play("ui_error");
      onServerError();
      break;
    case "notice":
      toast(m.text, "notice", 5000);
      audio.play("notice");
      break;
    case "export":
      showExport(m);
      break;
    case "pong":
      break;
  }
}

function connPill(status: string): void {
  let el = document.getElementById("conn");
  if (status === "open") {
    el?.remove();
    return;
  }
  if (!el) {
    el = h("div", { id: "conn", class: "conn" });
    document.body.appendChild(el);
  }
  el.textContent = status === "connecting" ? "Connecting…" : "Connection lost. Reconnecting…";
}

async function boot(): Promise<void> {
  fx.init();
  document.body.classList.toggle("reduced", prefs.reducedMotion);
  bootScreen("Opening the scales…");
  app.go = go;
  app.presence = presence;
  app.invite = () => invite(app.discord);
  const audioReady = audio.init();
  // Browsers only allow sound after a gesture: unlock on the first one.
  const unlock = () => {
    audio.unlock();
    if (S.screen === "game") audio.setScene(S.view?.over ? "endgame" : "match");
    else if (S.screen !== "boot" && S.screen !== "soundlab") audio.setScene("menu");
  };
  document.addEventListener("pointerdown", unlock, { once: true, capture: true });
  document.addEventListener("keydown", unlock, { once: true, capture: true });

  let hello: Hello;
  if (inDiscord()) {
    S.inDiscord = true;
    try {
      const ctx = await connectDiscord((s) => bootScreen(s));
      app.discord = ctx;
      watchLayout(ctx, (pip) => document.body.classList.toggle("pip", pip));
      hello = { t: "hello", token: ctx.token, dev_name: null, instance: ctx.instanceId };
    } catch (e) {
      bootScreen(`Couldn't sign in with Discord: ${(e as Error).message}`, true);
      return;
    }
  } else {
    const q = new URLSearchParams(location.search);
    const name = q.get("name") ?? `Tester-${Math.floor(Math.random() * 900 + 100)}`;
    hello = { t: "hello", token: null, dev_name: name, instance: q.get("room") ?? "local" };
  }
  await audioReady;
  bootScreen("Connecting to the table…");
  const net = new Net(hello, onMessage, connPill);
  app.send = (m) => net.send(m);
  onChange(() => {
    if (S.screen === "lobby") renderLobby(screen);
  });
}

void boot();
