# Nucleon Core Handoff

This is the current handoff for `nucleon-core` on branch `WIP`.

## Snapshot

- Repo: `nucleon-core`
- Remote: `origin https://github.com/Stylith/nucleon-core.git`
- Branch: `WIP`
- Product direction: neutral core platform plus shell/theme composition with desktop and terminal treated as distinct shell surfaces
- Current checkpoint: Phase 0 through Phase 8 implemented in code; Phases 7-8 audited and verified
- Current focus: Phase 9 (sound theming) — Phases 9-11 are spec'd in THEME_PHASE_CODEX_SPEC.md
- Phase 11 is the full product rebrand (crate renames, type renames, UI strings, CI/CD, state dir migration)

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

Historical limitation from early Phase 2:
- theme-pack persistence originally existed only as the single legacy field:
  - `Settings.active_theme_pack_id`
- this is now superseded by the later per-surface persistence work described under Phase 4 and Persistence status below

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

### Phase 4

This phase is now implemented in code.

Implemented behavior:

- built-in full-color themes exist:
  - `Nucleon Dark`
  - `Nucleon Light`
- Desktop and Terminal can each choose `Monochrome` or `Full Color` independently
- Full Color is palette-driven through `ColorStyle::FullColor`
- monochrome CRT tint is disabled when the active surface is in `Full Color`
- desktop Tweaks exposes Desktop and Terminal full-color/theme/layout controls
- terminal `Settings -> Appearance` now routes into a terminal-native Tweaks UI instead of the old legacy appearance rows
- per-surface durable settings are now serialized in shared config for:
  - theme-pack ID
  - color style
  - layout profile
- built-in light-theme contrast fixes were landed for:
  - top bar
  - taskbar
  - start menu
  - spotlight
  - file manager selection
  - window chrome
  - Tweaks tabs and controls

### Phase 5

This phase is now implemented in code.

Implemented behavior:

- terminal header branding is theme-controlled through `ThemePack.terminal_branding`
- `ThemePack::classic()` now defaults terminal branding to none
- native terminal screens no longer hardcode the heritage header
- when terminal branding is hidden, the terminal layout reclaims the old 3 header rows
- terminal branding is persisted in shared config through `Settings.terminal_branding`
- terminal theme-pack selection in Tweaks updates the live terminal branding state

### Phase 6 (initial — superseded)

What was implemented from the initial Phase 6:
- "Custom" naming (was "Manual"), read-only state label
- Built-in ThemePacks: Classic, Nucleon
- Flat top-level tabs: [Appearance, Desktop, Display, Terminal]
- Layout Overrides collapsible section

### Phase 6 (REVISED)

This phase is now implemented in code.

Implemented behavior:

- desktop Tweaks top-level tabs are now:
  - `Wallpaper`
  - `Theme`
  - `Effects`
  - `Display`
- `Wallpaper` and `Theme` each contain Desktop/Terminal sub-tabs
- terminal wallpaper path and size mode are now supported through shared config and live runtime rendering
- desktop and terminal full-color flows now support live per-token customization with `Custom` as the detached state label
- desktop Tweaks full-color customization now lists each token with a small current-color box; clicking the box opens a compact Paint-style preset palette with a wider hue range
- customized full-color themes can be exported to `exported_themes/`
- layout overrides now use per-position `PanelType` controls:
  - top panel
  - bottom panel
  - launcher style
  - window header style
- `DockPosition` and `PanelPosition` have been replaced by `PanelType` in the active layout model
- terminal screen separators/title/subtitle rendering is now controlled by `TerminalDecoration`
- terminal retro screens now paint through the wallpaper-aware terminal background helper

### Phase 6 (POLISH)

This polish pass is now implemented in code.

Implemented behavior:

- desktop menu bar and taskbar panel IDs are position-aware, so top/bottom can both be `MenuBar` or both be `Taskbar` without egui ID collisions
- Desktop/Terminal sub-tab buttons in desktop Tweaks now use the same stronger visual weight as the main tabs
- terminal-native Tweaks now shows collapsible top-level sections:
  - `Wallpaper`
  - `Theme`
  - `Effects`
  - `Display`
- only one terminal-native Tweaks section stays expanded at a time
- built-in ThemePack selection is now:
  - `Classic`
  - `Nucleon`
