# Changelog

All notable changes to RobCoOS will be documented in this file.

The format is based on Keep a Changelog, adapted to the way this project ships releases.

## 0.4.4 - 2026-03-17

### Fixed

- Embedded the native app icon into the shell binary so the macOS `.app` no longer falls back to the default eframe icon when launched outside the repo root.
- Fixed terminal shell startup so bundled macOS builds open a normal login shell instead of suppressing startup files, restoring Homebrew-installed commands in the PTY.
- Added explicit macOS bundle icon metadata for the packaged `.app` resource icon in the release workflow.

## 0.4.3 - 2026-03-17

### Fixed

- Fixed macOS app-bundle data migration by importing legacy runtime state from `RobCoOS.app/Contents/MacOS` into `~/Library/Application Support/RobCoOS`.
- Fixed user migration so existing `users.json` entries replace the temporary bootstrap admin record instead of leaving prior accounts hidden.
- Fixed installer and runtime-tool detection for Finder-launched macOS builds by resolving `brew`, `python3`, `blueutil`, and related commands from absolute fallback paths instead of relying on inherited shell `PATH`.
- Added direct regression coverage for the macOS runtime-data migration and command-path resolution failures that escaped the `0.4.2` release.

## 0.4.2 - 2026-03-17

### Fixed

- Fixed the macOS `.app` runtime data path so bundled builds create and read users, settings, and related state from `~/Library/Application Support/RobCoOS` instead of inside the app bundle.
- Embedded the retro font into the native UI and PTY renderer so standalone app bundles keep the intended typeface without relying on repo-relative `assets/fonts` paths.
- Stabilized the native release gate by making document-category service tests independent of global user-scoped config state.

## 0.4.1 - 2026-03-17

### Fixed

- Fixed the release workflow so tag builds use the updated checkout and artifact actions.
- Switched Linux release jobs to current GitHub-hosted x86_64 and ARM runners with explicit native GUI build dependencies.
- Packaged macOS as a real universal `RobCoOS.app` bundle with the app icon instead of a bare executable zip.
- Fixed the Linux shared sound build path so release builds no longer fail on the missing `Stdio` symbol.

## 0.4.0 - 2026-03-17

### Changed

- Promoted the native shell to the release target and shipped native-only release assets.
- Switched macOS release packaging to a single universal binary target before the later `.app` bundle follow-up.
- Split the native codebase into shared, service, shell, and app-focused workspace crates.
- Moved editor, file manager, terminal, installer, settings, programs, default apps, connections, edit menus, document browser, about, and nuke codes behind cleaner app boundaries.
- Centralized native desktop menu, taskbar, session, launcher, file, settings, and status behavior behind shared services.
- Refreshed the README and user manual to match the workspace-native architecture.
