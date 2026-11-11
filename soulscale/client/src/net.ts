// WebSocket client with automatic reconnect. URLs are document-relative so the same
// build works behind Discord's proxy ("/" or "/.proxy/") and on localhost.

import type { ClientMsg, ServerMsg } from "./protocol";

export type Hello = Extract<ClientMsg, { t: "hello" }>;
type Status = "connecting" | "open" | "closed";

export class Net {
  private ws: WebSocket | null = null;
  private backoff = 500;
  private stopped = false;
  status: Status = "connecting";
  private readonly hello: Hello;
  private readonly onMsg: (m: ServerMsg) => void;
  private readonly onStatus: (s: Status) => void;
  private pingTimer: number | undefined;

  constructor(hello: Hello, onMsg: (m: ServerMsg) => void, onStatus: (s: Status) => void) {
    this.hello = hello;
    this.onMsg = onMsg;
    this.onStatus = onStatus;
    this.connect();
  }

  static url(): string {
    const u = new URL("ws", location.href);
    u.protocol = u.protocol === "https:" ? "wss:" : "ws:";
    u.search = "";
    u.hash = "";
    return u.toString();
  }

  private setStatus(s: Status): void {
    this.status = s;
    this.onStatus(s);
  }

  private connect(): void {
    if (this.stopped) return;
    this.setStatus("connecting");
    const ws = new WebSocket(Net.url());
    this.ws = ws;
    ws.onopen = () => {
      this.backoff = 500;
      ws.send(JSON.stringify(this.hello));
      this.setStatus("open");
      clearInterval(this.pingTimer);
      // Keep proxies from idling the socket out.
      let n = 0;
      this.pingTimer = window.setInterval(() => this.send({ t: "ping", n: n++ }), 25_000);
    };
    ws.onmessage = (ev) => {
      try {
        this.onMsg(JSON.parse(String(ev.data)) as ServerMsg);
      } catch (e) {
        console.warn("bad server message", e);
      }
    };
    ws.onclose = () => {
      clearInterval(this.pingTimer);
      this.ws = null;
      this.setStatus("closed");
      if (this.stopped) return;
      setTimeout(() => this.connect(), this.backoff);
      this.backoff = Math.min(this.backoff * 2, 8000);
    };
    ws.onerror = () => ws.close();
  }

  send(m: ClientMsg): boolean {
    if (this.ws?.readyState !== WebSocket.OPEN) return false;
    this.ws.send(JSON.stringify(m));
    return true;
  }

  stop(): void {
    this.stopped = true;
    this.ws?.close();
  }
}
