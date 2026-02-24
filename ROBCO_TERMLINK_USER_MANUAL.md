
# ROBCO INDUSTRIES™
## UNIFIED OPERATING SYSTEM
### CONSUMER TERMLINK USER HANDBOOK
**Model Series:** RC-2075 Personal Terminal (Domestic)  
**OS Package:** RobCoOS (RustCoOS)  
**Edition:** Consumer / Household Computing  
**Print Year:** 2076 (Reprint 2077)  
**Document Code:** RC-TERM-HB-2076-A (ALPHA FIRMWARE)

---

> **WELCOME, VALUED ROBCO CUSTOMER!**  
> Thank you for selecting **RobCo Industries™**, America’s leader in household automation and computational leisure.  
> This handbook will assist you in operating your RobCo Terminal safely, efficiently, and patriotically.

---

## TABLE OF CONTENTS

1. System Overview  
2. Quick Start & Authorized Launch Procedures  
3. Required & Optional Modules (Preflight Check)  
4. First Login (Default Household Administrator)  
5. Global Controls & Text Entry  
6. Status Bar & System Indicators  
7. Session System (Up to 9 Concurrent Work Environments)  
8. Main Menu Directory  
9. Applications Directory  
10. NUKE CODES Utility  
11. Documents Directory  
12. Logs (Journal) — Create / View / Edit / Delete  
13. Document Categories & Supported Formats  
14. Network & Games Directories  
15. Program Installer (Administrator Clearance Only)  
16. Terminal (RobCo Maintenance TERMLINK)  
17. Settings Directory  
18. CLI Submenu (PTY Rendering Controls)  
19. Edit Menus (Adding/Removing Entries)  
20. User Management (Administrator Only)  
21. Authentication & Lock Behavior  
22. Logout & Process Cleanup  
23. Data Storage & File Locations  
24. Common Tasks  
25. Troubleshooting & Field Repairs  
26. Consumer Safety Disclaimer (ABSURDLY IMPORTANT)

---

## 1. SYSTEM OVERVIEW

RobCoOS is a terminal-style operating environment featuring:

- **Multi-user login** (household profiles)
- **Per-user menus and settings**
- Up to **9 switchable sessions**
- Embedded **PTY applications** (for tools like `vim`, `ranger`, `calcurse`, etc.)
- Built-in directories for **Applications**, **Documents**, **Network**, **Games**, **Program Installer**, **Terminal**, and **Settings**

RobCoOS is engineered to “feel” like a single unified machine, even when multiple household tasks are active simultaneously.

---

## 2. QUICK START & AUTHORIZED LAUNCH PROCEDURES

From the project root (`RustCoOS/`):

```bash
cargo run
```

Release mode (recommended for stability):

```bash
cargo run --release
```

Skip dependency preflight checks *(Field Technician Override)*:

```bash
cargo run --release -- --no-preflight
```

> **NOTE:** RobCo Industries discourages bypassing safety checks unless instructed by a certified RobCo technician, a Vault-Tec representative, or a suspiciously confident stranger wearing goggles indoors.

---

## 3. REQUIRED & OPTIONAL MODULES (PREFLIGHT CHECK)

On startup, RobCoOS performs an automated **PRE-WAR INTEGRITY PREFLIGHT™**.

### Required
- `curl`

### Optional (Recommended Enhancements)
- `epy` — document reading module
- `vim` — editing workflows
- `python3` — auxiliary runtime
- Python module `playsound` — legacy/extra audio backend

If required dependencies are missing, startup stops unless you launch with `--no-preflight`.

---

## 4. FIRST LOGIN (DEFAULT HOUSEHOLD ADMINISTRATOR)

On first run (when no users exist), RobCoOS creates:

- **Username:** `admin`
- **Password:** `admin`
- **Auth Method:** Password
- **Administrator Privileges:** Enabled

⚠ **CHANGE THIS IMMEDIATELY AFTER FIRST LOGIN.**  
If you do not, RobCo Industries will assume you enjoy risk, adventure, and/or learning lessons the hard way.

---

## 5. GLOBAL CONTROLS & TEXT ENTRY

### Menu Navigation (System-Wide)
- Move: `Up` / `Down` or `k` / `j`
- Select: `Enter` or `Space`
- Back: `q`, `Esc`, or `Tab`
- Confirm dialogs: `y` / `n`

### Text Input
- Type normally
- Backspace deletes
- `Enter` confirms
- `Esc` cancels

---

## 6. STATUS BAR & SYSTEM INDICATORS

The bottom status bar shows:

- **Left:** date/time
- **Center:** sessions shown as `[1][2][3*]` (`*` indicates active)
- **Right:** battery percentage and charging/discharging indicator *(when available)*

