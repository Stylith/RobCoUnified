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
