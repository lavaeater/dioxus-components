# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A fork (`lavaeater/dioxus-components`) of `DioxusLabs/components` — a shadcn-style
component library for Dioxus. Current work happens on the `0.8-alpha.3` branch, which
tracks upstream and pins Dioxus `=0.8.0-alpha.3` (and the matching `dioxus-code`,
`dioxus-icons`, `dioxus-sdk-time`, … alpha releases) in the workspace `Cargo.toml`; don't
loosen the pins. Minimum Rust version is 1.92.0. Downstream apps (e.g. `rusty-budgets`)
depend on this branch via git, so a pushed change here reaches them on `cargo update -p
dioxus-primitives`.

The older `0.8` branch (Dioxus alpha.1, with patched copies of git deps in `vendor/`) is
superseded.

Keep this branch easy to rebase on upstream: don't change upstream-owned config (Cargo
profiles, CI workflows) unless the task calls for it.

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

## Building, linting, testing

Run exactly these commands — they match the bacon jobs in `bacon.toml`, so builds share
artifacts instead of rebuilding each other. Keep the same feature set: a different
`-p`/`--features` combination compiles a different dependency graph. (`--all-features` is
for CI only — it also pulls in `dioxus/native`, which is a very heavy build.)

| Purpose | Command |
|---|---|
| Lint (default check) | `cargo clippy --workspace --all-targets --features preview/web` |
| Tests | `cargo test --workspace --features preview/web` |
| Format check (CI enforces) | `cargo fmt --all -- --check` |

- Never add `RUSTFLAGS`, `RUSTC_WRAPPER`, `CARGO_PROFILE_*`, `+toolchain` or `-Z` flags to a
  command. kache (rustc wrapper) and the nightly toolchain come from the user's global
  config. One-off overrides invalidate the cache and force rebuilds of everything.
- No cranelift. Dev builds favour compile speed.
- CI runs clippy with `-D warnings`, so fix every warning in code you touch.
- If cargo blocks on the build-directory lock, the user's bacon is building — just wait.

Other commands:

```sh
# Run the gallery/preview app
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
