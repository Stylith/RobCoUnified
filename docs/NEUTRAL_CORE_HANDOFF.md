# Nucleon Core Handoff

This is the current handoff for `nucleon-core` on branch `WIP`.

## Snapshot

- Repo: `nucleon-core`
- Remote: `origin https://github.com/Stylith/nucleon-core.git`
- Branch: `WIP`
- Product direction: neutral core platform plus shell/theme composition with desktop and terminal treated as distinct shell surfaces
- Current checkpoint: Phase 0 through Phase 3b of the theme system are implemented in code

## Source Of Truth

- Theme implementation spec: [docs/THEME_PHASE_CODEX_SPEC.md](/home/stylith/nucleon-core/docs/THEME_PHASE_CODEX_SPEC.md)
- This handoff is the current implementation/status summary

Do not infer behavior from older handoff text. The shell/theme work is already in progress and the codebase is no longer at the pre-theme baseline.

## Implemented Phases

### Phase 0

- `Appearance` in Settings now opens a dedicated `Tweaks` window/app instead of navigating inside Settings
- standalone Tweaks app exists
- desktop app presenter split was done
- Phase 0c future WM seam types were added:
  - `WindowSource`
  - `ManagedWindow`

Important: Phase 0c is still scaffold only. Actual window storage was not migrated yet.

### Phase 1

- shared theme data model was added in [crates/shared/src/theme.rs](/home/stylith/nucleon-core/crates/shared/src/theme.rs)
- desktop slot registry exists in [src/native/shell_slots/](/home/stylith/nucleon-core/src/native/shell_slots)
- classic desktop shell rendering dispatches through slots instead of direct hardcoded sequencing

### Phase 2

- `ColorStyle` exists and converts into retro palettes
- CRT full-frame monochrome tinting exists in the egui-wgpu CRT shader path
- theme-pack discovery hooks were added on top of the addon pipeline
- `ThemePack::classic()` is the baseline

Known limitation from Phase 2:
- theme-pack persistence in shared settings is still a single legacy field:
  - `Settings.active_theme_pack_id`

### Phase 3

- desktop layout is data-driven through `LayoutProfile`
- built-in desktop layouts exist:
  - `classic`
  - `minimal`
- desktop panel, dock/taskbar, launcher positioning, start menu anchoring, and desktop workspace geometry respect the active desktop layout
- Tweaks contains a desktop layout picker

### Phase 3b

This phase is now implemented.

Implemented behavior:

- Desktop and Terminal have separate live layout state
- Desktop and Terminal have separate live color state
- Desktop and Terminal have separate selected theme-pack IDs at runtime
- Tweaks is surface-first:
  - `Desktop`
  - `Terminal`
- terminal mode has its own slot registry parallel to the desktop slot registry
- steady-state terminal rendering dispatches through terminal slots:
  - `StatusBar`
  - `Screen`
  - `Overlay`
- terminal status bar is parameterized by `TerminalLayoutProfile`
- terminal-mode palettes are surface-scoped and no longer read desktop color state
- terminal PTY rendering is surface-scoped:
  - embedded terminal PTY uses terminal palette
  - desktop PTY windows use desktop palette

## Key Files

### Shared theme types

- [crates/shared/src/theme.rs](/home/stylith/nucleon-core/crates/shared/src/theme.rs)

Notable types:

- `ColorStyle`
- `LayoutProfile`
- `TerminalLayoutProfile`
- `ThemePack`

### Desktop slots

- [src/native/shell_slots/mod.rs](/home/stylith/nucleon-core/src/native/shell_slots/mod.rs)

### Terminal slots

- [src/native/terminal_slots/mod.rs](/home/stylith/nucleon-core/src/native/terminal_slots/mod.rs)
- [src/native/terminal_slots/classic_status_bar.rs](/home/stylith/nucleon-core/src/native/terminal_slots/classic_status_bar.rs)
- [src/native/terminal_slots/classic_screen.rs](/home/stylith/nucleon-core/src/native/terminal_slots/classic_screen.rs)
- [src/native/terminal_slots/classic_overlay.rs](/home/stylith/nucleon-core/src/native/terminal_slots/classic_overlay.rs)

### Runtime ownership and surface split

- [src/native/app.rs](/home/stylith/nucleon-core/src/native/app.rs)

Important runtime fields on `RobcoNativeApp`:

- `desktop_active_layout`
- `terminal_active_layout`
- `desktop_active_theme_pack_id`
- `terminal_active_theme_pack_id`
- `desktop_active_color_style`
- `terminal_active_color_style`
- `terminal_slot_registry`
- `tweaks_surface_tab`
- `desktop_tweaks_tab`
- `terminal_tweaks_tab`

### Frame routing

- [src/native/app/frame_runtime.rs](/home/stylith/nucleon-core/src/native/app/frame_runtime.rs)

Desktop mode:

- desktop shell slots render through the desktop slot registry

Terminal mode:

- steady-state terminal runtime renders through the terminal slot registry
- login / hacking / flash flows remain outside terminal slots, by design

### Surface-scoped palette pipeline

- [src/native/retro_ui.rs](/home/stylith/nucleon-core/src/native/retro_ui.rs)

Important APIs:

