# RobCoOS LLM Handoff

Status: current as of 2026-03-19  
Repository: `RobCoUnified`  
Primary native target: `robcos-native`  
Current version in repo manifests: `0.4.4`

This document is for another LLM or automation agent that needs to understand the repo quickly without rediscovering the architecture from scratch.

---

## 1. Executive Summary

RobCoOS is a Fallout-inspired application-layer shell environment written in Rust. It is not a real operating system. It provides two first-class user experiences:

- terminal mode
- desktop mode

The codebase has already begun a serious architecture split. The `crates/` workspace is not decorative. It is the current direction of the project and must be treated as the source of truth for long-term structure.

The current strategic direction is to evolve the native desktop into something closer to an XFCE-style environment:

- lightweight shell/orchestrator
- standalone-capable apps
- clear component boundaries
- low idle CPU
- bounded memory growth
- safer behavior on older or weaker hardware

The shell should increasingly behave like an orchestrator for components and services, not a monolithic owner of every app's state and UI.

---

## 2. Read This First: Important Truths

If you are another LLM, assume these statements are important and intentional:

1. Do not collapse terminal mode and desktop mode into one surface. They are separate first-class experiences.
2. Do not undo the workspace split by moving logic back into `src/native/app.rs`.
3. Do not recreate a second app/service architecture inside `src/native/`; the `crates/` layout is the architecture.
4. Prefer app logic in app crates, shared behavior in service/shared crates, and shell composition only in native UI host code.
5. The desired end state is closer to XFCE + Thunar than to one giant desktop app with many internal panels.
6. Standalone-capable apps are a feature, not a temporary detour.
7. All normal app launches should use proper app UI windows. Embedded in-shell flows are exceptions for pickers and ownership-specific dialogs, not the default model.
8. The current optimization goal is not just micro-speed; it is modularity plus low-overhead desktop behavior.

---

## 3. What The App Is

RobCoOS currently provides:

- a terminal-mode shell
- a desktop-mode shell
- multi-user support
- per-user settings
- built-in apps such as file manager, editor, installer, settings, applications browser, nuke codes viewer, terminal/PTy hosts, and game/utilities
- desktop surface concepts such as windows, taskbar, menus, desktop icons, start/search behavior

It is best thought of as:

- a native Rust desktop shell application
- with a legacy/reference shell still present
- with growing separation between shell, services, and app-specific logic

It is not yet a fully separate-process desktop environment, but that is the direction.

---

## 4. Current Strategic Goals

### 4.1 Product Goal

Turn the native app into a real desktop-environment-style shell with XFCE-like characteristics:

- understandable architecture
- low idle resource usage
- good behavior on lower-end hardware
- apps that can stand on their own
- shell components that can eventually be separated into distinct processes/services

### 4.2 Architecture Goal

Continue splitting the system into clean layers:

- shell/orchestrator
- shared services
- standalone-capable app cores
- optional future per-app processes

### 4.3 Performance Goal

Prioritize:

- low startup cost
- low idle CPU
- bounded caches and RAM growth
- reduced synchronous filesystem and SVG work on the UI thread
- compatibility with weak GPUs and weird driver situations

### 4.4 UX Goal

Keep the retro visual identity, but move toward a more robust desktop model:

- standalone app windows
- stable window hosting behavior
- better file manager polish
- practical, low-friction settings behavior

---

## 5. Current Direction, In Plain Language

The project is moving away from:

- one giant `RobcoNativeApp` that knows everything

and toward:

- a lean native shell
- reusable app crates
- standalone native app binaries
- eventually cleaner process boundaries

The correct next work is generally:

- remove ownership from the shell
- not re-centralize logic
- not add more giant special-case branches to `src/native/app.rs` unless absolutely necessary

---

## 6. Workspace Map

This is the current high-level workspace structure.

### 6.1 Top-Level Roles

| Path | Role |
| --- | --- |
| `Cargo.toml` | workspace manifest and root crate |
| `src/native/` | native shell UI, presenters, adapters, standalone app hosts |
| `src/legacy/` | legacy/reference implementation |
| `crates/shared/` | cross-shell shared config/state/runtime helpers |
| `crates/native-services/` | shared native service layer |
| `crates/native-shell/` | native binary entrypoints |
| `crates/legacy-shell/` | legacy binary entrypoint |
| `crates/native-*-app/` | app/domain crates for individual native apps |
| `scripts/` | profiling and helper scripts |
| `docs/` | roadmap, release checklist, and this handoff |

### 6.2 Workspace Crates

Current crates under `crates/`:

