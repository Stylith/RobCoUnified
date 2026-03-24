# Neutral-Core Refactor Handoff

This file is the concrete continuation handoff for the `WIP` branch.

Use it when resuming this refactor with Codex or another agent on a different machine.

## Current Status

- Repo: `RobCoUnified`
- Working branch: `WIP`
- Base branch for this work: `experimental`
- Refactor goal: move from a product-branded, built-in-app shell toward a neutral core platform with first-party and later third-party addons
- Current strategy: incremental migration, not rewrite

Important constraint summary:

- This is an application-layer shell / desktop environment, not a real operating system.
- Linux is the full-environment target.
- Windows and macOS stay supported under lighter launcher-style profiles.
- Built-in apps must become first-party addons over time.
- Core must depend on capabilities, not app names.
- Shell must not depend on app internals.
- Avoid dynamic plugin loading for now.

## What Was Implemented In This Step

The first safe step was completed: define the neutral contract layer before changing runtime behavior.

New shared platform contracts were added in `crates/shared/src/platform/`:

- `profile.rs`
  - `InstallProfile`
  - `IntegrationLevel`
- `paths.rs`
  - `LogicalRoot`
  - `PlatformPaths`
  - `PlatformPathEnvironment`
  - `ResolvedPlatformPaths`
- `addons.rs`
  - `AddonId`
  - `CapabilityId`
  - `PermissionId`
  - `AddonKind`
  - `AddonScope`
  - `AddonTrust`
  - `AddonEntrypoint`
  - `FileAssociation`
  - `AddonManifest`
  - `AppDefinition`
- `registry.rs`
  - `AddonRegistry`
  - `RegistryError`
- `shell.rs`
  - `LaunchTarget`
  - `LaunchSurface`
  - `ShellAction`
  - `ShellEvent`

The shared module is exported through:

- `crates/shared/src/lib.rs`
- `src/lib.rs`

A product-layer static first-party addon catalog was added in:

- `src/native/addons.rs`

That file currently defines code-backed manifests for:

- settings
- file manager
- editor
- document browser
- terminal
- installer
- programs
- default apps
- connections
- edit menus
- about
- Red Menace
- Zeta Invaders
- nuke codes

Those manifests are not wired into runtime launch yet. That is intentional. This step establishes the migration target without changing behavior.

## Files Added Or Changed

Added:

- `crates/shared/src/platform/mod.rs`
- `crates/shared/src/platform/profile.rs`
- `crates/shared/src/platform/paths.rs`
- `crates/shared/src/platform/addons.rs`
- `crates/shared/src/platform/registry.rs`
- `crates/shared/src/platform/shell.rs`
- `src/native/addons.rs`

Changed:

- `crates/shared/src/lib.rs`
- `src/lib.rs`
- `src/native/mod.rs`

## Verified State

Verified:

- `cargo test -p robcos-shared platform`

That test run passed and covered:

- install-profile path mapping
- registry duplicate detection
- registry capability indexing
- registry file-extension indexing

Partially verified:

- `cargo test -p robcos first_party_registry_exposes_core_capabilities --lib`

That broader root-crate test was started to compile the product layer and GUI dependency graph, but no completed pass/fail result was captured in the previous session. Re-run it on the next machine.

## Why This Was The Correct First Step

The current codebase already has partial module extraction under `src/native/app/`, so the highest-leverage missing piece was not another `app.rs` split in isolation.

The missing foundation was:

- neutral path modeling
- install-profile modeling
- addon/app manifests
- capability-based registry
- shell launch/action/event vocabulary

Without those contracts, later refactors would keep hardcoding product names, app names, executable-relative paths, and special-case built-ins.

## Current Architectural Reality

These are the important real-code observations from the current repo state:

1. `src/native/app.rs` is still the main orchestration pressure point, but it already delegates some logic into `src/native/app/*.rs`.
2. Path handling is still ad hoc and product-specific.
   - `crates/shared/src/config.rs` still owns `base_dir()`, `user_dir()`, `desktop_dir()`, `global_settings_file()`, etc.
   - `src/native/data.rs` still uses direct `dirs::*` and repo/process-relative behavior.
3. Built-in launch behavior is still special-cased by name.
   - `crates/native-services/src/desktop_launcher_service.rs` currently hardcodes built-in games and uses sibling-binary / cargo-manifest fallback logic.
4. Some code still depends on repo-relative or compile-time manifest assumptions.
   - Example: `env!("CARGO_MANIFEST_DIR")` usage in desktop/icon logic and launcher logic.
5. Current branding still leaks into core paths and model assumptions.

This means the next phases should be migration layers, not a sudden redesign.

## Target Architecture Summary

### Product Layering

Core platform should own:

- shell/runtime coordination
- session/user system
- terminal integration
- desktop/window management
- addon manager
- theme engine
- settings/config persistence
- permissions/capability resolution
- path abstraction
- app/addon registry

Optional first-party addons should own:

- settings UI
- file manager
- editor
- document browser
- installer UI
- games and novelty apps
- theme/content packs

Later third-party addons should use the same contract, but with reduced trust and explicit permissions.

### Filesystem Model

The logical model is:

