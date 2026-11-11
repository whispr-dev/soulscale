// Sound Lab: audition every asset, remap game events to sounds, tune gains, preview the
// Heaven/Hell music crossfade, and export the manifest back to client/public/audio.

import { app } from "../app";
import { audio, type SfxEntry } from "../audio";
import { copyText, download, h, mount } from "../dom";
import { toast } from "../fx";
import { btn } from "./components";

const EVENT_HELP: Record<string, string> = {
  ui_hover: "Pointer over a button or card",
  ui_click: "Button press",
  ui_select: "Glyph selected",
  ui_confirm: "Chamber chosen",
  ui_back: "Cancel / remove",
  ui_error: "Invalid move",
  notice: "Table notices",
  round_start: "New round banner",
  deal: "Hand dealt",
  dice_roll: "Intensity die lands",
  dice_reroll: "Reroll",
  lock_in: "Play locked in",
  reveal: "Both plays revealed",
  play_heaven: "Heaven glyph lands (unless a card/Chamber override applies)",
  play_hell: "Hell glyph lands (unless overridden)",
  clash: "Clash with a winner",
  clash_tie: "Tied clash",
  trap: "Trap springs",
  guard: "Guard weakens a play",
  fizzle: "Play fizzles at 0 power",
  sever: "Sever cuts enemy stack",
  release_heaven: "Heaven Release burst",
  release_hell: "Hell Release burst",
  stack_heaven: "Heaven Resonance added",
  stack_hell: "Hell Resonance added",
  place_heaven: "Heaven ongoing/guard takes hold",
  place_hell: "Hell ongoing/trap takes hold",
  cleanse: "Devil effect or condition removed",
  condition_cold: "Cold left in a Chamber",
  condition_thorn: "Thorn left in a Chamber",
  ongoing_heaven: "Heaven ongoing pulse",
  ongoing_hell: "Hell ongoing pulse",
  wild: "Little Miracle / Serpent extra die",
  delayed_heaven: "Delayed/Echo movement toward Heaven",
  delayed_hell: "Delayed/Echo movement toward Hell",
  soul_heaven: "Soul moves up",
  soul_hell: "Soul moves down",
  big_heaven: "Soul jumps 4+ toward Heaven",
  big_hell: "Soul drops 4+ toward Hell",
  purgatory: "Purgatory Event",
  strain: "Strain reaches 3",
  will_gain: "Will bonus scheduled",
  will_loss: "Will penalty scheduled",
  victory: "You win",
  defeat: "You lose",
  ascent: "Heaven wins (atmosphere)",
  descent: "Hell wins (atmosphere)",
};

let lean = 0;