- `legacy-shell`
- `native-about-app`
- `native-connections-app`
- `native-default-apps-app`
- `native-document-browser-app`
- `native-edit-menus-app`
- `native-editor-app`
- `native-file-manager-app`
- `native-installer-app`
- `native-nuke-codes-app`
- `native-programs-app`
- `native-services`
- `native-settings-app`
- `native-shell`
- `native-terminal-app`
- `shared`

---

## 7. Layering Model

### 7.1 `crates/shared`

Purpose:

- shared config
- shared session/user/runtime paths
- common cross-shell data and logic

Important note:

- `crates/shared/src/config.rs` is a central source of truth for runtime paths, settings, user data, and window mode settings.

### 7.2 `crates/native-services`

Purpose:

- shell-independent native services/helpers

Current service modules include:

- desktop connections
- default apps
- documents
- files
- launcher/catalog
- search
- session
- settings
- shortcuts
- status
- desktop surface policy
- users
- shared file manager settings/types

Use this crate for reusable native-domain logic that should not live directly inside the shell UI.

### 7.3 `crates/native-*-app`

Purpose:

- app-specific state, actions, and domain logic

Important examples:

- `native-file-manager-app`
- `native-settings-app`
- `native-editor-app`
- `native-installer-app`
- `native-terminal-app`

These crates are the correct home for app logic. Avoid moving that logic back into the shell.

### 7.4 `src/native/`

Purpose:

- native shell presentation/orchestration
- egui presenters
- window hosting
- adapter glue between shell and app/service crates
- standalone native app wrappers

This directory is still important, but it should increasingly be shell composition and presentation, not business logic.

---

## 8. Current Native Runtime Model

### 8.1 Main Shell Binary

Primary shell binary:

- `crates/native-shell/src/main.rs`
- binary name: `robcos-native`

This is the desktop/terminal shell host.

### 8.2 Standalone Native App Binaries

These standalone binaries currently exist:

- `robcos-file-manager`
- `robcos-settings`
- `robcos-editor`
- `robcos-applications`
- `robcos-nuke-codes`
- `robcos-installer`

They are defined in `crates/native-shell/Cargo.toml`.

### 8.3 Standalone Launch Path

Standalone launches are routed through:

- `src/native/standalone_launcher.rs`

That launcher resolves sibling binaries and propagates session user context via:

- `ROBCOS_NATIVE_STANDALONE_USER`

### 8.4 Current UX Rule For App Launching

Normal app opens should use proper app UI windows.

Embedded in-shell presenters still exist for cases such as:

- pickers
- ownership-specific dialogs
- shell-integrated flows that are not yet fully split

Do not interpret the presence of embedded presenters as a signal to collapse back to shell-owned app hosting.

---

## 9. Shell vs Standalone Status

### 9.1 Apps With Standalone Native Windows Today

These apps now have standalone binary paths and app-specific native window hosts:

- File Manager
- Settings
- Editor
- Applications
- Nuke Codes
- Installer

### 9.2 Still Shell-Centric Or Shell-Owned

These areas are still more shell-owned than desired:

- desktop surface
- taskbar/panel behavior
- Start/search/launcher shell behavior
- terminal-mode shell hosting
- PTY-hosted app/window behavior
- some desktop component/window orchestration
- some embedded presenters and picker flows
- overall `RobcoNativeApp` orchestration in `src/native/app.rs`

### 9.3 Important Nuance

Even after standalone splits, some shell-side adapters and presenters remain. That does not mean the split failed. It means the system is mid-transition.

The guiding question is:

- "Is the shell just hosting/orchestrating?"

not:

- "Does any shell code still exist?"

---

## 10. Major Architectural Decisions Already Made

These decisions matter and should not be accidentally reversed.

### 10.1 The Workspace Split Is Real

The move into `crates/` is the correct direction and already underway. Future work should reinforce it.

### 10.2 Native Shell Should Shrink

`src/native/app.rs` still exists and is still large, but the correct direction is to move logic out of it, not add more app-specific ownership into it.

### 10.3 Desktop Component Metadata Has Been Centralized

The desktop component registry lives in:

- `src/native/desktop_app.rs`

This was done so window metadata and lifecycle routing stop being scattered across multiple giant match blocks.

### 10.4 Shared Desktop Window Chrome Was Extracted

Common desktop host/window behavior has already started being centralized. Continue that pattern rather than re-copying window frame logic per app.

### 10.5 Standalone App Model Is Now Real

The system already supports sibling binaries for multiple apps. Future splits should follow that pattern when it makes sense.

---

## 11. Recent Milestones

The following commits are useful landmarks:

