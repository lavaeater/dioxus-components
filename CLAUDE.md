# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A fork (`lavaeater/dioxus-components`) of `DioxusLabs/components` — a shadcn-style
component library for Dioxus. Current work happens on the `0.8` branch, which tracks
Dioxus `0.8.0-alpha.1` (pinned with `=` in the workspace `Cargo.toml`; don't loosen it).
Minimum Rust version is 1.92.0.

The preview app's git dependencies (`dioxus-code`, `dioxus-icons`, `dioxus-i18n`)
upstream still pin `=0.8.0-alpha.0`, so this fork commits patched copies of them
(their `0.8` branches with pins bumped to alpha.1) in `vendor/`, wired in via
`[patch]` sections in the workspace `Cargo.toml`. When bumping Dioxus versions,
remember to bump the pins inside `vendor/` too.

## Workspace layout

Cargo workspace with four members:

- **`primitives/`** (`dioxus-primitives`) — the core crate: unstyled, headless
  components (accordion, dialog, select, slider, …). Each component is a module under
  `primitives/src/`. Rules for components here:
  - Follow the WAI-ARIA Authoring Practices for accessibility.
  - No styling baked in — every element must spread attributes and children from props
    so all styling is controllable by the caller.
  - Shared interaction machinery lives in private modules (`focus.rs`, `pointer.rs`,
    `selection.rs`, `move_interaction.rs`, `portal.rs`, `virtual/`). JS interop assets
    live in `primitives/src/js/` and `ts/`, bundled via `lazy-js-bundle` in `build.rs`.
- **`preview/`** — Dioxus app that is both the component gallery site and the source of
  the styled components that `dx components add <name>` copies into user projects.
  Each `preview/src/components/<name>/` directory contains:
  - `component.rs` + `style.css` — the distributable styled component (this is what
    end users get).
  - `component.json` — distribution metadata (files to exclude, cargo deps, global
    assets like the theme CSS).
  - `docs.md` — rendered to HTML at build time by `preview/build.rs`.
  - `variants/<variant>/mod.rs` — demo variants shown in the gallery.
  The top-level `component.json` lists which component directories are distributable;
  add new components there too.
- **`test-harness/`** — minimal app CI uses to verify `dx components add` output
  actually compiles against the local `primitives/`.
- **`dioxus-attributes/`** — proc macro for dynamic attribute lists, used by primitives.

## Commands

```sh
# Unit tests (primitives crate)
cargo test -p dioxus-primitives

# Run the gallery/preview app (requires dioxus-cli; CI uses dx 0.7.6)
dx serve -p preview --web
dx serve -p preview --desktop

# CSS lint (class names must match ^dx-[a-z0-9-]+$; component CSS may only
# target .dx-* selectors — enforced by preview/.stylelintrc)
cd preview && npm install && npx stylelint "src/**/*.css"

# End-to-end tests (auto-builds and serves preview on :8080 via `dx run --web --release`,
# so the first run is slow)
cd playwright && npm install && npx playwright test

# Single e2e test file / single browser
npx playwright test slider.spec.ts --project=chromium
```

Playwright runs chromium, firefox, and webkit; tests tagged `mobile` are currently
skipped in CI. Tests also run axe accessibility checks (`@axe-core/playwright`).

## Adding a new component

1. If it needs new interaction/accessibility logic, implement an unstyled version in
   `primitives/` (following the rules above) and export it from `primitives/src/lib.rs`.
2. Create the styled version in `preview/src/components/<name>/` (component.rs,
   style.css, component.json, docs.md, variants/) and register it in
   `preview/src/components/mod.rs` and the root `component.json`.
3. Add a Playwright spec in `playwright/`.

## Notes

- `complaints.md` tracks upstream Dioxus limitations discovered while building this
  library (prop ergonomics, portals, attribute spreading) — useful context for why
  some primitives are structured awkwardly.
- CI (`.github/workflows/`) runs cargo check/test, stylelint, Playwright, and an
  "add components" job that exercises `dx components add` for every component against
  the test-harness.
