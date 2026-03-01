# Native Rewrite Parity Audit

This file is the working parity plan for the native rewrite.

The old product has two distinct surfaces:

1. Terminal mode
2. Desktop mode

The native rewrite must preserve that separation.

This is not just a feature checklist. UI parity matters heavily. A screen that does the same thing but looks or behaves differently in a way that breaks the RobCo feel is not parity.

## Ground Rules

- `robcos` remains the reference implementation until native parity is good enough.
- Native terminal mode should emulate the old terminal presentation, not just restyle generic GUI widgets.
- Native desktop mode should remain visually and behaviorally distinct from terminal mode.
- Shared logic should move into reusable core/domain modules. UI code should stop owning business logic.

## Current Native Rewrite Status

What exists now:

- native app binary: `robcos-native`
- shared auth/session core
- native retro renderer foundation
- native login/menu/footer in terminal style
- native terminal-style screens for:
  - main menu
  - applications
  - documents
  - document browser
  - settings
- native desktop shell placeholder
- native file manager window
- native word processor window
- native settings window
- native applications window
- external launcher for old terminal mode
- theme-driven native retro palette
- native interface size setting

What this means:

- the rewrite has a real foundation now
- terminal-side native UI is no longer a generic `egui` mockup
- desktop-mode native UI is still far from old desktop parity

## Audit Method

Reference surfaces reviewed:

- terminal login and main menu in `src/auth.rs` and `src/main.rs`
- terminal settings in `src/settings.rs`
- applications/network/games in `src/apps.rs`
- documents/editor/logs in `src/documents.rs`
- desktop mode in `src/desktop.rs`
- shared terminal renderer in `src/ui.rs`
- current native rewrite in `src/native/`

## Terminal Mode Parity

### UI parity: what is still too different

High priority UI gaps:

- login is closer now, but still not fully identical to the old terminal prompt flow
- password prompt is still a native approximation, not a true old-style terminal prompt interaction
- menu spacing, vertical rhythm, and exact alignment are still approximate in several screens
- current footer/status bar content is still partially synthetic
- selected-row behavior is close, but cursor/selection feel is not yet identical to old `run_menu`
- no native equivalent yet for all old terminal prompt types:
  - `input_prompt`
  - `password_prompt`
  - `confirm`
  - flash-message overlays
  - compact menus
  - pagers

UI principle for terminal mode:

- all native terminal-mode screens should use the retro grid renderer
- avoid normal GUI form controls except where unavoidable during transition
- prompts should feel like old terminal prompts, not dialogs dropped on top of a retro screen

### Functional parity: missing or incomplete

Missing terminal-mode surfaces in native rewrite:

- `Network` menu
- `Games` menu
- `Program Installer`
- `About`
- `Default Apps`
- `Edit Menus`
- `Connections`
- `User Management`
- hacking login/auth flow
- embedded terminal app
- logs flow
- full text-editor terminal feature parity in native terminal-style renderer

Partially present but not yet parity:

- Applications:
  - built-in word processor path exists
  - external app launching is still placeholder messaging
- Documents:
  - word processor documents exist
  - old logs/documents split is not represented in native terminal mode yet
- Settings:
  - only a subset exists natively
  - structure is incomplete relative to old settings

### Terminal Mode Priority Order

P0:

- native prompt framework for:
  - password
  - text input
  - confirm
  - flash/status overlays
- hacking auth support
- exact old terminal menu behavior and spacing pass

P1:

- terminal `About`
- terminal `Default Apps`
- terminal `Edit Menus`
- terminal `User Management`

P2:

- terminal `Network`
- terminal `Games`
- terminal `Program Installer`
- terminal `Connections`

## Desktop Mode Parity

### UI parity: what is still too different

This is the biggest gap in the rewrite.

Current native desktop is still a placeholder shell, not a parity implementation.

Major UI differences from old desktop:

- no old-style top status bar/taskbar framing
- no old-style start menu tree/leaf structure
- no old-style desktop wallpaper handling
- no desktop icon field matching old layout/style
- no old-style window chrome
- no old minimize/maximize/close behavior
- no old window focus styling
- no old task switching/taskbar buttons
- no old menu-bar behavior
- no old hub window structure
- no old mouse semantics and hit-testing behavior
- no old multi-window density or stacking behavior

Desktop UI principle:

- native desktop should feel like the old RobCo desktop first, and like a generic GUI toolkit second
- do not replace old desktop with a sparse set of floating utility windows and call it done

### Functional parity: missing or incomplete

Missing desktop-mode surfaces:

- start menu structure
- taskbar/task switching
- start submenu logic
- desktop icons and icon styles
- wallpaper and wallpaper sizing modes
- old window manager behavior
- PTY-backed app windows
- native equivalent of old desktop hub system
- logs hubs
- installer hubs
- connections hubs
- edit-menu hubs
- user-management hubs
- spotlight/search overlay
- help popup patterns
- recent session/window restoration at old desktop fidelity