- `core_root`
- `system_addons_root`
- `user_root`
- `user_addons_root`
- `cache_root`
- `runtime_root`

Current `ResolvedPlatformPaths` maps that model to:

- `linux-desktop`
- `windows-launcher`
- `mac-launcher`
- `portable-dev`

Important note:

- The new path layer exists, but the rest of the app still mostly uses the old `config.rs` helpers.
- Migration to `PlatformPaths` still needs to happen in later steps.

### App / Addon Model

Apps are moving toward:

- stable `id`
- manifest-backed metadata
- capability-based lookup
- file associations from metadata
- structured shell actions/events

The current manifests intentionally use static routes instead of dynamic loading.

## Detailed Phased Roadmap

## Phase 0: Contract Layer

Status: done

Objective:

- Create the neutral vocabulary for paths, install profiles, manifests, registry, and shell actions.

Exit criteria:

- Shared contracts exist in `robcos-shared`.
- First-party addon catalog exists in product layer.
- No runtime behavior changed yet.

This phase is complete.

## Phase 1: Add Runtime Adapters Without Breaking Existing Flows

Status: next

Objective:

- Keep current behavior, but stop launching apps through hardcoded names where a capability can be used instead.

Concrete tasks:

1. Add a small runtime adapter in the native shell that can resolve:
   - `LaunchTarget::Addon { addon_id }`
   - `LaunchTarget::Capability { capability }`
2. Back that adapter with `first_party_addon_registry()`.
3. For now, route resolved addon ids into existing open-window/open-screen functions.
4. Start with one app only.
   - Recommended first app: Settings
5. Add focused tests for registry lookup and runtime mapping.

Why Settings first:

- It is central but low-risk.
- It already exists as a distinct app/screen surface.
- It exercises the capability pattern with minimal file-association complexity.

Suggested initial output of this phase:

- one new function or module that maps addon ids/capabilities to the existing runtime open actions
- one launch path converted from app-name special casing to capability lookup

Exit criteria:

- At least one existing app launch uses registry lookup by capability.
- No visible behavior regression.

## Phase 2: Begin Path Migration

Status: after Phase 1 starts landing cleanly

Objective:

- Move filesystem layout decisions behind `PlatformPaths` while keeping compatibility.

Concrete tasks:

1. Add a central resolver that chooses:
   - product slug
   - install profile
   - `ResolvedPlatformPaths`
2. Bridge old helpers in `crates/shared/src/config.rs` to the new path model.
3. Preserve `ROBCOS_BASE_DIR` as a temporary compatibility override.
4. Move the worst ad hoc path sites first:
   - `src/native/data.rs`
   - executable-relative runtime data logic
   - current hardcoded journal/doc directories
5. Keep compatibility shims until all important callers migrate.

Important:

- Do not delete the old helpers all at once.
- First make them derive from the new model, then migrate callers gradually.

Exit criteria:

- The logical path model is authoritative.
- Existing path helpers become wrappers or compatibility shims.

## Phase 3: Convert `src/native/app.rs` Into A Coordinator

Status: after the registry/path substrate is actually in use

Objective:

- Reduce `src/native/app.rs` from owner-of-everything to coordinator.

Target grouped state:

- `ShellState`
- `DesktopState`
- `TerminalState`
- `AppWindowsState`
- `CacheState`
- `RuntimeState`

Target module extraction:

- `desktop_runtime.rs`
- `terminal_runtime.rs`
- `session_runtime.rs`
- window-management modules
- cache modules
- shell dispatch/action module
- helper/path modules

How to do it safely:

1. Identify state clusters already operating together inside `app.rs`.
2. Move behavior with the owning state cluster, not random utility chunks.
3. Keep `RobcoNativeApp` as the root coordinator.
4. Avoid moving app-specific logic into the shell during extraction.

Exit criteria:

- `app.rs` mostly composes modules and coordinates state.
- business rules live outside the god file.

## Phase 4: Convert Built-Ins To First-Party Addons

Status: staged, one app at a time

Objective:

- Stop treating built-in apps as permanently special.

Recommended migration order:

1. Settings
2. File Manager
3. Editor
4. Document Browser
5. Terminal
6. Installer
7. Programs / Default Apps / Connections / Edit Menus / About
8. Games and novelty tools

Per-app migration checklist:

1. Assign stable addon id.
2. Add or refine manifest.
3. Assign capabilities.
4. Assign permissions.
5. Add file associations where relevant.
6. Route launch through registry lookup.
7. Move app-specific state ownership out of shell if possible.
8. Add tests.

Exit criteria:

- Built-ins launch through addon ids/capabilities.
- Core knows capabilities, not app names.

## Phase 5: External Manifests And Scopes

Status: after static registry is proven

Objective:

- Move from code-only manifests to bundled/system/user manifest sources.

Concrete tasks:

1. Load manifests from:
   - bundled scope
   - system scope
   - user scope
2. Add enable/disable/remove state.
3. Preserve static fallback for first-party manifests during transition.
4. Keep actual binary loading simple.
   - manifest + known entrypoint
   - no complex dynamic plugin system yet

Exit criteria:

