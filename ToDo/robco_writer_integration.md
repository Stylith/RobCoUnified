# RobCo Writer (Vim Integration) Implementation Guide

## Overview

This document describes how to integrate a RobCo-themed text editor module into the application using Vim.

The goal is:
- Use Vim as the underlying editor
- Apply RobCo theming and behavior
- Keep maintenance low (no deep Vim fork)
- Integrate cleanly as a built-in RobCo app

---

## Architecture

Use **Vim + custom runtime configuration**, not a full fork.

### Structure

robco/
  bin/
    vim
    robco-writer (launcher)
  runtime/
    robco_vimrc
  addons/

---

## Core Features

### 1. Theme Integration

The editor must inherit the app's current theme dynamically.

Use environment variables:

ROBCO_THEME
ROBCO_FG
ROBCO_DIM
ROBCO_ACCENT
ROBCO_ERROR

The vimrc should:
- read these values
- apply them to highlight groups
- fallback to defaults if not set

DO NOT hardcode colors.

---

### 2. Startup Screen (RobCo Intro)

Replace Vim default intro with:

ROBCO INDUSTRIES (TM) TERMLINK PROTOCOL

TEXT EDITOR MODULE INITIALIZED

AVAILABLE KEYS:
  F1  HELP
  F2  SAVE
  F3  QUIT
  F4  SAVE AND QUIT

COMMANDS:
  :new        OPEN NEW BUFFER
  :e <file>   OPEN FILE
  :w          WRITE FILE
  :q          QUIT

READY.

Behavior:
- Only show when no file is opened
- Read-only buffer
- No line numbers
- No editing

---

### 3. Statusline (RobCo Style)

Format:

ROBCO WRITER | FILE: <name> | LINE: <n> | COL: <n> | MODE: <mode>

Mode mapping:
- NORMAL → COMMAND
- INSERT → INSERT
- VISUAL → SELECT
- REPLACE → REPLACE

---

### 4. Keybind Layer (User-Friendly)

Add:

F1 → help
F2 → save
F3 → quit
F4 → save+quit

Also:

Leader+h → clear search
Leader+n → new buffer
Leader+w → save
Leader+q → quit
Leader+x → save+quit

---

### 5. File Behavior

Defaults:
- No swap files
- No backups
- Unix line endings
- Smart indentation
- Case-aware search

---

### 6. Text Mode Defaults

For text/markdown:
- wrap enabled
- linebreak enabled
- spell check enabled

---

### 7. Holotape / Read-Only Mode

If launched with:

ROBCO_READONLY=1

Then:
- set Vim to readonly
- disable writes

---

### 8. Custom Commands

Add:

:RobCoHelp
:RobCoClear
:RobCoNew

---

## Wrapper Launcher

Create robco-writer launcher.

Responsibilities:
- Set environment variables from RobCo
- Load custom vimrc
- Pass file arguments

Example (Linux/macOS):

#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

export ROBCO_THEME="${ROBCO_THEME:-default}"

exec "$SCRIPT_DIR/vim" -u "$SCRIPT_DIR/../runtime/robco_vimrc" "$@"

---

## Integration Into RobCo

### App Registration

id: robco_writer
name: RobCo Writer
type: builtin
launch: external process
command: robco-writer

---

### File Associations

Set as default for:
- .txt
- .md
- .log
- config/code files

---

### Launch Behavior

When opening a file:

1. Resolve theme
2. Set environment variables
3. Launch:

robco-writer <file>

Optional:
ROBCO_READONLY=1

---

### UI Integration

In RobCo:

- Add "RobCo Writer" to Applications
- Add "New Document"
- Add "Open with RobCo Writer"
- Use for logs/journal system

---

## Packaging

Bundle:
- Vim binary
- robco_vimrc
- launcher script
- Vim license file

Do not modify Vim core unless necessary.

---

## Future Improvements

- Multiple themes (handled via env)
- Help file integration
- Plugin support
- Custom file headers

---

## Summary

This approach provides:
- Full Vim power
- RobCo visual identity
- Low maintenance
- Clean integration

Avoid:
- Forking Vim
- Hardcoding themes
- Breaking Vim behavior

---

END OF FILE
