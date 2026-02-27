# RobCoOS (Rust)

Fallout-style terminal environment built with Rust, ratatui, and crossterm.

RobCoOS is an application-layer shell experience, not a full standalone operating system.

## Version

`0.2.1`

## Highlights

- Multi-user login with per-user settings and per-user app/menu data.
- Up to 9 sessions with hot-switching (menu and PTY-aware switching).
- Terminal-mode main menu: Applications, Documents, Network, Games, Program Installer, Terminal, Desktop Mode, Settings.
- Desktop Mode with:
  - top status bar + spotlight icon
  - taskbar + Start menu
  - draggable/resizable windows
  - minimize/maximize/close controls
  - draggable desktop icons (`My Computer`, `Trash`) with persisted positions
- Built-in app: `Nuke Codes` (visibility toggle in Edit Applications).
- Default Apps system (terminal + desktop settings):
  - separate defaults for Text/Code and Ebook files
  - supports built-in ROBCO Terminal Writer, menu entries, and custom argv JSON
  - per-user settings
  - first-login prompt for new users
- Document routing with explicit error when no app is configured:
  - `Error: No App for filetype`

## Requirements

- Rust stable toolchain (`cargo`, `rustc`)
- `curl` (required preflight dependency)
- Optional external tools:
  - `epy`
  - `vim`
  - other CLI apps you add to menus
- Optional audio backend:
  - `python3`
  - Python module `playsound`

Platform audio fallbacks:

- macOS: `afplay`
- Linux: `aplay` or `paplay`
- Windows: PowerShell `Media.SoundPlayer`

## Build

```bash
cargo build
```

Release build:

```bash
cargo build --release
```

## Run

```bash
cargo run
```

Release run:

```bash
cargo run --release
```

Skip preflight checks:

```bash
cargo run --release -- --no-preflight
```

## First Login

If no users exist, RobCoOS creates default admin:

- Username: `admin`
- Password: `admin`

New users (including the first admin) are prompted once after login to set Default Apps.

## Settings Summary

Terminal Settings includes:

- About
- Theme
- Default Apps
- CLI
- Edit Menus
- User Management (admin)
- Default Open Mode
- Bootup toggle
- Sound toggle

Desktop Settings includes panels for:

- Appearance
- General
- Default Apps
- CLI Display
- CLI Profiles
- Edit Menus
- User Management (admin)
- About

## Data Layout

Runtime data is stored relative to the executable directory.

```text
RobCoOS/
  robcos
  settings.json
  users/
    users.json
    <username>/
      settings.json
      apps.json
      games.json
      networks.json
      documents.json
  journal_entries/
    <username>/
      YYYY-MM-DD.txt
```

## Manual

See `USER_MANUAL.md` for full usage details and control reference.

## Credits and Attribution

- UI framework: [ratatui](https://github.com/ratatui/ratatui)
- Terminal/input backend: [crossterm](https://github.com/crossterm-rs/crossterm)
- PTY support: [portable-pty](https://github.com/wez/wezterm/tree/main/pty)
- Terminal emulation parser: [vt100](https://crates.io/crates/vt100)
- System/time utilities: [sysinfo](https://github.com/GuillaumeGomez/sysinfo), [chrono](https://github.com/chronotope/chrono)

Nuclear launch code data in the built-in Nuke Codes app is fetched from community-maintained sources:

- [NukaCrypt](https://nukacrypt.com/)
- [NukaPD](https://www.nukapd.com/)
- [NukaTrader](https://nukatrader.com/)

This project is an unofficial fan-made work. Fallout and related names, characters, settings, and marks are property of their respective owners (including Bethesda Softworks LLC/ZeniMax Media Inc./Microsoft). This project is not endorsed by or affiliated with those entities.

## AI Assistance Disclaimer

This project was created with the help of AI-assisted development tools.