- runtime can enumerate addons from scoped manifests
- install/enable/disable/remove are possible without changing core contracts

## Phase 6: Third-Party Addons

Status: last

Objective:

- Open the same model to external addons with explicit trust and permissions.

Do not start this early.

The codebase should already have:

- stable manifest schema
- stable path model
- stable registry rules
- stable permission flow

## Proposed Module Tree After Refactor

This is the intended destination, not the current repo state.

```text
crates/shared/src/
  platform/
    mod.rs
    profile.rs
    paths.rs
    addons.rs
    registry.rs
    shell.rs

src/native/
  addons.rs
  runtime/
    coordinator.rs
    desktop_runtime.rs
    terminal_runtime.rs
    session_runtime.rs
    app_windows.rs
    cache_runtime.rs
    shell_dispatch.rs
    paths.rs
  windows/
    settings_window.rs
    file_manager_window.rs
    editor_window.rs
    document_browser_window.rs
```

Important note:

- The exact tree can vary, but the ownership pattern matters more than the folder names.

## What Must Remain In Core

Keep in core:

- app/addon registry
- install-profile selection
- path abstraction
- shell action/event dispatch
- session and user model
- terminal runtime
- desktop/window lifecycle
- addon permission/trust handling
- theme engine framework
- settings/config persistence substrate

Do not keep as hardwired core features forever:

- settings UI
- file manager UI
- editor UI
- document browser UI
- default apps UI
- connections UI
- novelty apps and games
- branded content/theme packs

## Architectural Traps To Avoid

1. Replacing app-name hardcoding with addon-id hardcoding in core

Core should resolve by capability whenever possible. Addon ids are still useful, but not as a new universal special-case key.

2. Moving faster on `app.rs` splitting than on contract adoption

If the runtime still uses old path/app assumptions, a prettier module tree will not solve the real coupling.

3. Migrating paths by search-and-replace

The current repo mixes:

- `config.rs` path helpers
- direct `dirs::*`
- current-exe-relative logic
- `CARGO_MANIFEST_DIR` fallbacks

This needs deliberate adaptation, not blind substitution.

4. Introducing dynamic plugin loading too early

Registry + manifests + packaging can go far without dynamic libraries or complex hot-loading.

5. Letting Linux full-environment assumptions leak into Windows/mac launcher code

All three platforms should share one runtime model, with different install-profile resolution and integration level.

## Recommended Next Task

The next Codex task should be:

1. introduce a small native launch adapter backed by `first_party_addon_registry()`
2. wire Settings launch through `LaunchTarget::Capability("settings-ui")`
3. keep the old runtime behavior underneath
4. add focused tests for the mapping

Do not start with file manager or terminal. Settings is the smallest useful slice.

## Suggested Continuation Steps For The Next Session

When resuming:

1. checkout branch `WIP`
2. re-run:
   - `cargo test -p robcos-shared platform`
   - `cargo test -p robcos first_party_registry_exposes_core_capabilities --lib`
3. inspect current hardcoded settings launch sites in:
   - `src/native/app.rs`
   - `src/native/desktop_app.rs`
   - any menu/start/spotlight launch helpers
4. add a single adapter module for registry-backed launch resolution
5. convert one settings launch path to capability lookup
6. verify no visible behavior change

## Suggested Prompt For The Next Codex Session

Copy this into the next Codex session after pointing it at this repo:

```text
You are continuing the neutral-core/addon refactor on branch WIP.

Read these files first:
- docs/NEUTRAL_CORE_HANDOFF.md
- docs/PROJECT_CONTEXT_FOR_LLM.md
- docs/NATIVE_ROADMAP.md

Important context:
- The first contract step is already implemented in crates/shared/src/platform/ and src/native/addons.rs.
- Do not redesign those contracts unless there is a concrete bug.
- The next task is to add a small runtime adapter that resolves LaunchTarget::Addon / LaunchTarget::Capability through first_party_addon_registry().
- Start with the Settings app only.
- Preserve current behavior and avoid broad rewrites.
- Do not introduce dynamic plugin loading.
- Prefer migration layers over replacing large parts of src/native/app.rs in one pass.

Goal for this session:
- wire one existing Settings launch path through capability-based lookup
- keep shell behavior unchanged
- add focused tests
```

## Reference Files To Inspect Next

High priority:

- `crates/shared/src/platform/mod.rs`
- `crates/shared/src/platform/profile.rs`
- `crates/shared/src/platform/paths.rs`
- `crates/shared/src/platform/addons.rs`
- `crates/shared/src/platform/registry.rs`
- `crates/shared/src/platform/shell.rs`
- `src/native/addons.rs`
- `src/native/app.rs`
- `src/native/desktop_app.rs`

Path migration targets:

- `crates/shared/src/config.rs`
- `src/native/data.rs`
- `crates/native-services/src/desktop_launcher_service.rs`
- `src/native/standalone_launcher.rs`

## Final Notes

The important principle is sequence:

1. contracts
2. adapters
3. path authority
4. runtime decomposition
5. app migration
6. manifest externalization
7. packaging and third-party support

Do not skip ahead. The architecture will be cleaner if the migration order stays disciplined.
