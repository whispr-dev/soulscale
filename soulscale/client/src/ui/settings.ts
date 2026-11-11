// Settings: volumes, animation speed, reduced motion.

import { audio } from "../audio";
import { h } from "../dom";
import { prefs, savePrefs } from "../store";
import { btn, modal } from "./components";

function slider(label: string, get: () => number, set: (v: number) => void): HTMLElement {
  const input = h("input", { type: "range", min: 0, max: 1, step: 0.01, value: String(get()) });
  input.addEventListener("input", () => {
    set(parseFloat(input.value));
    savePrefs();
    audio.applyVolumes();
  });
  input.addEventListener("change", () => audio.play("ui_click"));
  return h("label", { class: "setting" }, h("span", null, label), input);
}

function toggle(label: string, get: () => boolean, set: (v: boolean) => void): HTMLElement {
  const input = h("input", { type: "checkbox", checked: get() });
  input.addEventListener("change", () => {
    set(input.checked);
    savePrefs();
    audio.applyVolumes();
    audio.play("ui_click");
  });
  return h("label", { class: "setting check" }, input, h("span", null, label));
}

export function openSettings(): void {
  audio.unlock();
  const speed = h(
    "div",
    { class: "seg" },
    ([1, 2, 4] as const).map((n) =>
      btn(`${n}×`, () => {
        prefs.speed = n;
        savePrefs();
        for (const b of speed.querySelectorAll("button")) b.classList.toggle("on", b.textContent === `${n}×`);
      }, prefs.speed === n ? "on" : ""),
    ),
  );
  modal(
    "Settings",
    h(
      "div",
      { class: "settings" },
      slider("Master volume", () => prefs.master, (v) => (prefs.master = v)),
      slider("Music", () => prefs.music, (v) => (prefs.music = v)),
      slider("Sound effects", () => prefs.sfx, (v) => (prefs.sfx = v)),
      toggle("Mute everything", () => prefs.muted, (v) => (prefs.muted = v)),
      h("div", { class: "setting" }, h("span", null, "Animation speed"), speed),
      toggle("Reduced motion (fewer particles, no shake or tilt)", () => prefs.reducedMotion, (v) => {
        prefs.reducedMotion = v;
        document.body.classList.toggle("reduced", v);
      }),
      toggle("Show the Chronicle (game log)", () => prefs.showLog, (v) => (prefs.showLog = v)),
    ),
  );
}
