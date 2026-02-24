# RobCoOS (Rust)

Fallout-style terminal OS built with Rust, `ratatui`, and `crossterm`.

## Version

`0.1.0`

## Features

- Multi-user login with per-user settings/data.
- Session switching across menu screens and PTY apps.
- Embedded PTY launcher for CLI applications.
- Boot sequence and UI audio effects.
- Theme-aware status bar and CLI rendering options.

## Requirements

- Rust stable toolchain (`cargo`, `rustc`)
- `curl` (preflight dependency)
- Optional: `vim`, `epy`
- Optional audio backend:
  - `python3`
  - `playsound`

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

On first run, if no users exist, RobCoOS creates a default admin account:

- Username: `admin`
- Password: `admin`

Login flow:

- Select a user on the login screen.
- Enter password (up to 3 attempts).
- After 3 failed attempts, the terminal locks.

Admin account management is available in:

- `Settings` -> `User Management`

## Controls

Menu navigation:

- `Up`/`Down` or `k`/`j` to move selection
- `Enter` or `Space` to select
- `q`, `Esc`, or `Tab` to go back

Session switching:

 - `Ctrl + Q`, then `1..9`
 - `Ctrl + Q`, then `N`/`Tab`/`0`/`+` for next/new session

## Menus

Main Menu entries:

- Applications
- Documents
- Network
- Games
- Program Installer
- Terminal
- Settings
- Logout

## Settings

Settings menu entries:

- About
- Theme
- CLI
- Edit Menus
- User Management (admin only)
- Bootup: ON/OFF
- Sound: ON/OFF

CLI submenu entries:

- Styled PTY Rendering: ON/OFF
- PTY Color Mode:
  - Theme Lock
  - Palette-map (Theme Shades)
  - Color (Default Terminal)
  - Monochrome
- Border Glyphs:
  - Unicode Smooth
  - ASCII

## Data Layout

Runtime data is stored relative to the executable directory.

```text
RobCoOS/
  robcos
  settings.json   (created at runtime)
  users/          (created at runtime)
```

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
