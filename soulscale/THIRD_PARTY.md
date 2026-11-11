# Third-party material

Soulscale's game design, rules text, music and sound effects are Wofl's own work.

## Shipped with the client

| What | Where | Licence |
|---|---|---|
| Twemoji glyph art (the coloured emoji images) | `client/public/glyphs/*.svg` | CC-BY 4.0, © Twitter, Inc and other contributors. Attribution is shown in How to Play → Credits, as the licence requires. https://github.com/jdecked/twemoji |
| Cinzel font | bundled by `@fontsource/cinzel` | SIL Open Font License 1.1 |
| Cormorant Garamond font | bundled by `@fontsource/cormorant-garamond` | SIL Open Font License 1.1 |
| Discord Embedded App SDK | `@discord/embedded-app-sdk` | MIT |

## Used to build and run

Rust crates (serde, tokio, axum, tower-http, reqwest, rustls with ring, tracing, clap, ts-rs and
their dependencies) are MIT and/or Apache-2.0, apart from ring, which also carries ISC-style and
OpenSSL-derived notices. `cargo install cargo-about` can generate a full licence report if you ever
need one for a store listing.

Vite and TypeScript (build only, not shipped) are MIT and Apache-2.0.