> Some consumer-grade machines do not expose battery telemetry. In such cases, the terminal may display nothing, a placeholder, or a disapproving silence.

---

## 7. SESSION SYSTEM (IMPORTANT)

RobCoOS supports up to **9 sessions** for the currently logged-in user.

### 7.1 Switching Sessions on Normal Menus/Screens
Use any of these (terminal support varies by platform):

- `Alt/Option + 1..9`
- `Ctrl + 1..9`
- `F1..F9`

### 7.2 Switching Sessions Inside PTY Apps (Recommended)
For applications running in embedded terminal mode (`vim`, `ranger`, `calcurse`, etc.) use the **leader chord**:

```
Ctrl+Q, then 1..9
```

Additional PTY session commands:

- `Ctrl+Q`, then `n` / `N` / `Tab` / `0` / `+` → next/new session

Fallback chord *(in case of stubborn TERMLINK software)*:

```
~~ then 1..9
```

### 7.3 Session Behavior
- PTY apps are **suspended** when you switch away
- They **resume** instantly when you return (tmux-like behavior)

---

## 8. MAIN MENU DIRECTORY

Main Menu entries:

1. Applications
2. Documents
3. Network
4. Games
5. Program Installer
6. Terminal
7. Settings
8. Logout

---

## 9. APPLICATIONS DIRECTORY

### 9.1 Built-in Utility
- **Nuke Codes** is always present.

### 9.2 User Applications
Additional entries come from the current user’s `apps.json`.

---

## 10. NUKE CODES UTILITY

Purpose:
- Fetches live **Alpha/Bravo/Charlie** launch code data from community sources.
- Data changes weekly at the source; the app refreshes on open and on demand.

Controls:
- `R` → refresh immediately
- `q` / `Esc` / `Tab` → return

> RobCo Industries neither confirms nor denies the usefulness of this utility for “fireworks,” “home defense,” “statecraft,” or “lighthearted mischief.”

---

## 11. DOCUMENTS DIRECTORY

Contains:
- **Logs** (built-in journal)
- User-defined document categories (folders)

---

## 12. LOGS (JOURNAL)

Options:
- Create New Log
- View Logs
- Delete Logs

Inside a log:
- View
- Edit
- Delete

Editor controls:
- `Ctrl+W` (or `F2`) → save
- `Ctrl+X` or `Esc` → cancel

Logs are stored per user under:

```
journal_entries/<username>/
```

---

## 13. DOCUMENT CATEGORIES & SUPPORTED FORMATS

Each category points to a folder path.

Supported file extensions:
- `.pdf`, `.epub`, `.txt`, `.mobi`, `.azw3`

Reader launch:
- Uses `epy` in suspended terminal mode.

---

## 14. NETWORK & GAMES DIRECTORIES

- Launch custom commands saved per user.
- Menu data files:
  - `networks.json`
  - `games.json`

---

## 15. PROGRAM INSTALLER (ADMIN ONLY)

Only administrator users can open **Program Installer**.

Menu:
1. Search
2. Installed Apps
3. Install Audio Runtime (`playsound`)

Supported package managers (auto-detected):
- `brew`
- `apt` / `apt-get`
- `dnf`
- `pacman`
- `zypper`

### 15.1 Search
- Searches available packages
- Allows installation of selected packages

### 15.2 Installed Apps
For each installed package:
- Update
- Uninstall
- Add to Menu

**Add to Menu** targets:
- Applications
- Games
- Network

### 15.3 Install Audio Runtime
Installs/upgrades Python `playsound` for users who want that backend.

---

## 16. TERMINAL (ROBCO MAINTENANCE TERMLINK)

`Terminal` opens your system shell in embedded PTY mode.

Behavior:
- Prompt forced to `>`
- Top bar shows: **ROBCO MAINTENANCE TERMLINK**
- Startup files minimized for consistency:
  - `bash --noprofile --norc`
  - `zsh -f`

Exit terminal:
- `exit`
- `Ctrl+D`

---

## 17. SETTINGS DIRECTORY

Settings entries:

1. About
2. Theme
3. CLI
4. Edit Menus
5. User Management *(admin only)*
6. Bootup: ON/OFF
7. Sound: ON/OFF

### 17.1 About
Shows system info and branding block.

### 17.2 Theme
Changes overall app theme color.

### 17.3 Bootup + Sound Toggles
- **Bootup** controls whether boot sequence plays at startup.
- **Sound** controls UI/system sound effects.

---

## 18. CLI SUBMENU (PTY RENDERING CONTROLS)

Controls PTY rendering behavior:

- **Styled PTY Rendering:** ON/OFF
- **PTY Color Mode** cycles:
  - Theme Lock
  - Palette-map (Theme Shades)
  - Color (Default Terminal)
  - Monochrome
- **Border Glyphs** toggle:
  - Unicode Smooth
  - ASCII