export function renderSoundLab(screen: HTMLElement): void {
  audio.unlock();
  const m = audio.manifest;
  const ids = [...audio.assets.keys()].sort();
  const sfxIds = ids.filter((i) => i.startsWith("sfx/"));
  const musicIds = ids.filter((i) => i.startsWith("music/"));
  const redraw = () => renderSoundLab(screen);
  if (!m) {
    mount(screen, h("div", { class: "page" }, btn("← Lobby", () => app.go("lobby"), "ghost"), h("p", null, "No audio manifest found. Run tools/convert-audio first.")));
    return;
  }
  const save = () => audio.saveOverride();

  const assetSelect = (value: string, options: string[], onPick: (v: string) => void, blank = false) => {
    const s = h("select", null, blank ? h("option", { value: "" }, "— add a sound —") : null, options.map((o) => h("option", { value: o, selected: o === value }, o.replace(/^(sfx|music)\//, ""))));
    s.addEventListener("change", () => onPick(s.value));
    return s;
  };

  const eventRow = (name: string, e: SfxEntry) => {
    const gain = h("input", { type: "range", min: 0, max: 1.5, step: 0.05, value: String(e.gain ?? 1), title: "Gain" });
    gain.addEventListener("change", () => {
      e.gain = parseFloat(gain.value);
      save();
      audio.play(name);
    });
    return h(
      "div",
      { class: "sl-row" },
      btn("▶", () => audio.play(name), "small", "Play this event"),
      h("div", { class: "sl-name" }, h("b", null, name), h("small", null, EVENT_HELP[name] ?? "")),
      h(
        "div",
        { class: "sl-files" },
        e.files.map((f, i) =>
          h(
            "span",
            { class: "file-chip" },
            h("button", { class: "fc-play", type: "button", on: { click: () => void audio.playFile(f, e.gain ?? 1, e.rate ?? 1) } }, `▶ ${f.replace("sfx/", "")}`),
            h(
              "button",
              {
                class: "fc-x",
                type: "button",
                title: "Remove",
                on: {
                  click: () => {
                    e.files.splice(i, 1);
                    save();
                    redraw();
                  },
                },
              },
              "✕",
            ),
          ),
        ),
        assetSelect("", sfxIds, (v) => {
          if (!v) return;
          e.files.push(v);
          save();
          redraw();
          void audio.playFile(v, e.gain ?? 1);
        }, true),
      ),
      gain,
    );
  };

  const leanInput = h("input", { type: "range", min: -1, max: 1, step: 0.01, value: String(lean) });
  leanInput.addEventListener("input", () => {
    lean = parseFloat(leanInput.value);
    audio.setLean(lean);
  });

  mount(
    screen,
    h(
      "div",
      { class: "page soundlab" },
      h(
        "header",
        { class: "page-head" },
        btn("← Lobby", () => {
          audio.setScene("menu");
          app.go("lobby");
        }, "ghost"),
        h("h2", null, "Sound Lab"),
        h(
          "div",
          { class: "row" },
          btn("Export manifest.json", async () => {
            const text = JSON.stringify(m, null, 2) + "\n";
            download("manifest.json", text, "application/json");
            await copyText(text);
            toast("manifest.json downloaded and copied. Replace client/public/audio/manifest.json with it.", "notice", 6000);
          }, "primary"),
          btn("Reset to shipped mapping", () => {
            audio.resetOverride();
            redraw();
          }, "ghost"),
        ),
      ),
      h("p", { class: "muted" }, "Changes save in this browser straight away. Export when you're happy and drop the file into client/public/audio/ so everyone gets it."),
      h(
        "section",
        { class: "sl-music" },
        h("h3", null, "Music"),
        Object.entries(m.music).map(([key, e]) =>
          h(
            "div",
            { class: "sl-row" },
            h("div", { class: "sl-name" }, h("b", null, key), h("small", null, { menu: "Title & lobby", match: "In a match (base layer)", heaven: "Fades in as the Soul leans to Heaven", hell: "Fades in as the Soul leans to Hell", endgame: "Summary screen" }[key] ?? "")),
            assetSelect(e.file, musicIds, (v) => {
              e.file = v;
              save();
            }),
          ),
        ),
        h(
          "div",
          { class: "row wrap" },
          btn("▶ Menu", () => audio.setScene("menu"), "small"),
          btn("▶ Match", () => audio.setScene("match"), "small"),
          btn("▶ Endgame", () => audio.setScene("endgame"), "small"),
          btn("■ Stop", () => audio.stopAllMusic(), "small"),
        ),
        h("label", { class: "setting" }, h("span", null, "Preview Soul lean (Hell ← → Heaven) during Match"), leanInput),
      ),
      h("section", { class: "sl-sfx" }, h("h3", null, "Game events"), Object.entries(m.sfx).map(([k, e]) => eventRow(k, e))),
      h(
        "section",
        { class: "sl-all" },
        h("h3", null, "Every sound"),
        h("div", { class: "all-grid" }, ids.map((id) => btn(`▶ ${id}`, () => void (id.startsWith("music/") ? audio.playFile(id, 0.6) : audio.playFile(id)), "small ghost"))),
      ),
    ),
  );
}
