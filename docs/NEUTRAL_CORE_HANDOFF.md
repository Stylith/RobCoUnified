# Nucleon Core Handoff

This is the current baseline handoff for `nucleon-core` on branch `WIP`.

## Snapshot

- Repo: `nucleon-core`
- External addons repo: `nucleon-core-addons`
- Branch: `WIP`
- Product direction: neutral core platform with external first-party addons now, broader addon model later
- Current state: stable cleanup checkpoint before the upcoming shell/theme composition phase

## What Changed

The codebase was pushed through the neutral-core migration far enough to leave the active core in a clean, addon-agnostic state.

Completed work, at a high level:

- runtime launch and capability routing were moved onto shared launch-target and registry seams
- path/storage ownership was centralized under named runtime/state layouts
- the old `app.rs` god-file shape was broken down into smaller runtime modules
- optional first-party addons were externalized into `nucleon-core-addons`
- core now uses a generic addon pipeline:
  - manifest/index discovery
  - install/update/remove
  - `.ndpkg` package support
  - WASM addon hosting inside terminal and desktop surfaces
- repo/runtime naming was moved onto the `nucleon` / `nucleon-core` direction:
  - `NUCLEON_*` env vars are canonical
  - default storage/runtime roots now live under `nucleon`
  - release binary is `nucleon-native`
  - compatibility fallbacks for older `robcos` names still exist where needed
- the remaining hardcoded optional-addon references were removed from core, including the last legacy-shell injection paths

## Current Architecture Baseline

Core is responsible for generic platform behavior only:

- addon discovery and registry
- addon install/update/remove flows
- package extraction and compatibility handling
- generic WASM addon runtime
- terminal and desktop shell integration surfaces
- capability and manifest driven launch resolution
- shared path/runtime/state management

Core should not own or mention specific optional addons anymore.

Addon-specific source, assets, and build ownership belong outside this repo in `nucleon-core-addons`.

## Verification Status

Verified at this checkpoint:

- `cargo check -p robcos`
- `cargo check -p robcos-legacy-shell`
- `cargo test -p robcos-hosted-addon-contract`

The working tree should be clean at the handoff point.

## Rules For The Next Phase

Do not reintroduce addon-specific branches into core.

When working on the next phase:

- keep shell composition generic
- separate layout/profile, component behavior, skin/presentation, and capability/schema
- do not flatten those concerns into a single “theme” object
- keep desktop mode and terminal mode as distinct shell surfaces with shared semantics underneath
- preserve the generic addon host/runtime and installer infrastructure while changing shell composition around it

## What Comes Next

The next major phase is the shell/theme composition project.

That phase has not started in this handoff. Expect a new instruction set from the user for it later.

Use this repo state as the clean slate before that work begins.
