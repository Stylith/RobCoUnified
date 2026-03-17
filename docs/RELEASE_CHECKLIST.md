# Release Checklist

Use this before publishing a release tag.

## Pre-Tag

- Run `cargo check`.
- Run `make release-check`.
- Confirm [CHANGELOG.md](/Users/hal-9000/RobCoUnified/CHANGELOG.md) has a section for the release version.
- Confirm [README.md](/Users/hal-9000/RobCoUnified/README.md) and [USER_MANUAL.md](/Users/hal-9000/RobCoUnified/USER_MANUAL.md) are up to date for user-visible changes.

## Native Smoke Pass

### Terminal Mode

- Login screen shows existing users.
- Login, logout, and session switching work.
- Terminal shell opens the real user shell.
- Common installed commands resolve, especially Homebrew-installed commands on macOS.
- Default Apps, Connections, Programs, Edit Menus, and Document Browser open and return correctly.
- Installer detects the platform package manager and runtime tools.

### Desktop Mode

- Top bar, taskbar, Start menu, and Spotlight all open and behave correctly.
- Editor opens, edits, saves, and reopens files.
- File Manager navigation, rename, new folder, open-with, and picker flows work.
- Settings opens, routes between panels correctly, and saves changes.
- Applications window launches built-ins and configured apps.
- Installer window opens, searches, and performs install actions.
- About and Nuke Codes open and close correctly.

## macOS Bundle Checks

- App icon appears correctly in Finder and Dock.
- Retro font is loaded in both native UI and PTY.
- Existing users, settings, installed apps, and other runtime data migrate from prior app builds.
- Finder-launched app still detects Homebrew and other external tools.
- Packaged `.app` launches without depending on the repo working directory.

## Release Publishing

- Create the release tag only after the checks above pass.
- Let GitHub Actions build the release as a draft.
- Download and spot-check at least the macOS artifact before publishing the draft.
- Publish the draft only after the artifact checks pass.