Partially present but not yet parity:

- file manager:
  - basic native version exists
  - old desktop file manager is much richer
  - missing tree, tabs, search modes, save-as integration parity, recent state, multi-window depth
- text editor:
  - native editor exists
  - not yet at old desktop feature depth or chrome style
- settings:
  - native settings window exists
  - not at old desktop settings structure/parity

### Desktop Mode Priority Order

P0:

- define native desktop shell layout model:
  - top bar
  - desktop area
  - taskbar
  - start menu
  - window chrome
- match old visual framing before adding many more desktop features

P1:

- old-style window manager behavior
- start menu tree/leaf system
- taskbar and task switching
- desktop icons and wallpaper

P2:

- rebuild old desktop hubs:
  - applications
  - documents/logs
  - network/connections
  - games
  - installer
  - settings
  - user management

P3:

- polish:
  - hover behavior
  - drag/resize feel
  - taskbar indicators
  - start/search/help overlays

## Shared Feature Gaps

These are not terminal-only or desktop-only.

Still missing or incomplete in native rewrite:

- preflight and boot flow parity
- sound behavior parity
- session switching parity
- suspended PTY app/session model parity
- native equivalent of old launcher behavior
- full logs support
- full default-apps/file-association flow
- full connections capability behavior
- more complete persistence of shell state

## UI-Specific Acceptance Criteria

Native terminal mode is acceptable only when:

- screens use the retro grid renderer consistently
- prompts do not feel like ordinary GUI forms
- spacing, selection, and footer behavior match the old product closely
- theme changes affect terminal-mode native screens consistently

Native desktop mode is acceptable only when:

- it has a recognizable RobCo desktop shell
- window chrome, taskbar, and start menu behave like the old desktop
- it does not feel like a generic GUI app with a few RobCo colors applied

## Recommended Implementation Order

1. Finish terminal-mode UI primitives
- prompt renderer
- confirm/input/password/flash patterns
- exact spacing/alignment pass

2. Complete terminal-mode settings/admin surfaces
- about
- default apps
- edit menus
- user management

3. Restore missing terminal product features
- hacking auth
- connections
- installer
- network
- games

4. Build real native desktop shell frame
- top bar
- taskbar
- start menu
- desktop area
- window chrome

5. Port desktop hubs in descending value order
- settings
- file manager
- word processor
- applications
- documents/logs
- installer
- connections
- user management

6. Add system-level parity
- sessions
- PTY windows
- restore state
- help/search overlays

## Recommended Near-Term Checkpoints

Checkpoint A:

- native terminal mode has all old terminal settings/admin/document flows
- native prompts are all retro-grid based

Checkpoint B:

- native desktop shell visually matches old desktop frame
- start/taskbar/window chrome are in place

Checkpoint C:

- core desktop hubs are ported
- old TUI desktop no longer needs to be the daily-driver desktop path

## Do Not Lose Track Of

- UI parity is not optional
- terminal mode and desktop mode must remain distinct products inside the same app
- every new native screen should answer:
  - is this terminal-mode UI or desktop-mode UI?
  - does it match the old surface’s layout language?
  - are we reusing shared logic instead of duplicating behavior again?

## Audit Update (2026-03-01, terminal-first pass)

Scope completed:

1. Terminal visual parity sweep across current native terminal screens.
2. Shared renderer/layout fixes only (no app-specific hacks).
3. Session-state deepening for newly added terminal-native screen state.

Findings from this pass:

- Nested PTY launch for native `Nuke Codes` caused double footer/status bars and mixed renderer/color/size behavior.
- Hacking/menu highlight vertical geometry still had subtle divergence because hacking used custom glyph-band math instead of shared retro renderer text-band geometry.
- Session parity coverage needed explicit validation for terminal-native `Nuke Codes` screen state.

Changes applied:

- Replaced nested PTY `Nuke Codes` path with a native terminal-style screen (`TerminalScreen::NukeCodes`) used from both terminal and desktop application launch paths.
- Introduced shared `RetroScreen::text_band_rect(...)` and switched selectable-row paint to that shared text-band geometry.
- Switched hacking glyph highlight paint to the same shared text-band geometry to keep highlight height/position consistent with terminal menus.
- Expanded session tests to include `Nuke Codes` submenu in sweep coverage and explicit `Nuke Codes` state restore validation across session switches.

Current terminal parity status after this pass:

- Terminal chrome and highlight behavior are more consistent across menus and hacking.
- `Nuke Codes` now matches native terminal rendering model (single footer, single palette/scale domain).
- No remaining native `pending rewrite` placeholders are present.

Next desktop transition gate:

- Terminal parity baseline is stable enough to begin desktop-focused work, while continuing to treat terminal renderer/layout regressions as blockers.
