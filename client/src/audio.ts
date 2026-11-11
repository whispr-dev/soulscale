// Audio engine (Web Audio).
//
// * Assets: public/audio/index.json (written by tools/convert-audio.*) lists every
//   converted file with its exact sample count; manifest.json maps game events to files.
// * Format: Ogg Opus where supported (gapless loops), AAC .m4a otherwise (Safari / iOS).
// * Music: menu / match / endgame scenes. During a match the Heaven and Hell themes
//   crossfade in as the Soul leans toward either side (equal-power fades).
// * SFX: per-event gain, playback rate and random pitch jitter; several files per event
//   pick at random. Per-card and per-Chamber overrides for the "glyph played" sound.
// * Anything missing falls back to a small built-in synth so the game never goes mute.
// * The Sound Lab edits a copy of the manifest stored in localStorage.

import { prefs } from "./store";
import type { Chamber, Faction } from "./protocol";

export type AssetInfo = { id: string; kind: "music" | "sfx"; duration: number; rate: number; samples: number; channels: number };
export type SfxEntry = { files: string[]; gain?: number; rate?: number; jitter?: number };
export type MusicEntry = { file: string; loop?: boolean; gain?: number };
export type Manifest = {
  version: number;
  music: Record<string, MusicEntry>;
  dynamic: { lean_start: number; lean_full: number; crossfade_seconds: number };
  sfx: Record<string, SfxEntry>;
  cards: Record<string, string>;
  chambers: Record<string, string>;
};
export type MusicScene = "silent" | "menu" | "match" | "endgame";

const OVERRIDE_KEY = "soulscale.audio.manifest";

class AudioEngine {
  ctx: AudioContext | null = null;
  private master!: GainNode;
  private musicBus!: GainNode;
  private sfxBus!: GainNode;
  private ext: "ogg" | "m4a" = "ogg";
  assets = new Map<string, AssetInfo>();
  manifest: Manifest | null = null;
  defaultManifest: Manifest | null = null;
  private buffers = new Map<string, Promise<AudioBuffer | null>>();
  private loops = new Map<string, { src: AudioBufferSourceNode; gain: GainNode; file: string }>();
  private scene: MusicScene = "silent";
  private lean = 0;
  private lastPlayed = new Map<string, number>();

  async init(): Promise<void> {
    const probe = document.createElement("audio");
    this.ext = probe.canPlayType('audio/ogg; codecs="opus"') ? "ogg" : "m4a";
    try {
      const [idx, man] = await Promise.all([
        fetch("audio/index.json").then((r) => (r.ok ? r.json() : { assets: [] })),
        fetch("audio/manifest.json").then((r) => (r.ok ? r.json() : null)),
      ]);
      for (const a of (idx as { assets: AssetInfo[] }).assets) this.assets.set(a.id, a);
      this.defaultManifest = man as Manifest | null;
      this.manifest = this.loadOverride() ?? structuredClone(this.defaultManifest);
    } catch (e) {
      console.warn("audio manifest unavailable; using synth fallbacks", e);
    }
  }

  private loadOverride(): Manifest | null {
    try {
      const raw = localStorage.getItem(OVERRIDE_KEY);
      return raw ? (JSON.parse(raw) as Manifest) : null;
    } catch {
      return null;
    }
  }

  saveOverride(): void {
    try {
      localStorage.setItem(OVERRIDE_KEY, JSON.stringify(this.manifest));
    } catch {
      /* not persisted */
    }
  }

  resetOverride(): void {
    try {
      localStorage.removeItem(OVERRIDE_KEY);
    } catch {
      /* ignore */
    }
    this.manifest = structuredClone(this.defaultManifest);
  }

