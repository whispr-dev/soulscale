// "Terms of Service · Privacy Policy" links, shown in the lobby footer and the Credits tab.

import { app } from "../app";
import { openExternal, PRIVACY_URL, TERMS_URL } from "../discord";
import { h } from "../dom";

function link(label: string, url: string): HTMLElement {
  return h(
    "a",
    {
      href: url,
      on: {
        click: (e: Event) => {
          e.preventDefault();
          openExternal(app.discord, url);
        },
      },
    },
    label,
  );
}

export function legalLinks(): HTMLElement {
  return h("nav", { class: "legal-links" }, link("Terms of Service", TERMS_URL), h("span", null, "·"), link("Privacy Policy", PRIVACY_URL));
}
