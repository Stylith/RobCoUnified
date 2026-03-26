# RobCoOS User Manual

## Table of Contents

1. [Overview](#1-overview)
2. [Starting Up](#2-starting-up)
3. [Login and Users](#3-login-and-users)
4. [Global Controls](#4-global-controls)
5. [Sessions](#5-sessions)
6. [Terminal Mode](#6-terminal-mode)
7. [Desktop Mode](#7-desktop-mode)
8. [Logs and Journal](#8-logs-and-journal)
9. [Hacking Minigame](#9-hacking-minigame)
10. [Nuke Codes](#10-nuke-codes)
11. [Settings Reference](#11-settings-reference)
12. [Troubleshooting](#12-troubleshooting)

---

## 1. Overview

Welcome to RobCoOS.

RobCoOS provides a retro-styled terminal shell and a native desktop interface on top of your existing operating system. It supports multi-user login, concurrent sessions, PTY-backed terminal access, configurable apps and documents, and a windowed desktop mode with menus, taskbar, and file management.

---

## 2. Starting Up

### Build from Source

```bash
# Development
cargo run -p robcos-native-shell --bin nucleon-native

# Optimized release build
cargo build --release -p robcos-native-shell --bin nucleon-native
cargo run --release -p robcos-native-shell --bin nucleon-native
```

### Startup Flags

| Flag | Effect |
|---|---|
| *(none)* | Normal startup with preflight checks |
| `--no-preflight` | Skip startup checks |

### Validate a Release Build

```bash
make release-check
# or directly:
./scripts/release-check.sh
```

### Required and Optional Dependencies

| Dependency | Status |
|---|---|
| `curl` | Required |
| `epy` | Optional document reader |
| `vim` | Optional external editor |
| `python3` / `playsound` | Optional enhanced audio |
| `blueutil` | Optional macOS Bluetooth support |

---

## 3. Login and Users

### Default Admin Account

| Field | Value |
|---|---|
| Username | `admin` |
| Password | `admin` |

Change these credentials before regular use.

### Authentication Modes

| Mode | Behavior |
|---|---|
| Password | Standard login |
| No Password | Immediate access |
| Hacking Minigame | Login through the hacking screen |

### First-Login Default Apps Prompt

New users are prompted once after login to configure their default applications.

---

## 4. Global Controls

### Navigation

| Key | Action |
|---|---|
| `Up` / `k` | Move up |
| `Down` / `j` | Move down |
| `Enter` or `Space` | Confirm selection |
| `q`, `Esc`, or `Tab` | Exit or cancel |

### Text Input

| Key | Action |
|---|---|
| Type normally | Enter characters |
| `Backspace` | Delete character |
| `Enter` | Confirm input |
| `Esc` | Cancel |

---

## 5. Sessions

RobCoOS supports up to 9 concurrent sessions.

### Session Switching

- `Alt` / `Option` + `1`-`9`
- `Ctrl` + `1`-`9`
- `F1`-`F9`

### Session Switching Inside a PTY

| Sequence | Action |
|---|---|
| `Ctrl+Q`, then `1`-`9` | Switch session |
| `Ctrl+Q`, then `N` or `Tab` | New or next session |
| `~~`, then `1`-`9` | Emergency switch |

---

## 6. Terminal Mode

Terminal mode is the menu-driven shell experience.

### Main Menu

| # | Entry | Description |
|---|---|---|
| 1 | Applications | Launch configured apps |
| 2 | Documents | Browse stored files |
| 3 | Network | Launch configured network entries |
| 4 | Games | Launch games and entertainment entries |
| 5 | Program Installer | Admin-only package management |
| 6 | Terminal | Open the PTY shell |
| 7 | Desktop Mode | Enter the windowed desktop |
| 8 | Settings | Open system settings |
| 9 | Logout | End the current session |

### Applications

Contains configured entries and selected built-in modules.

### Documents

Supported document formats include `.pdf`, `.epub`, `.txt`, `.mobi`, and `.azw3`.

### Network and Games

Both menus launch configured command entries.

### Program Installer

The installer provides administrative package management and desktop install flows.

### Terminal (PTY)

Exit the PTY with `exit` or `Ctrl+D`.

---

## 7. Desktop Mode

Desktop mode provides a structured workspace for multitasking.

### Desktop Surface

| Element | Description |
|---|---|
| Status bar | System time and indicators |
| Taskbar | Active windows |
| Desktop icons | Shortcuts and built-ins |

### Start Menu

Provides quick access to apps, documents, and shell actions.

### Window Controls

Windows can be moved, resized, minimized, maximized, and closed.

### Top Menu Bar

The menu bar is contextual to the focused window or app.

### Spotlight Search

Searches files, documents, and applications.

### File Manager

The native file manager supports tree, list, and grid views, multi-selection, drag and drop, picker flows, and drive browsing.

---

## 8. Logs and Journal

Daily journal entries are stored per user.

---

## 9. Hacking Minigame

The hacking screen is an alternative login mode used for selected accounts.

---

## 10. Nuke Codes

Displays externally sourced launch-code data inside RobCoOS.

---

## 11. Settings Reference

Settings cover appearance, sessions, desktop behavior, default apps, applications, games, network entries, documents, connections, and user management.

---

## 12. Troubleshooting

### Session Switching Inside a PTY Does Not Trigger

Use the `Ctrl+Q` escape sequences or the `~~` fallback.

### App Command from a Menu Fails

Check the configured command, your PATH, and any required dependencies.

### Documents Do Not Open as Expected

Review default app settings and file associations.

### Audio Issues

Check optional dependencies or the platform fallback players.

### Connections Menu Is Missing or Reduced

Some connection features depend on platform-specific tools.

---

This project was created with the help of AI-assisted development tools.