- old persisted pack IDs `nucleon-dark` / `nucleon-light` are normalized to `nucleon` at runtime for compatibility
- the `Nucleon Dark` and `Nucleon Light` choices remain available as full-color theme variants inside the Theme controls

### Post-Phase 6 Fixes

These fixes were applied after Phase 6 POLISH and are now in the codebase:

**Color palette picker bug fix:**
- The inline MS Paint-style color grid in Tweaks was not persisting color changes
- Root cause: settings persistence spawned a background thread, which triggered an IPC `SettingsChanged` self-notification, which reloaded settings from disk and called `apply_surface_theme_state_from_settings()` — unconditionally clearing `desktop_color_overrides` and `tweaks_customize_colors_open`
- Fix: three-part — (1) `SettingsPersisted` handler now calls `refresh_settings_sync_marker()` after file write, (2) IPC `SettingsChanged` handler guards with mtime check to skip self-notifications, (3) `maybe_sync_settings_from_disk` skips while Tweaks is open

**Color palette picker UX:**
- Palette grid and custom color picker now render inline directly below the selected token (not at the bottom of the token list)
- Each token row is clickable; clicking opens a bordered frame with the 48-color grid + `color_edit_button_srgba` fine-tuner

**RetroPalette separation:**
- Added 3 new fields: `window_chrome`, `window_chrome_focused`, `bar_bg`
- Mapped from `ColorToken::WindowChrome`, `WindowChromeFocused`, `StatusBar`
- Window headers now use `window_chrome_focused` instead of `selected_bg`
- Taskbar, menu bar, PTY status bar now use `bar_bg` instead of `selected_bg`
- `selected_bg` is now only used for actual selection highlighting (file manager, start menu hover, etc.)
- Monochrome mode unchanged (all three new fields default to `fg`)
- Built-in Nucleon themes already had distinct values for these tokens — now wired through

## Key Files

### Shared theme types

- [crates/shared/src/theme.rs](/home/stylith/nucleon-core/crates/shared/src/theme.rs)

Notable types:

- `ColorStyle`
- `LayoutProfile`
- `TerminalLayoutProfile`
- `TerminalBranding`
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

Important runtime fields on `NucleonNativeApp`:

- `desktop_active_layout`
- `terminal_active_layout`
- `desktop_active_theme_pack_id`
- `terminal_active_theme_pack_id`
- `desktop_active_color_style`
- `terminal_active_color_style`
- `desktop_color_overrides`
- `terminal_color_overrides`
- `terminal_branding`
- `terminal_decoration`
- `terminal_slot_registry`
- `tweaks_tab`
- `tweaks_wallpaper_surface`
- `tweaks_theme_surface`
- `tweaks_layout_overrides_open`
- `tweaks_customize_colors_open`

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

- desktop Tweaks top-level tabs:
  - `Wallpaper`
  - `Theme`
  - `Effects`
  - `Display`
- desktop `Wallpaper` and `Theme` tabs each contain:
  - `Desktop`
  - `Terminal`
- terminal-native Tweaks sections:
  - `Wallpaper`
  - `Theme`
  - `Effects`
  - `Display`
- terminal-native Tweaks keeps one of those sections expanded at a time
- desktop mode uses the desktop Tweaks window
- terminal mode uses a terminal-native Tweaks screen via `Settings -> Appearance`

Theme-pack rule currently implemented in Tweaks:

- one installed catalog is shared
- Desktop and Terminal choose independently from that catalog
- selecting a theme pack for one surface does not retheme the other surface
- `Custom` appears only as the selected-state label when overrides are active

## Important Runtime Semantics

### Separate theme packs

Desktop and Terminal may point at different theme packs at the same time.

That split now exists both at runtime and in persisted settings.

### Persistence status

This is critical:

- Desktop/Terminal live split is implemented at runtime
- Desktop/Terminal independent theme-pack selection is implemented at runtime
- Desktop/Terminal independent color styles are implemented at runtime
- Desktop/Terminal independent layouts are implemented at runtime
- Desktop/Terminal independent theme-pack selection is also serialized to disk
- Desktop/Terminal independent color styles are also serialized to disk
- Desktop/Terminal independent layouts are also serialized to disk

