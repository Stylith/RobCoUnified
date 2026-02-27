# RobCoOS User Manual

## 1. Overview

RobCoOS is a terminal-first operating environment with two UI modes:

- Terminal mode (menu-driven)
- Desktop mode (windowed CLI shell)

Core capabilities:

- Multi-user authentication
- Per-user settings and menu data
- Up to 9 sessions with fast switching
- Embedded PTY app launching
- Document browsing and per-user logs/journal
- Program Installer (admin only)

Important scope note:

- Despite the name, RobCoOS is not a full standalone operating system. It is a terminal application environment/shell layer.

## 2. Quick Start

From the project root:

```bash
cargo run
```

Release mode:

```bash
cargo run --release
```

Skip preflight checks:

```bash
cargo run --release -- --no-preflight
```

Release validation:

```bash
make release-check
```

## 3. Dependencies

Startup preflight checks:

- Required:
  - `curl`
- Optional:
  - `epy` (external reader; default ebook app)
  - `vim`
  - `python3`
  - Python `playsound` (optional audio backend)

If required dependencies are missing, startup stops unless launched with `--no-preflight`.

## 4. Login, Users, and First-Run Behavior

### 4.1 First system start

If no users exist, RobCoOS creates:

- Username: `admin`
- Password: `admin`
- Auth method: `Password`
- Admin privileges: enabled

### 4.2 First login default app prompt (per user)

Each newly created user gets a one-time prompt after login:

- `Set default apps for your files.`

This opens `Settings -> Default Apps` flow immediately.

### 4.3 Auth methods

Available auth methods:

- `Password`
- `No Password`
- `Hacking Minigame`

## 5. Global Controls

General menu controls:

- Move: `Up` / `Down` or `k` / `j`
- Select: `Enter` or `Space`
- Back: `q`, `Esc`, or `Tab`

Text input:

- Type normally
- `Backspace` to delete
- `Enter` to confirm
- `Esc` to cancel

## 6. Sessions

RobCoOS supports up to 9 sessions per logged-in user.

### 6.1 Session switch keys (menus and normal screens)

- `Alt/Option + 1..9`
- `Ctrl + 1..9` (terminal support varies)
- `F1..F9`

### 6.2 Session switch inside PTY apps

Leader chord:

- `Ctrl + Q`, then `1..9` (direct switch)
- `Ctrl + Q`, then `N` / `n` / `Tab` / `0` / `+` (next/new)

Fallback chord:

- `~~`, then `1..9`

Desktop mode note:

- Session switching is disabled while Desktop Mode is active.

## 7. Main Menu (Terminal Mode)

Entries:

1. Applications
2. Documents
3. Network
4. Games
5. Program Installer
6. Terminal
7. Desktop Mode
8. Settings
9. Logout

## 8. Applications Menu

- Includes user-defined app entries from per-user `apps.json`.
- Includes built-in `Nuke Codes` only when visible.

Visibility control path:

- `Settings -> Edit Menus -> Edit Applications -> Nuke Codes in Applications: VISIBLE/HIDDEN [toggle]`

## 9. Documents and Default App Routing

Documents menu contains:

- `Logs` (journal)
- User-defined categories (folder mappings)

### 9.1 Category browsing

- Categories can contain subfolders and files.
- Supported document extensions for category listing:
  - `.pdf`, `.epub`, `.txt`, `.mobi`, `.azw3`

### 9.2 File open behavior

Open actions route through per-user Default Apps settings.

Default slots:

- `Text/Code Files`
- `Ebook Files`

Factory defaults:

- Text/Code: built-in `ROBCO Terminal Writer`
- Ebook: external `epy`

If no app is resolved for the selected file type, RobCoOS shows:

- `Error: No App for filetype`

### 9.3 Default App choices

You can set each slot to:

- Built-in app (where available)
- Existing menu entry from:
  - Applications
  - Games
  - Network
- Custom command via argv JSON

Custom argv input format example:

```json
["epy"]
```

## 10. Logs and Journal

Logs are per-user and stored under:

- `journal_entries/<username>/`

Actions:

- Create New Log
- View Logs
- Edit Log
- Delete Log

Editor controls:

- `Ctrl+W` or `F2`: save
- `Ctrl+X` or `Esc`: cancel

## 11. Network and Games

- Launch user-defined commands from per-user files:
  - `networks.json`
  - `games.json`

## 12. Program Installer (Admin Only)

Path:

- Main Menu -> Program Installer
- Desktop Start -> System -> Program Installer

Features:

- Search packages
- View installed packages
- Update / uninstall packages
- Add package commands to Applications/Games/Network menus
- Install optional audio runtime (`playsound`)

Supported package managers (auto-detected):

- `brew`
- `apt` / `apt-get`
- `dnf`
- `pacman`
- `zypper`

