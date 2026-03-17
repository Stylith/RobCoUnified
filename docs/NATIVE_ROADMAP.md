# Native Roadmap

This is the current native rewrite roadmap for `robcos-native`.

## Product Rules

1. Terminal mode and desktop mode remain separate first-class experiences.
2. Legacy `robcos` stays intact as the reference surface until native parity is good enough.
3. Native work should keep moving logic out of UI code and into reusable app/service layers.

## Current Architecture

- `crates/native-shell`: native launcher binary
- `crates/legacy-shell`: legacy launcher binary
- `crates/native-services`: shared native service/domain helpers
- `src/native/`: native shell UI, app presenters, and runtime orchestration
- `src/legacy/`: legacy/reference implementation

## Completed Refactor Work

### Phase 1: App Boundaries

- editor state and commands moved into `editor_app`
- file manager behavior split across:
  - `file_manager`
  - `file_manager_app`
  - `file_manager_desktop`
  - desktop presenter helpers

### Phase 2: Desktop Shell Menus and Actions

- top bar menus use shared menu specs instead of app-specific inline shell code
- taskbar and desktop shell actions route through shared desktop action models

### Phase 3: Native Services

The shell no longer owns most business logic directly. Shared native services now cover:

- files and file reveal/open planning
- launcher/catalog persistence and command resolution
- settings persistence and default-app updates
- connections and saved/discovered connection flows
- session/login/logout/session-restore planning
- users and user-management persistence
- status message helpers
- document categories
- Start/Spotlight search data
- desktop shortcuts
- desktop surface settings and icon/wallpaper policy

## Current Gaps

### Architecture

- `src/native/app.rs` is still too large and still owns too much UI orchestration
- `crates/native-services` still shares some source files from the main tree and needs a cleaner long-term boundary
- editor and file manager are modular, but not yet standalone crates/apps

### Product / Parity

- native desktop still needs more visual and interaction parity polish
- terminal-mode native parity still has remaining gaps in some settings/admin/utility flows
- more built-ins still need app-owned menu/state surfaces

## Next Steps

1. Finish the native-services boundary so native UI imports the crate cleanly.
2. Split native shell-facing shared models into a stable app/API layer.
3. Move editor into its own native app crate boundary.
4. Move file manager into its own native app crate boundary.
5. Revisit other built-ins after those two patterns are stable.

## Release Target

The next sensible release after the current native/workspace/service work is `0.4.0`, not a patch release.