| Commit | Summary |
| --- | --- |
| `fadb976` | split installer into standalone app and reduced installer search freezing |
| `4a3a3e5` | split editor, applications, and nuke codes into standalone apps |
| `3517ba4` | launch file manager and settings as standalone native apps |
| `2414ff3` | add standalone native file manager binary |
| `68adbd4` | extract shared desktop window host helpers |
| `899cb19` | unify desktop component registry and open hooks |
| `2e423a5` | route desktop shell lifecycle through component adapters |
| `1b52d9c` | extract native desktop component registry |
| `a8755e8` | add borderless fullscreen window mode |
| `1dfd3f7` | make window mode live and default to windowed |
| `1326624` | add startup window mode setting |
| `1b03378` | optimize native desktop startup and file manager UI |

These commits tell the story:

- first optimize and clean desktop behavior
- then introduce better window mode handling
- then centralize desktop component metadata
- then move apps onto standalone binary paths

---

## 12. Current Performance / Optimization State

### 12.1 Optimization Direction

The performance target is explicitly XFCE-like:

- lightweight shell
- low idle churn
- less synchronous heavy work on UI paths
- compatibility-oriented defaults

### 12.2 Important Optimizations Already Done

Recent work already completed includes:

- startup profiling harness fixed so desktop startup is measured correctly
- default native window mode changed to `Windowed`
- live persistent `Window Mode` setting added
- separate `Borderless Fullscreen` mode added
- terminal idle repaint throttled instead of repainting every second
- file manager label/path/min-size/tab handling improved
- file manager SVG warmup capped instead of eager whole-folder work
- installer desktop search no longer performs some of the old blocking checks that caused macOS freezes

### 12.3 Last Recorded Desktop Profile

Most recent recorded desktop profile from this workstream:

- `STARTUP_MS=357`
- `AVG_IDLE_CPU=0.00`
- `RSS_MB=155.0`

Treat this as a recent measured reference point, not a permanent guarantee.

### 12.4 Main Remaining Performance Work

Highest-value remaining work:

1. pre-rasterize built-in SVG assets at build time instead of runtime
2. move preview/icon rasterization and similar expensive work off the UI path
3. add bounded caches and LRU behavior for previews/icons
4. debounce or watch-driven filesystem refreshes instead of repeated synchronous scans
5. reduce shell-owned state so the shell process stays lean even when apps are busy
6. continue splitting into separate components/processes where that reduces shell overhead and confusion

---

## 13. Windowing / Desktop Behavior Facts

### 13.1 Current Window Mode Model

The native shell supports:

- `Windowed`
- `Borderless Fullscreen`
- `Fullscreen`

The default is:

- `Windowed`

This is intentional for compatibility on older GPUs and flaky driver situations.

### 13.2 Live Behavior

Window mode changes:

- apply live
- persist across launches

### 13.3 Settings Placement

The window mode control is intended to be an appearance concern, not a general/system metadata field.

### 13.4 Environment Override

Startup window mode can be overridden via:

- `ROBCOS_NATIVE_WINDOW_MODE`

Supported override values include:

- `windowed`
- `safe`
- `borderless`
- `desktop`
- `fullscreen`

---

## 14. Current Product Rules And Preferences

These are not random implementation details; they are part of the current direction.

1. Terminal mode and desktop mode remain separate first-class experiences.
2. Legacy shell remains in the repo as a reference surface until native parity is good enough.
3. The shell should increasingly orchestrate, not own.
4. Custom apps should function as standalone-style apps, similar to XFCE using separate applications such as Thunar.
5. The architecture should become neatly separated and organized to reduce confusion.
6. App launches should use actual app windows where appropriate.
7. Performance work should serve the desktop-environment goal, not just isolated benchmarks.

---

## 15. Current Gaps

### 15.1 Architecture Gaps

- `src/native/app.rs` is still too large and still owns too much orchestration
- panel/start/search/desktop-surface behavior is not yet split into truly independent shell components/services
- terminal hosting and PTY app hosting are still more shell-tied than ideal
- some embedded presenters still exist where standalone boundaries are preferred long term
- there is still one large shell process for many responsibilities

### 15.2 Performance Gaps

- some icon/preview work remains too synchronous
- some directory scanning and refresh work should become more event-driven and bounded
- shell memory usage can improve further by moving more heavy state out of the shell process

### 15.3 Product Gaps

- native desktop still needs ongoing polish
- native terminal parity with the legacy/reference shell is still not perfect
- remaining shell components still need cleaner ownership boundaries

---

## 16. Recommended Next Steps

If continuing the current direction, the best next steps are:

1. continue splitting shell-owned components, especially launcher/panel/start/search/surface concerns
2. make terminal/PTy hosting cleaner and more separable
3. add cleaner shell-to-standalone-app coordination where live reactions are needed
4. keep reducing `src/native/app.rs` by moving ownership behind adapters/services
5. continue XFCE-class performance work:
   - build-time rasterized built-in icons
   - async/bounded preview pipelines
   - watcher/debounced filesystem refresh
   - bounded caches