- `ShellSurfaceKind`
- `current_palette_for_surface(...)`
- `set_active_color_style(...)`

Compatibility note:

- `current_palette()` still exists as the desktop compatibility path and is still used by desktop-oriented UI code

### Tweaks

- [src/native/app/tweaks_presenter.rs](/home/stylith/nucleon-core/src/native/app/tweaks_presenter.rs)

Current Tweaks behavior:

- top-level surface tabs:
  - `Desktop`
  - `Terminal`
- desktop sub-tabs:
  - `Background`
  - `Display`
  - `Colors`
  - `Icons`
  - `Layout`
- terminal sub-tabs:
  - `Colors`
  - `Layout`
  - `Terminal`

Theme-pack rule currently implemented in Tweaks:

- one installed catalog is shared
- Desktop and Terminal choose independently from that catalog
- selecting a theme pack for one surface does not retheme the other surface

## Important Runtime Semantics

### Separate theme packs

Desktop and Terminal may point at different theme packs at the same time.

That split is currently runtime-only inside `RobcoNativeApp`.

### Persistence status

This is critical:

- Desktop/Terminal live split is implemented at runtime
- Desktop/Terminal independent theme-pack selection is implemented at runtime
- Desktop/Terminal independent color styles are implemented at runtime
- Desktop/Terminal independent layouts are implemented at runtime

But persistence is not fully split yet.

Current persisted settings still only have the legacy global field:

- `Settings.active_theme_pack_id`

So if you continue this work later, do not assume per-surface selection is already serialized to disk. The runtime architecture is split; the durable config model is not fully split yet.

### Terminal slot scope

Terminal slots currently cover only steady-state terminal runtime.

Still outside terminal slots:

- login flow
- hacking flow
- locked screen
- flash screens

That is intentional and matches the spec.

### Sound system

Sound-theme swapping is not part of the implemented work yet.

The user explicitly wants a future swappable sound system, but it was kept out of Phase 3 / 3b. Do not silently fold sound-theme work into the current theming code without a written spec.

## Bug Fixes Already Landed During This Theme Work

- CRT uniform/shader layout mismatch was fixed
- hosted addon texture destruction on close was fixed by deferring release
- desktop hosted addon and desktop PTY windows now coexist as separate windows
- desktop `PtyApp` instance routing bug was fixed so a secondary PTY window does not repaint the primary hosted addon

These fixes are already in this checkpoint. Do not reintroduce the older single-surface PTY/addon behavior.

## Verification Status

Verified at this checkpoint:

- `cargo check -p robcos`
- `cargo check -p robcos-native-shell`

They both pass.

Current warnings are expected and known:

- Phase 0c window-manager scaffold is still unused:
  - `WindowSource`
  - `ManagedWindow`
  - `from_desktop_window(...)`
- older classic-only workspace helpers are now dead code
- `palette_for_settings(...)` and `configure_visuals_for_settings(...)` are now unused after the surface split
- `SlotAction::None` is still unused
- `retro_footer_height()` is now unused because the terminal classic height is baked into `TerminalLayoutProfile::classic()`

No GUI/manual validation was run in this handoff step.

## What Must Be Tested Manually Next

On the next machine, test these exact flows first:

1. Launch `nucleon-native`
2. Open `Tweaks`
3. Switch between `Desktop` and `Terminal`
4. Change Desktop color/layout/theme-pack and confirm terminal mode does not change
5. Change Terminal color/layout/theme-pack and confirm desktop mode does not change
6. In terminal mode, verify recoloring for:
   - main menu
   - login
   - settings
   - document browser
   - about
   - embedded PTY
7. In desktop mode, verify recoloring/layout for:
   - top bar
   - taskbar
   - desktop surface
   - start menu
   - desktop PTY window
8. Verify Desktop and Terminal can point at different theme packs simultaneously

If something regresses, inspect these files first:

- [src/native/app/frame_runtime.rs](/home/stylith/nucleon-core/src/native/app/frame_runtime.rs)
- [src/native/app.rs](/home/stylith/nucleon-core/src/native/app.rs)
- [src/native/retro_ui.rs](/home/stylith/nucleon-core/src/native/retro_ui.rs)
- [src/native/app/tweaks_presenter.rs](/home/stylith/nucleon-core/src/native/app/tweaks_presenter.rs)
- [src/native/pty_screen.rs](/home/stylith/nucleon-core/src/native/pty_screen.rs)

## What Is Not Done Yet

- per-surface durable settings serialization
- per-surface persisted theme-pack IDs in shared config
- per-surface persisted color-style/layout state in shared config
- sound-theme/sound-pack system
- future external-window / external-process WM integration beyond the current seam types
- later spec phases after 3b

## Resume Guidance

If resuming on another machine:

1. Read [docs/THEME_PHASE_CODEX_SPEC.md](/home/stylith/nucleon-core/docs/THEME_PHASE_CODEX_SPEC.md) first.
2. Read this handoff second.
3. Start by running the two cargo checks.
4. Then do the manual Desktop vs Terminal surface-independence tests above.
5. Only after validation should you continue into persistence or later theme phases.

Do not revert the runtime surface split in order to “simplify” persistence work. Persistence should be brought up to the runtime model, not the other way around.
