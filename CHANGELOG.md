# Changelog

All notable changes to RobCoOS will be documented in this file.

The format is based on Keep a Changelog, adapted to the way this project ships releases.

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