The guiding heuristic should be:

- "Will this make the shell smaller, clearer, and less hot at idle?"

If yes, it is probably aligned with the current goal.

---

## 17. What Not To Do

Avoid these mistakes:

- do not treat `src/native/app.rs` as the permanent home for every new behavior
- do not move app logic out of app crates back into the shell
- do not ignore the existing `crates/` split and rebuild duplicate logic locally
- do not assume the native shell should remain one forever-monolithic process
- do not regress standalone app launching back to shell-only windows
- do not optimize only for fullscreen modern GPUs; low-end compatibility matters
- do not assume old roadmap text is perfectly current without checking recent commits

---

## 18. Important Files For Future Work

These files are high-value orientation points.

### 18.1 Shell / Orchestration

- `src/native/app.rs`
- `src/native/desktop_app.rs`
- `src/native/standalone_launcher.rs`
- `src/native/mod.rs`

### 18.2 Standalone Native App Hosts

- `src/native/file_manager_standalone.rs`
- `src/native/settings_standalone.rs`
- `src/native/editor_standalone.rs`
- `src/native/applications_standalone.rs`
- `src/native/nuke_codes_standalone.rs`
- `src/native/installer_standalone.rs`

### 18.3 Native Binary Entrypoints

- `crates/native-shell/src/main.rs`
- `crates/native-shell/src/file_manager_main.rs`
- `crates/native-shell/src/settings_main.rs`
- `crates/native-shell/src/editor_main.rs`
- `crates/native-shell/src/applications_main.rs`
- `crates/native-shell/src/nuke_codes_main.rs`
- `crates/native-shell/src/installer_main.rs`

### 18.4 Shared Layers

- `crates/shared/src/config.rs`
- `crates/native-services/src/lib.rs`

### 18.5 App Crates

- `crates/native-file-manager-app/src/lib.rs`
- `crates/native-settings-app/src/lib.rs`
- `crates/native-editor-app/src/lib.rs`
- `crates/native-installer-app/src/lib.rs`
- `crates/native-terminal-app/src/lib.rs`

### 18.6 Existing Docs

- `README.md`
- `docs/NATIVE_ROADMAP.md`
- `docs/RELEASE_CHECKLIST.md`

Note:

- `docs/NATIVE_ROADMAP.md` is still useful for principles, but it predates some of the newer standalone-app and component-registry work.

---

## 19. Useful Commands

### 19.1 Main Native Shell

```bash
cargo run -p robcos-native-shell --bin robcos-native
cargo run --release -p robcos-native-shell --bin robcos-native
```

### 19.2 Standalone Native Apps

```bash
cargo run -p robcos-native-shell --bin robcos-file-manager
cargo run -p robcos-native-shell --bin robcos-settings
cargo run -p robcos-native-shell --bin robcos-editor
cargo run -p robcos-native-shell --bin robcos-applications
cargo run -p robcos-native-shell --bin robcos-nuke-codes
cargo run -p robcos-native-shell --bin robcos-installer
```

### 19.3 Build / Check

```bash
cargo check
cargo build --release -p robcos-native-shell --bin robcos-native
```

### 19.4 Startup / Idle Profiling

```bash
./scripts/profile_native_desktop.sh
```

The profiling script uses a temporary runtime root and autologin desktop startup to collect startup and idle stats.

---

## 20. Runtime / Data Facts

Runtime data is stored under the detected base directory.

Key behavior:

- `ROBCOS_BASE_DIR` overrides runtime location
- macOS app bundles use application support storage
- otherwise data is usually relative to the executable

Important runtime artifacts include:

- `settings.json`
- `users/users.json`
- per-user settings/data
- journal entries

---

## 21. If You Are Continuing The Refactor

Use this mental model:

- `crates/shared` = shared data/config/runtime
- `crates/native-services` = reusable native services
- `crates/native-*-app` = app logic
- `src/native/` = shell presentation/adapters and standalone app hosts
- `crates/native-shell` = binary entrypoints

When deciding where code belongs:

- if it is app logic, it probably belongs in an app crate
- if it is reusable native service logic, it probably belongs in `native-services`
- if it is shell composition or egui host glue, it probably belongs in `src/native/`
- if it makes the shell own more state than before, it is probably the wrong direction

---

## 22. Single-Sentence Goal

Build a retro-themed native desktop shell that behaves more like a lightweight modular desktop environment than a single giant app, with standalone-capable built-in apps and XFCE-like performance/clarity characteristics.