> For maximum authenticity, RobCo Industries recommends **ASCII borders** and **Monochrome** display when operating near suspicious humming, dust storms, or dramatic lighting.

---

## 19. EDIT MENUS (ADDING/REMOVING ENTRIES)

Path:
- `Settings → Edit Menus`

Submenus:
- Edit Applications
- Edit Documents
- Edit Network
- Edit Games

### 19.1 Add Command Entries (Applications / Network / Games)

Flow:
1. Enter display name
2. Enter launch command

**Important command parsing rule:**  
Commands are split by whitespace. Complex shell syntax (quotes/pipes/chains) is not parsed as a full shell string.

For complex commands:
- Create a script
- Launch the script path instead

### 19.2 Edit Document Categories
- Add Category: name + folder path
- Delete Category: remove mapping

Folder path notes:
- `~` expands to home directory
- Folder must exist and be a directory

---

## 20. USER MANAGEMENT (ADMIN ONLY)

Path:
- `Settings → User Management`

Actions:
1. Create User
2. Delete User
3. Reset Password
4. Change Auth Method
5. Toggle Admin

### 20.1 Authentication Methods
- Password
- No Password
- Hacking Minigame

### 20.2 Create User
- Enter username
- Choose auth method
- If Password: enter and confirm

### 20.3 Delete User
- Cannot delete yourself

### 20.4 Reset Password
- Select user
- Enter new password + confirmation
- Auth forced to Password

### 20.5 Change Auth Method
- Select user
- Choose new method
- If Password: set password

### 20.6 Toggle Admin
- Grants/revokes admin rights
- Current logged-in user excluded

---

## 21. AUTHENTICATION & LOCK BEHAVIOR

- Password login allows **up to 3 attempts**
- After 3 failed attempts, lock screen is shown
- No Password users log in immediately
- Hacking Minigame users must complete the minigame

---

## 22. LOGOUT & PROCESS CLEANUP

Logout performs:

- Plays logout sound
- Clears active user
- Clears all sessions
- Terminates suspended PTY child processes
- Returns to login screen

---

## 23. DATA STORAGE & FILE LOCATIONS

Runtime data is stored next to the executable (`base_dir = parent of current executable`).

Typical layout:

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

Notes:
- Under `cargo run`, `<base_dir>` is typically under `target/debug` or `target/release`
- In distributed builds, `<base_dir>` is wherever the binary is placed

---

## 24. COMMON TASKS

### 24.1 Add a New App to Applications (via Menus)
1. `Settings → Edit Menus → Edit Applications → Add App`
2. Enter display name
3. Enter command (example: `ranger`)

### 24.2 Add an Installed Package to a Menu
1. `Program Installer → Installed Apps`
2. Select package
3. `Add to Menu`
4. Pick target menu
5. Enter display name

### 24.3 Create a New User
1. Log in as admin
2. `Settings → User Management → Create User`
3. Choose auth method
4. Set password if using Password mode

### 24.4 Change a User’s Auth Mode
1. `Settings → User Management → Change Auth Method`
2. Select user
3. Choose new method
4. If Password mode: set a new password

---

## 25. TROUBLESHOOTING & FIELD REPAIRS

### 25.1 Session Switching Does Not Work Inside PTY App
Use:
- `Ctrl+Q` then session key (`1..9`)

Fallback:
- `~~` then `1..9`

### 25.2 App Command Fails From Menu
- Verify command exists in `PATH`
- Try launching directly in a normal terminal first
- For complex command lines, create a wrapper script and launch it

### 25.3 Documents Do Not Open
- Verify `epy` is installed
- Verify file extension is supported
- Verify category folder path is valid

### 25.4 No Sound / Partial Sound
- Check `Settings → Sound` is ON
- Confirm audio backends exist on your platform
- Use `Program Installer → Install Audio Runtime` if `playsound` is desired

### 25.5 Missing Battery Indicator
- Some systems do not expose battery info
- In that case, the field may be blank or placeholder

---

## 26. CONSUMER SAFETY DISCLAIMER

**ROBCO INDUSTRIES™ IMPORTANT NOTICE:**  
By operating this terminal, you agree that RobCo Industries is not responsible for:

- Loss of data, loss of dignity, loss of hair, or loss of faith in humanity  
- Any terminal becoming self-aware and requesting a middle name  
- Any terminal becoming *patriotic* and requesting your allegiance  
- Improper installation of “NUKE CODES” resulting in excessive fireworks, moderate fireworks, or extremely enthusiastic fireworks  
- Damage caused by: dust, sand, water, coffee, radiation, static electricity, dynamic electricity or Forced Evolutionary Virus
 

**ROBCO INDUSTRIES™** — *The Future Is Yours. The Liability Is Not Ours.*
