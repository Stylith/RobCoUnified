# RobCoOS — Rust Edition

A Fallout-style terminal operating system, rewritten in Rust.

## Features

- 🟢 Themed TUI with Green/Amber/Blue/Red/White/Purple/Cyan color schemes
- 🔐 Multi-user login with SHA-256 password hashing and session files
- 📱 Apps / Games / Network launcher menus (JSON-backed, per-user)
- 📄 Document browser (epub, pdf, txt, mobi, azw3) via `epy`
- 📓 Built-in journal / log editor with Ctrl+W save, Ctrl+X cancel
- 💻 Embedded shell terminal (suspends TUI, resumes on exit)
- 🎮 Fallout-style hacking minigame with dud-removal bracket mechanic
- 📦 Package manager integration (brew/apt/dnf/pacman/zypper)
- ⚙️  Per-user settings (theme, sound, bootup animation)
- 🚀 Animated boot sequence (skippable with Space)
- 🔋 Live status bar: date/time + battery %

---

## Building

### Prerequisites

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# System deps (Debian/Ubuntu)
sudo apt install tmux epy vim

# Or on macOS
brew install tmux epy vim
```

### Compile & Run

```bash
cargo build --release
./target/release/robcos
```

Optional sound support (requires `rodio` dependencies):
```bash
cargo build --release --features sound
```

Skip dependency preflight:
```bash
./target/release/robcos --no-preflight
```

---

## Project Structure

```
src/
├── main.rs          Entry point, terminal setup, outer login loop
├── config.rs        Paths, JSON helpers, theme colors, global state (RwLock)
├── ui.rs            Reusable TUI widgets: menu, input, confirm, pager, flash
├── status.rs        Status bar renderer (date/time, battery)
├── auth.rs          Login screen, session tokens, password hashing, user mgmt
├── boot.rs          Animated boot sequence
├── launcher.rs      Suspend/resume TUI, run subprocesses
├── apps.rs          App / Game / Network menus with add/delete
├── docedit.rs       Document category management
├── documents.rs     Document browser, journal editor, logs menu
├── hacking.rs       Fallout hacking minigame (word grid + bracket pairs)
├── settings.rs      Settings menu, theme picker, about/sysinfo screen
├── installer.rs     Package manager search/install/remove (admin-only)
├── shell_terminal.rs Embedded shell (suspends TUI, hands off to $SHELL)
└── checks.rs        Dependency preflight checker
```

---

## Key Improvements Over Python Version

| Area              | Python                       | Rust                                    |
|-------------------|------------------------------|-----------------------------------------|
| Concurrency       | GIL, threading               | `OnceLock<RwLock<T>>` — lock-free reads |
| Error handling    | Exceptions, bare `except:`   | `anyhow::Result<T>` — typed propagation |
| TUI rendering     | curses + `stdscr.getch()`    | `ratatui` + crossterm event stream      |
| Startup time      | ~200 ms                      | <5 ms                                   |
| Memory footprint  | ~30 MB (CPython)             | ~2 MB (static binary w/ strip=true)     |
| Type safety       | Runtime `KeyError`, crashes  | Exhaustive enum matching at compile time|
| Distribution      | Requires Python + pip deps   | Single self-contained binary            |

---

## Default Credentials

On first launch, a default admin account is created:

- **Username:** `admin`
- **Password:** `admin`

Change this immediately via Settings → User Management.

---

## Key Bindings

| Key               | Action                    |
|-------------------|---------------------------|
| ↑ / k             | Move up                   |
| ↓ / j             | Move down                 |
| Enter / Space     | Select                    |
| q / Esc / Tab     | Back / Cancel             |
| Ctrl+W            | Save (journal editor)     |
| Ctrl+X / Esc      | Cancel (journal editor)   |
| Space             | Skip boot animation       |

### Hacking minigame

| Key            | Action                   |
|----------------|--------------------------|
| Arrow keys / WASD | Move cursor           |
| Tab            | Switch column            |
| Enter / Space  | Select word or bracket   |
| q / Esc        | Exit (forfeit)           |