## 13. Terminal Menu

`Terminal` opens embedded shell PTY mode.

Exit methods:

- `exit`
- `Ctrl+D`

## 14. Desktop Mode

Desktop mode is a windowed shell in the same terminal.

### 14.1 Desktop surface

- Top status bar with clock/date and spotlight icon
- Taskbar with Start and window buttons
- Desktop icons:
  - `My Computer`
  - `Trash`

Icons can be dragged. Positions are persisted per user.

### 14.2 Start menu layout

Top-level Start entries:

- Applications
- Documents
- Network
- Games
- System
- Return To Terminal Mode
- Logout
- Shutdown

System submenu entries:

- Program Installer
- Terminal
- File Manager
- Settings

### 14.3 Window controls

Desktop windows support:

- Move (drag title bar)
- Resize (drag corners)
- Minimize
- Maximize/restore
- Close

### 14.4 Top menu bar

Desktop top menu categories:

- App
- File
- Edit
- View
- Window
- Help

### 14.5 File Manager and Documents in desktop mode

- My Computer opens File Manager.
- Trash icon opens Trash in File Manager.
- Desktop mode can reopen the last file manager tab set from the previous session.
- Recent folders are saved and exposed from the desktop File menu.
- Document category windows include a Back action that closes that category window.

## 15. Settings (Terminal Mode)

Path: Main Menu -> Settings

Entries:

- General
  - Sound [toggle]
  - Bootup [toggle]
  - Default Open Mode [toggle]
- Appearance
  - Theme
  - CLI Display
- Default Apps
- Connections
  - hidden when the platform disables connections support
  - Bluetooth is hidden when required tooling is missing
- Edit Menus
- User Management (admin)
- About

## 16. Settings (Desktop GUI)

Path: Desktop Start -> System -> Settings

Home panels:

- General
- Appearance
- Default Apps
- Connections
  - hidden when the platform disables connections support
  - Bluetooth is hidden when required tooling is missing
- CLI Profiles
- Edit Menus
- User Management (admin)
- About

Notable appearance controls:

- Theme chooser
- CLI Display
- Desktop cursor toggle
- Desktop icon style:
  - DOS
  - Win95
  - Minimal
  - No Icons
- Wallpaper management (including custom wallpapers)

Default Apps panel mirrors terminal Default Apps behavior.

## 17. Edit Menus

Path:

- Terminal: `Settings -> Edit Menus`
- Desktop: `Settings -> Edit Menus` panel

Editors:

- Edit Applications
- Edit Documents
- Edit Network
- Edit Games

### 17.1 Edit Applications

Includes:

- Built-in visibility toggle:
  - `Nuke Codes in Applications: VISIBLE/HIDDEN [toggle]`
- Add app entry
- Delete app entry

### 17.2 Add command entry format

App/network/game commands are stored as argv arrays.
Input command strings are split by whitespace.

For complex shell pipelines/chains, use a wrapper script and call that script.

### 17.3 Edit Documents

Category mapping requires:

- Category name
- Existing directory path

## 18. User Management (Admin Only)

Path:

- Settings -> User Management

Actions:

1. Create User
2. Delete User
3. Reset Password
4. Change Auth Method
5. Toggle Admin

Notes:

- You cannot delete the currently logged-in user.
- Creating users in terminal or desktop marks them for first-login Default Apps setup.

## 19. Data Layout

Runtime data is stored relative to executable base directory.

```text
<base_dir>/
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

Per-user `settings.json` includes:

- Theme, CLI settings, open mode, bootup/sound
- Default Apps bindings
- Desktop settings (cursor, icon style, wallpapers, file manager preferences)
- Persisted desktop icon positions

## 20. Troubleshooting

### 20.1 Session switching inside PTY does not trigger

Use leader chord:

- `Ctrl+Q` then session key

Fallback:

- `~~` then session key

### 20.2 App command from menu fails

- Confirm command exists in `PATH`.
- Test command directly in a normal terminal.
- For complex commands, use a script wrapper.

### 20.3 Documents do not open as expected

- Check `Settings -> Default Apps` for slot bindings.
- Verify selected app exists and is runnable.
- For custom command JSON, ensure valid non-empty argv array.

### 20.4 Audio issues

- Check `Settings -> Sound` is ON.
- Confirm platform audio tool availability.
- Optionally install `playsound` from Program Installer.

### 20.5 Connections menu is missing or reduced

- On macOS, the entire Connections feature may be hidden when the current implementation is unsupported.
- Bluetooth entries are hidden when `blueutil` is not installed or available.
- This is intentional; unsupported actions are removed from menus instead of failing after selection.

## 21. AI Assistance Disclaimer

This project was created with the help of AI-assisted development tools.
