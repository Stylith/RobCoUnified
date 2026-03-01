# Native Rewrite Plan

## Non-negotiable product rules

1. RobCoOS keeps two first-class modes:
   - Terminal mode
   - Desktop mode
2. The native rewrite must not collapse the product into GUI-only behavior.
3. The current TUI app remains the reference implementation until the native shell reaches parity in the areas we choose to replace.

## Rewrite direction

The rewrite is a new native application layer, not a replacement of the current binary in one step.

- `robcos`: existing terminal-first application
- `robcos-native`: new native shell workbench

## Intended end state

### Terminal mode

- remains available as an actual mode, not a compatibility leftover
- can stay terminal-rendered
- should share auth, settings, files, and app metadata with the native shell

### Desktop mode

- becomes a true native desktop shell
- owns native windows, file manager, settings, and word processor behavior
- uses PTY-backed terminal sessions only for real terminal apps or legacy shells

## Phase 1

- native login
- native shell frame
- native start/app launcher
- native file manager
- native settings
- native word processor

## Phase 2

- shared domain state extraction from terminal UI code
- PTY app windows in native shell
- desktop session persistence parity
- default apps and file associations parity

## Phase 3

- decide how terminal mode and native desktop coexist operationally
- either:
  - native shell launches terminal mode in a PTY window
  - or both binaries remain first-class entry points

## What we are explicitly avoiding

- pretending a terminal emulator wrapper is the final architecture
- deleting terminal mode early
- chasing full feature parity before the native shell is structurally sound

## Current status (Mar 1, 2026)

- Native terminal-style UI now targets old RobCo proportions as the default baseline.
- Terminal-style login, main menu, settings, and hacking screens were rebuilt for closer legacy parity.
- Native PTY integration is active for terminal apps (inside the app, not external windows).
- Performance work landed for PTY repaint throttling and plain-render fast path behavior.
- Session switching is re-enabled in native mode via shared session manager wiring:
  - login now creates/activates a managed session entry
  - logout clears session list and pending switch requests
  - native input now captures switch shortcuts and applies pending session switches
  - switch UX is currently constrained to `Ctrl+Q` then `1..9` only
  - existing native sessions now restore parked runtime state (screen/context) instead of always resetting to main menu
- Native terminal footer now uses legacy-style session indicators (`[1*][2]...`) and no longer shows `username | terminal`.
- Native logout now force-terminates active and parked PTY child processes to prevent background apps (e.g., music players) from continuing after logout.
- Native footer battery indicator now uses shared live battery polling/cache (no hardcoded value).
- Native session leader now supports closing the active session (`Ctrl+Q`, then `W` or `X`):
  - closes the active session
  - reindexes remaining sessions
  - selects the previous session when possible (legacy behavior target)
  - close command is blocked when only one session exists
- Session lifecycle tests expanded (close active/first/last/only cases) in shared session manager.
- Terminal-style parity pass continued:
  - login menu selection bounds are now normalized to selectable rows
  - hacking grid glyph spacing/highlight geometry tightened for more stable theme-consistent rendering
- Theme customization pass landed:
  - `Custom` theme is now part of the same theme chooser list as built-in themes
  - per-user custom RGB is persisted in settings and drives active terminal/native palette when selected
  - RGB sliders/adjusters are wired in both native desktop settings and terminal-style settings flows
  - RGB controls are now shown only when theme selection is `Custom`
- Session runtime parking now preserves full native UI context across session switches:
  - terminal screen/menu state
  - file manager/editor/settings/app window state
  - terminal prompt/flash/session-leader transient state
  - active user identity is synchronized before parked-state restore
  - parity coverage now includes a native app test asserting full context restore on park/restore

## Next parity targets

- Add visible session-switch UX hints (without clutter) and verify behavior inside every terminal-style submenu.
- Preserve deeper per-session runtime state (screen/selection context), not only user/snapshot restoration.
- Continue terminal-first parity hardening before broad desktop-only expansion.