  /** Must run inside a user gesture the first time (autoplay rules). */
  unlock(): void {
    if (!this.ctx) {
      const Ctx = window.AudioContext ?? (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      this.ctx = new Ctx({ latencyHint: "interactive" });
      const comp = this.ctx.createDynamicsCompressor();
      comp.threshold.value = -10;
      comp.ratio.value = 4;
      this.master = this.ctx.createGain();
      this.musicBus = this.ctx.createGain();
      this.sfxBus = this.ctx.createGain();
      this.musicBus.connect(this.master);
      this.sfxBus.connect(this.master);
      this.master.connect(comp).connect(this.ctx.destination);
      this.applyVolumes();
      this.preloadSfx();
      if (this.scene !== "silent") {
        const s = this.scene;
        this.scene = "silent";
        this.setScene(s);
      }
    }
    if (this.ctx.state === "suspended") void this.ctx.resume();
  }

  get unlocked(): boolean {
    return this.ctx !== null && this.ctx.state === "running";
  }

  applyVolumes(): void {
    if (!this.ctx) return;
    const t = this.ctx.currentTime;
    this.master.gain.setTargetAtTime(prefs.muted ? 0 : prefs.master, t, 0.05);
    this.musicBus.gain.setTargetAtTime(prefs.music, t, 0.05);
    this.sfxBus.gain.setTargetAtTime(prefs.sfx, t, 0.05);
  }

  private preloadSfx(): void {
    for (const a of this.assets.values()) if (a.kind === "sfx") void this.buffer(a.id);
  }

  buffer(id: string): Promise<AudioBuffer | null> {
    let p = this.buffers.get(id);
    if (!p) {
      p = (async () => {
        if (!this.ctx || !this.assets.has(id)) return null;
        try {
          const r = await fetch(`audio/${id}.${this.ext}`);
          if (!r.ok) return null;
          return await this.ctx.decodeAudioData(await r.arrayBuffer());
        } catch (e) {
          console.warn(`could not decode ${id}`, e);
          return null;
        }
      })();
      this.buffers.set(id, p);
    }
    return p;
  }

  // ------------------------------------------------------------------- sfx

  /** Play a game event sound. Never throws; silently no-ops before unlock. */
  play(event: string, opts: { gain?: number; rate?: number; delay?: number } = {}): void {
    if (!this.ctx || prefs.muted) return;
    const e = this.manifest?.sfx[event];
    // Avoid machine-gunning the same event within 40 ms (e.g. four deal sounds at once).
    const now = performance.now();
    if ((this.lastPlayed.get(event) ?? 0) > now - 40 && !opts.delay) return;
    this.lastPlayed.set(event, now);
    const files = e?.files?.filter((f) => this.assets.has(f)) ?? [];
    if (files.length === 0) {
      this.synth(event, opts.delay ?? 0);
      return;
    }
    const file = files[Math.floor(Math.random() * files.length)];
    const jitter = e?.jitter ?? 0;
    const rate = (e?.rate ?? 1) * (opts.rate ?? 1) * (1 + (Math.random() * 2 - 1) * jitter);
    void this.playFile(file, (e?.gain ?? 1) * (opts.gain ?? 1), rate, opts.delay ?? 0);
  }

  async playFile(file: string, gain = 1, rate = 1, delay = 0): Promise<AudioBufferSourceNode | null> {
    if (!this.ctx) return null;
    const buf = await this.buffer(file);
    if (!buf || !this.ctx) return null;
    const src = this.ctx.createBufferSource();
    src.buffer = buf;
    src.playbackRate.value = rate;
    const g = this.ctx.createGain();
    g.gain.value = gain;
    src.connect(g).connect(this.sfxBus);
    src.start(this.ctx.currentTime + delay / 1000);
    return src;
  }

  /** The sound of a glyph being played: card override → side+Chamber → side default. */
  playCard(key: string, faction: Faction, chamber: Chamber, delay = 0): void {
    const m = this.manifest;
    const file = m?.cards[key] ?? m?.chambers[`${faction}:${chamber}`];
    if (file && this.assets.has(file) && this.ctx && !prefs.muted) {
      const base = m?.sfx[`play_${faction}`];
      void this.playFile(file, base?.gain ?? 0.8, 1, delay);
      return;
    }
    this.play(`play_${faction}`, { delay });
  }

  // ----------------------------------------------------------------- music

  setScene(scene: MusicScene): void {
    if (scene === this.scene) return;
    this.scene = scene;
    if (!this.ctx) return;
    const want: Record<string, number> = {};
    const m = this.manifest?.music ?? {};
    const add = (key: string, level: number) => {
      const e = m[key];
      if (e && this.assets.has(e.file)) want[key] = level * (e.gain ?? 0.6);
    };
    if (scene === "menu") add("menu", 1);
    if (scene === "match") {
      add("match", 1);
      add("heaven", 0);
      add("hell", 0);
    }
    if (scene === "endgame") add("endgame", 1);
    const fade = this.manifest?.dynamic.crossfade_seconds ?? 2.5;
    // Fade out loops that are not wanted (or whose file changed).
    for (const [key, l] of this.loops) {
      if (!(key in want) || l.file !== m[key]?.file) this.stopLoop(key, fade);
    }
    for (const [key, level] of Object.entries(want)) {
      if (!this.loops.has(key)) void this.startLoop(key, level, fade);
    }
    if (scene === "match") this.setLean(this.lean);
  }

  private async startLoop(key: string, level: number, fade: number): Promise<void> {
    const e = this.manifest?.music[key];
    if (!e || !this.ctx) return;
    const buf = await this.buffer(e.file);
    if (!buf || !this.ctx || this.loops.has(key)) return;
    const info = this.assets.get(e.file);
    const src = this.ctx.createBufferSource();
    src.buffer = buf;
    src.loop = e.loop !== false;
    if (info && src.loop) {
      // Loop on the exact original length so codec padding never adds a gap.
      src.loopStart = 0;
      src.loopEnd = Math.min(buf.duration, info.samples / info.rate);
    }
    const g = this.ctx.createGain();
    g.gain.value = 0;
    g.gain.setTargetAtTime(level, this.ctx.currentTime, fade / 3);
    src.connect(g).connect(this.musicBus);
    src.start();
    this.loops.set(key, { src, gain: g, file: e.file });
  }

  private stopLoop(key: string, fade: number): void {
    const l = this.loops.get(key);
    if (!l || !this.ctx) return;
    this.loops.delete(key);
    l.gain.gain.setTargetAtTime(0, this.ctx.currentTime, fade / 3);
    l.src.stop(this.ctx.currentTime + fade * 1.5);
  }

  /** Soul position as a fraction of the goal (-1 = Hell, +1 = Heaven). */
  setLean(lean: number): void {
    this.lean = lean;
    if (!this.ctx || this.scene !== "match") return;
    const d = this.manifest?.dynamic ?? { lean_start: 0.25, lean_full: 0.6, crossfade_seconds: 2.5 };
    const x = Math.min(1, Math.max(0, (Math.abs(lean) - d.lean_start) / Math.max(0.01, d.lean_full - d.lean_start)));
    const side = lean >= 0 ? "heaven" : "hell";
    const other = side === "heaven" ? "hell" : "heaven";
    const m = this.manifest?.music ?? {};
    const level = (key: string, f: number) => {
      const l = this.loops.get(key);
      if (l && this.ctx) l.gain.gain.setTargetAtTime(f * (m[key]?.gain ?? 0.5), this.ctx.currentTime, d.crossfade_seconds / 3);
    };
    level("match", Math.cos((x * Math.PI) / 2));
    level(side, Math.sin((x * Math.PI) / 2));
    level(other, 0);
  }

  stopAllMusic(): void {
    for (const k of [...this.loops.keys()]) this.stopLoop(k, 0.3);
    this.scene = "silent";
  }

  // ----------------------------------------------------------------- synth

  /** Fallback voices, used only when an event has no playable file. */
  private synth(event: string, delayMs: number): void {
    const ctx = this.ctx;
    if (!ctx) return;
    const t = ctx.currentTime + delayMs / 1000;
    const out = ctx.createGain();
    out.connect(this.sfxBus);
    const tone = (freq: number, dur: number, type: OscillatorType, vol: number, slide = 1) => {
      const o = ctx.createOscillator();
      const g = ctx.createGain();
      o.type = type;
      o.frequency.setValueAtTime(freq, t);
      o.frequency.exponentialRampToValueAtTime(Math.max(20, freq * slide), t + dur);
      g.gain.setValueAtTime(0.0001, t);
      g.gain.exponentialRampToValueAtTime(vol, t + 0.01);
      g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
      o.connect(g).connect(out);
      o.start(t);
      o.stop(t + dur + 0.05);
    };
    const noise = (dur: number, vol: number, hp: number) => {
      const b = ctx.createBuffer(1, Math.ceil(ctx.sampleRate * dur), ctx.sampleRate);
      const d = b.getChannelData(0);
      for (let i = 0; i < d.length; i++) d[i] = (Math.random() * 2 - 1) * (1 - i / d.length) ** 2;
      const s = ctx.createBufferSource();
      s.buffer = b;
      const f = ctx.createBiquadFilter();
      f.type = "highpass";
      f.frequency.value = hp;
      const g = ctx.createGain();
      g.gain.value = vol;
      s.connect(f).connect(g).connect(out);
      s.start(t);
    };
    if (event.startsWith("ui_") || event === "deal" || event === "lock_in") noise(0.04, 0.3, 2500);
    else if (event.includes("heaven") || event === "cleanse" || event === "victory") {
      tone(660, 0.5, "sine", 0.18);
      tone(990, 0.6, "sine", 0.1);
    } else if (event.includes("hell") || event === "defeat") tone(110, 0.5, "sawtooth", 0.12, 0.6);
    else if (event.startsWith("dice") || event === "wild") for (let i = 0; i < 5; i++) noise(0.02, 0.25, 1800);
    else if (event === "clash" || event === "clash_tie" || event === "trap") tone(90, 0.35, "sine", 0.4, 0.4);
    else if (event === "purgatory") tone(220, 1.2, "triangle", 0.15, 1.5);
    else tone(440, 0.15, "triangle", 0.12);
  }
}

export const audio = new AudioEngine();