Current per-surface durable settings fields are:

- `Settings.desktop_theme_pack_id`
- `Settings.terminal_theme_pack_id`
- `Settings.desktop_color_style`
- `Settings.terminal_color_style`
- `Settings.desktop_layout_profile`
- `Settings.terminal_layout_profile`
- `Settings.terminal_branding`
- `Settings.desktop_wallpaper`
- `Settings.terminal_wallpaper`
- `Settings.desktop_wallpaper_size_mode`
- `Settings.terminal_wallpaper_size_mode`

The old global `Settings.active_theme_pack_id` is legacy compatibility state now, not the source of truth for the split runtime model.

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

- `cargo check -p nucleon`
- `cargo check -p nucleon-native-shell`
- `cargo check -p nucleon -p nucleon-native-shell`

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

## Phase 4 Polish Boundary

Use this section to decide whether a rough edge belongs to the current phase or a later one.

These are still Phase 4 work and should be fixed now, not deferred:

- built-in `Nucleon Dark` / `Nucleon Light` readability issues
- CRT interaction bugs with full-color themes
- black or low-contrast selection/highlight states
- Desktop vs Terminal surface leakage bugs
- Tweaks routing problems
- desktop Tweaks vs terminal Tweaks behavior mismatches
- per-surface persistence bugs for theme-pack/color-style/layout state
- manual polish gaps in built-in menus, windows, taskbar, start menu, spotlight, PTY chrome, and Tweaks

These are not Phase 4 and should not be silently mixed into this work:

- sound-theme / sound-pack work
- asset-pack / shell-style system design beyond the current built-in themes
- packaging / external theme repo work
- future external-window / external-process window-manager integration

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
   - settings home
   - terminal Tweaks opened from `Settings -> Appearance`
   - document browser
   - about
   - embedded PTY
   - classic terminal screens render without the old heritage header and content starts 3 rows higher
7. In desktop mode, verify recoloring/layout for:
   - top bar
   - top-bar dropdown menus and submenus
   - taskbar
   - desktop surface
   - start menu
   - spotlight
   - file manager selection states
   - desktop PTY window
8. Verify Desktop and Terminal can point at different theme packs simultaneously
9. Verify per-surface choices survive full app restart
10. Verify wallpaper picker launched from terminal Tweaks returns to terminal Tweaks after selection

If something regresses, inspect these files first:

- [src/native/app/frame_runtime.rs](/home/stylith/nucleon-core/src/native/app/frame_runtime.rs)
- [src/native/app.rs](/home/stylith/nucleon-core/src/native/app.rs)
- [src/native/retro_ui.rs](/home/stylith/nucleon-core/src/native/retro_ui.rs)
- [src/native/app/tweaks_presenter.rs](/home/stylith/nucleon-core/src/native/app/tweaks_presenter.rs)
- [src/native/pty_screen.rs](/home/stylith/nucleon-core/src/native/pty_screen.rs)

## What Is Not Done Yet

- manual GUI validation of Phase 4 across Desktop and Terminal surfaces
- manual GUI validation of Phase 5 terminal branding/header behavior
- manual GUI validation of Phase 6 polish behavior:
  - panel ID collision cases
  - terminal-native Tweaks one-section expansion
  - desktop Tweaks sub-tab sizing/weight
  - built-in ThemePack dropdown showing `Classic | Nucleon`
- sound-theme/sound-pack system
- asset-pack / shell-style phase spec
- `nucleon-core-themes` packaging phase spec
- future external-window / external-process WM integration beyond the current seam types

## Resume Guidance

If resuming on another machine:

1. Read [docs/THEME_PHASE_CODEX_SPEC.md](/home/stylith/nucleon-core/docs/THEME_PHASE_CODEX_SPEC.md) first.
2. Read this handoff second.
3. Start by running the two cargo checks.
4. Then do the manual Desktop vs Terminal surface-independence tests above.
5. Validate the Phase 5 terminal-branding/header behavior and the Phase 6 polish items above.
6. Only after that validation should you continue into Phase 7 or later deferred work.

Do not revert the runtime surface split in order to “simplify” polish work. Any remaining Full Color, Tweaks, or persistence issues should be brought up to the current split runtime model, not the other way around.
