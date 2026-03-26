# Neutral-Core Refactor Handoff

This file is the concrete continuation handoff for the `WIP` branch.

Use it when resuming this refactor with Codex or another agent on a different machine.

## Current Status

- Repo: `RobCoUnified`
- Working branch: `WIP`
- Base branch for this work: `experimental`
- Refactor goal: move from a product-branded, built-in-app shell toward a neutral core platform with first-party and later third-party addons
- Current strategy: incremental migration, not rewrite
- Phase status:
  - Phase 0 neutral contract layer: done enough
  - Phase 1 runtime adoption and capability routing: complete for the current shell architecture scope
  - Phase 2 path-authority/state-root migration: complete for the current scope
  - Phase 3 `app.rs` coordinator decomposition: done enough for this stage
  - Phase 4 scoped manifests + addon state + inventory UI: started and now materially real
  - Phase 5 packaged first-party addons / external install-remove lifecycle: substantially complete — see "Phase 5 Progress" section below
  - Phase 6 branding/theme/content extraction: not done
  - Phase 7 third-party addon model: intentionally not started

Important constraint summary:

- This is an application-layer shell / desktop environment, not a real operating system.
- Linux is the full-environment target.
- Windows and macOS stay supported under lighter launcher-style profiles.
- Built-in apps must become first-party addons over time.
- Core must depend on capabilities, not app names.
- Shell must not depend on app internals.
- Avoid native dynamic library plugin loading for now; the current preferred external-addon runtime direction is sandboxed WASM for shell-integrated addons.
- Keep desktop mode and terminal menu mode aligned; do not let one drift too far ahead of the other.

Current practical status summary:

- shared neutral contracts exist for install profiles, logical paths, manifests, registry, shell launch/action types, scoped manifest discovery, and addon enabled-state
- desktop and terminal addon-backed entry points mostly launch through capability/addon resolution instead of direct hardcoded UI actions
- `src/native/app.rs` is no longer the old god-file shape; most cohesive runtime blocks are extracted
- compatibility path migration is in place for the named legacy runtime files and bundled binary roots
- installer/addon-manager groundwork is now real:
  - discovered scoped manifests
  - layered addon inventory
  - persisted enable/disable state
  - desktop + terminal installed-addon inventory views
  - essential vs optional addon separation
  - user-scoped addon install/remove path imports
  - discovery issue/provenance visibility in addon inventory
- current essential-addon policy:
- shell-critical first-party addons are essential; optional first-party addons are treated as external packages and should not be named or owned by the core repo

Important honesty note:

- the codebase is much cleaner and more modular than before, but it is not fully untangled yet
- contract/policy separation is far ahead of runtime/package separation
- there are still hardwired first-party runtime mappings and shell-owned window/screen enums that know about current first-party addons
- product branding and theme assumptions are still present in many UI/runtime surfaces

Near-term engineering rules while finishing the current roadmap:

- Do not pause the current roadmap for a full shell-theme rewrite yet.
- Do stop adding new shell-facing behavior through raw theme checks or scattered capability string checks.
- Any shell-facing visibility/launch decision touched during normal work should prefer shared registry/launch-target resolution over local `if theme == ...` or `if capability == ...` branches.
- Treat desktop mode and terminal mode as separate first-class shell profiles, not a single theme with toggles.
- Separate these concerns explicitly even before the full refactor:
  - structure/layout profile: which slots or regions exist
  - component behavior: what fills a slot functionally
  - skin/presentation: how that component looks
  - capability/schema: which settings/options are valid for the active component/skin
- A component and its visual must remain separate objects. For example, `dock` is not the same thing as `macos dock skin`.
- Do not build the global theme chooser first. The chooser should come after registry/config support for layout profiles, components, skins, and capability-driven settings.

## What Has Been Implemented So Far

The safe-first sequence has been followed. The project now has:

1. neutral shared contracts
2. capability-based runtime adoption across desktop and terminal addon-backed flows
3. major native runtime decomposition
4. first compatibility pass for path authority
5. scoped manifest discovery and addon state groundwork
6. a real installed-addon inventory surface in the Program Installer

The sections below retain the detailed slice-by-slice history.

New shared platform contracts were added in `crates/shared/src/platform/`:

- `profile.rs`
  - `InstallProfile`
  - `IntegrationLevel`
- `paths.rs`
  - `LogicalRoot`
  - `PlatformPaths`
  - `PlatformPathEnvironment`
  - `ResolvedPlatformPaths`
- `addons.rs`
  - `AddonId`
  - `CapabilityId`
  - `PermissionId`
  - `AddonKind`
  - `AddonScope`
  - `AddonTrust`
  - `AddonEntrypoint`
  - `FileAssociation`
  - `AddonManifest`
  - `AppDefinition`
- `registry.rs`
  - `AddonRegistry`
  - `RegistryError`
- `shell.rs`
  - `LaunchTarget`
  - `LaunchSurface`
  - `ShellAction`
  - `ShellEvent`

The shared module is exported through `crates/shared/src/lib.rs` and `src/lib.rs`.

The product-layer first-party addon catalog and runtime table live in `src/native/addons.rs`.

That file currently defines code-backed manifests for:

- settings
- file manager
- editor
- document browser
- terminal
- installer
- programs
- default apps
- connections
- edit menus
- about

Current essential/optional split:

- essential:
  - settings
  - file manager
  - editor
  - document browser
  - terminal
  - installer
  - programs
  - default apps
  - connections
  - edit menus
  - about
- optional:
  - externally distributed first-party addons only

Runtime adoption is no longer limited to Settings. The shell now uses capability/addon routing for a broad first-party set.

Adopted runtime flows in the native shell include:

- `src/native/app/launch_registry.rs` was added as the first runtime launch adapter
- `DesktopShellAction` now supports a shared `LaunchTarget`
- desktop Start/Spotlight/menu/context/IPC launch paths for Settings, File Manager, Editor, Terminal, Applications, Installer, Connections, Default Apps, Edit Menus, About, and optional addon entries now route through capability/addon resolution where appropriate
- terminal menu mode now uses the same registry seam for addon-backed destinations instead of a separate local lookup table
- settings subtools now have addon-backed identities even where they still render inside the Settings host
- visible behavior has been intentionally preserved; most of the change is in launch contract and ownership boundaries

## Files Added Or Changed

Added:

- `crates/shared/src/platform/mod.rs`
- `crates/shared/src/platform/profile.rs`
- `crates/shared/src/platform/paths.rs`
- `crates/shared/src/platform/addons.rs`
- `crates/shared/src/platform/registry.rs`
- `crates/shared/src/platform/shell.rs`
- `src/native/addons.rs`
- `src/native/app/launch_registry.rs`

Changed:

- `crates/shared/src/lib.rs`
- `docs/NEUTRAL_CORE_HANDOFF.md`
- `src/lib.rs`
- `src/native/app.rs`
- `src/native/app/desktop_spotlight.rs`
- `src/native/app/desktop_start_menu.rs`
- `src/native/mod.rs`
- `src/native/desktop_app.rs`

## Verified State

Verified:

- `cargo test -p robcos-shared platform`

That test run passed and covered:

- install-profile path mapping
- registry duplicate detection
- registry capability indexing
- registry file-extension indexing

Partially verified:

- `cargo test -p robcos first_party_registry_exposes_core_capabilities --lib`

That broader root-crate test was started to compile the product layer and GUI dependency graph, but no completed pass/fail result was captured in the previous session. Re-run it on the next machine.

Additional verified slice:

- `cargo test -p robcos settings_capability_resolves_to_settings_panel --lib`
- `cargo test -p robcos settings_launch_target_opens_settings_window --lib`
- `cargo test -p robcos desktop_menu_open_settings_uses_registry_launch --lib`
- `cargo test -p robcos generic_context_menu_open_settings_uses_registry_launch --lib`
- `cargo test -p robcos file_manager_capability_resolves_to_file_manager_window --lib`
- `cargo test -p robcos file_manager_launch_target_opens_file_manager_window --lib`
- `cargo test -p robcos desktop_menu_open_file_manager_uses_registry_launch --lib`
- `cargo test -p robcos desktop_program_request_open_file_manager_uses_registry_launch --lib`
- `cargo test -p robcos editor_capability_resolves_to_editor_window --lib`
- `cargo test -p robcos editor_launch_target_opens_editor_window --lib`
- `cargo test -p robcos desktop_program_request_open_text_editor_uses_registry_launch --lib`
- `cargo test -p robcos open_text_editor_action_uses_registry_launch --lib`
- `cargo test -p robcos-shared platform`
- `cargo test -p robcos roundtrip_ipc_message --lib`
- `cargo test -p robcos settings_changed_serializes --lib`

Those two tests verify:

- the new launch adapter resolves the Settings capability through the first-party addon registry
- the native shell still opens the existing Settings window when that launch target is used

The additional menu/context tests verify:

- desktop menu bar Settings uses the registry-backed launch path
- desktop context menu Settings uses the registry-backed launch path
- desktop menu bar File Manager uses the registry-backed launch path
- desktop program-request File Manager uses the registry-backed launch path
- desktop program-request Editor uses the registry-backed launch path
- shell-level OpenTextEditor now uses the registry-backed launch path
- desktop program-request optional addon entries use the registry-backed launch path
- shell-level optional addon launches now use the registry-backed launch path

Additional path-adoption slice:

- shared runtime-path detection now lives in `crates/shared/src/platform/runtime.rs`
- the shared layer now exposes `RuntimeEnvironment`
- install-profile parsing now accepts explicit profile strings such as `linux-desktop`, `windows-launcher`, `mac-launcher`, and `portable-dev`
- config now exposes `runtime_environment()`, `install_profile()`, `platform_paths()`, and `runtime_root_dir()`
- native IPC now uses the logical `runtime_root` instead of placing `shell.sock` under the legacy data directory
- native log/journal writes no longer depend on the process working directory; they resolve under `base_dir()/journal_entries`

Additional runtime-registry slice:

- `src/native/addons.rs` now contains an explicit first-party runtime registry separate from manifest metadata
- desktop launch resolution no longer infers behavior from manifest route strings; it resolves manifest target -> addon id -> runtime registry entry
- the first Start-menu Program Installer action now launches through `LaunchTarget::Capability("installer-ui")`
- the Spotlight Terminal system result now launches through `LaunchTarget::Capability("terminal-tool")`
- native helper methods now exist for registry-backed launches of settings, file manager, editor, nuke codes, terminal, and applications/program catalog
- the old `DesktopShellAction::OpenWindow(...)` bypass was removed from active code paths
- shared default-app builtin resolution now accepts first-party addon ids, with legacy `robco_terminal_writer` mapped to `shell.editor` for compatibility
- native and legacy document-open paths now accept `ResolvedDocumentOpen::BuiltinAddon(...)` instead of the older product-specific builtin enum variant

Additional settings-subtool addon slice:

- first-party addon runtime entries now exist for `shell.default-apps`, `shell.connections`, `shell.edit-menus`, and `shell.about`
- those addon ids now resolve to Settings subpanels instead of being metadata-only entries
- desktop launch targets now exist for `default-apps-ui`, `connections-ui`, `edit-menus-ui`, and `about-ui`
- the desktop Start-menu Connections action now launches through `LaunchTarget::Capability("connections-ui")`
- the old desktop-only `OpenConnectionsSettings` shell action was removed
- settings subtools are still hosted inside the Settings window for now, but they now have addon-backed launch identities, which is the intended migration seam

Additional install-profile policy slice:

- first-party addon enablement is now centralized in `src/native/addons.rs`
- desktop launch resolution is now profile-aware instead of assuming every first-party addon is enabled in every install profile
- a profile-filtered first-party registry now exists for runtime launch resolution
- the first concrete policy rule is intentionally narrow: `shell.connections` is disabled for `mac-launcher`, which matches the existing platform limitation there
- unresolved launch status now distinguishes between “not wired” and “disabled by install profile”
- broader menu/catalog visibility still needs to move onto this policy layer in a later step; this slice only established the core resolution seam

Additional desktop visibility-policy slice:

- first-party capability enablement helpers now exist beside addon enablement in `src/native/addons.rs`
- desktop Start System menu visibility now filters through addon capability policy instead of a local macOS-only connections exception
- desktop Start Applications builtins now respect both user visibility settings and addon capability policy
- desktop Applications window sections now rebuild against profile-aware builtin visibility, including `File Manager`
- visibility is still driven by static first-party manifests/runtime entries; no packaging or dynamic loading was introduced in this slice

Additional Spotlight/terminal visibility slice:

- desktop Spotlight still uses the shared search service, but native result filtering now removes system entries that are disabled by addon capability policy or user builtin-visibility settings
- Spotlight launch resolution for system entries now guards on capability availability instead of assuming every builtin launcher is present
- terminal Applications builtins now derive from the same effective builtin visibility used by the desktop Applications window
- terminal Edit Menus now only expose builtin toggles for addons that are actually available in the current install profile

Additional desktop Settings visibility slice:

- desktop Settings home tiles now have an explicit visibility model in `crates/native-settings-app/src/lib.rs`
- native desktop settings now derive that visibility from the same addon/profile policy used by Start, Spotlight, Applications, and terminal Settings
- disabled desktop Settings subpanels such as Connections now coerce back to the default Settings home panel instead of remaining directly targetable through stale state

Additional `app.rs` extraction slice:

- addon/profile visibility helpers and their caches were extracted from `src/native/app.rs` into `src/native/app/addon_policy.rs`
- the native app root remains behaviorally identical for these paths, but the coordinator now delegates one more cohesive responsibility area to a dedicated submodule

Additional terminal settings capability-routing slice:

- terminal Settings no longer emits hardcoded screen-specific events for addon-backed tools such as Connections, Default Apps, Edit Menus, and About
- `crates/native-settings-app/src/lib.rs` now emits capability-based terminal settings events for those addon-backed destinations
- native terminal runtime now resolves those capability requests against the first-party addon registry for the active install profile before opening a terminal screen
- this means terminal menu mode has started using the same capability contract as desktop launch paths, not just the same visibility policy

Additional shared terminal launch-registry slice:

- first-party addon runtimes in `src/native/addons.rs` now carry both desktop and terminal runtime routes instead of only desktop routes
- `src/native/app/launch_registry.rs` now resolves terminal launch targets through the same profile-aware addon/runtime registry seam as desktop
- terminal settings capability launches, terminal Applications builtin launches, and terminal main-menu launches for Settings, Program Installer, and Terminal now consume that shared terminal registry path
- terminal mode still has older direct screen routing for non-addon sections like Documents, Network, Games, and user-management flows, but addon-backed entry points are no longer maintained as a separate terminal-only lookup table

Additional launch-runtime extraction slice:

- desktop launch helpers and shell-action execution moved out of `src/native/app.rs` into `src/native/app/launch_runtime.rs`
- the shared terminal launch executor for addon-backed terminal routes moved into that same runtime module, so desktop and terminal launch behavior now live beside each other instead of being split across `app.rs` and `terminal_screens.rs`
- this does not change runtime behavior, but it gives the coordinator a cleaner seam for the later `ShellState` / runtime extraction work

Additional session-runtime extraction slice:

- session restore/reset lifecycle moved out of `src/native/app.rs` into `src/native/app/session_runtime.rs`
- that module now owns user restore, shell runtime reset, snapshot persistence, and logout reset flow
- `src/native/app/session_management.rs` still owns session switching and parked-session coordination, but it now calls into a dedicated session-runtime module instead of reaching back into a large coordinator block in `app.rs`

Additional window-runtime extraction slice:

- secondary desktop window spawning, desktop PTY window lookup, active desktop PTY access, window-title resolution, and secondary PTY cleanup moved out of `src/native/app.rs` into `src/native/app/desktop_window_mgmt.rs`
- this keeps secondary-window and embedded-PTY mechanics beside the rest of the desktop window manager instead of leaving another runtime block in the root coordinator

Additional desktop-runtime extraction slice:

- desktop standalone-window preparation and update flow moved out of `src/native/app.rs` into `src/native/app/desktop_runtime.rs`
- that module now owns profile autologin open-mode handling, standalone session restore for desktop windows, standalone Settings/Editor/Applications/Installer shell prep and repaint flow, and the unsaved-editor viewport-close interception path
- this establishes a concrete `desktop_runtime` module without changing runtime behavior

Additional terminal-runtime extraction slice:

- terminal navigation, user-management prompt handling, terminal flash queueing, terminal PTY launch/open helpers, and terminal PTY exit handling moved out of `src/native/app.rs` into `src/native/app/terminal_runtime.rs`
- `src/native/app/terminal_dispatch.rs` and `src/native/app/terminal_screens.rs` now call into a dedicated terminal runtime module instead of a large shared block on the root coordinator
- this gives the native shell parallel `desktop_runtime` and `terminal_runtime` seams, which is closer to the planned coordinator split

Additional prompt-runtime extraction slice:

- terminal prompt construction helpers, file-manager prompt bridging, and prompt/status result plumbing moved out of `src/native/app.rs` into `src/native/app/prompt_runtime.rs`
- this keeps prompt lifecycle code beside the existing prompt dispatch modules instead of leaving another utility block in the root coordinator

Additional document-runtime extraction slice:

- document open/save flow, file-manager picker flow, editor command handling, and desktop file-manager shortcut/footer handling moved out of `src/native/app.rs` into `src/native/app/document_runtime.rs`
- that module now owns the shared document/file-manager runtime seam used by both desktop and terminal paths, which keeps editor and file-manager behavior aligned instead of letting one mode drift ahead of the other
- focused regressions passed for terminal save-as prompt flow, desktop save-as picker behavior, and file-manager prompt/settings behavior after the extraction

Additional runtime-state extraction slice:

- shared cache invalidation, settings-sync, background-result handling, IPC handling, status application, and native settings persistence moved out of `src/native/app.rs` into `src/native/app/runtime_state.rs`
- that module now owns the cross-mode runtime helpers used by desktop and terminal flows, which is a better fit for the planned `CacheState` / `RuntimeState` split than keeping them in the root coordinator
- representative regressions passed for file-manager command/prompt flow, settings window reset/reopen behavior, and parked-session restore behavior after the extraction

Additional frame-runtime extraction slice:

- the per-frame shell pass moved out of `src/native/app.rs` into `src/native/app/frame_runtime.rs`, including login drawing, early desktop PTY input handling, terminal runtime drawing, and the main `update` orchestration body
- `app.rs` now keeps the `eframe::App` hook but delegates the actual frame coordination to a dedicated runtime module, which is much closer to the intended root-coordinator shape
- representative regressions passed for terminal capability routing, settings reopen behavior, and parked-session restore behavior after this extraction

Additional desktop-component-host extraction slice:

- the desktop window/component host adapter methods moved out of `src/native/app.rs` into `src/native/app/desktop_component_host.rs`
- that module now owns the mechanical bridge between desktop window hosting and the underlying component state/draw hooks, which keeps the root coordinator from carrying another large table of adapter methods
- representative regressions passed for settings reopen behavior, secondary desktop PTY spawning, and applications-window rendering behavior after the extraction

Additional UI-helper extraction slice:

- shared UI/static helper methods moved out of `src/native/app.rs` into `src/native/app/ui_helpers.rs`
- that module now owns the reusable egui helpers for SVG loading, tinted icon painting, file-manager label truncation, editor text-edit ids, and retro/settings control widgets instead of leaving that generic support code in the root coordinator
- focused regressions passed for file-manager preview scaling, file-manager label truncation, and compact desktop icon label rendering after the extraction

Additional asset-helper extraction slice:

- settings/file-manager asset helpers moved out of `src/native/app.rs` into `src/native/app/asset_helpers.rs`
- that module now owns settings panel icons, installer/game icons, file-manager row/preview icon selection, and the small file-manager selection helper methods that sit beside those assets, instead of leaving that mixed asset/support block in the root coordinator
- focused regressions passed for file-manager preview scaling, lazy SVG preview loading, and file-manager navigation/selection behavior after the extraction

Additional desktop-file-runtime extraction slice:

- desktop/file-manager surface interaction helpers moved out of `src/native/app.rs` into `src/native/app/desktop_file_runtime.rs`
- that module now owns file-manager drop handling, desktop file/folder actions, desktop surface open/delete/property helpers, file-manager command dispatch, and the generic context-menu bridge instead of leaving that interaction block in the root coordinator
- focused regressions passed for file-manager navigation/clipboard behavior and desktop file-manager reveal behavior after the extraction

Additional editor-runtime extraction slice:

- editor text-command, find/replace, save, and close-confirmation helpers moved out of `src/native/app.rs` into `src/native/app/editor_runtime.rs`
- that module now owns the shared editor helper block used by desktop presenters, desktop menus, document runtime, and save/close flows, which keeps editor behavior centralized without leaving another cohesive utility block in the root coordinator
- focused regressions passed for editor search/replace command flow, dirty-close confirmation, and save/save-as behavior after the extraction

Additional edit-menu-runtime extraction slice:

- edit-menu/program-catalog/document-category helpers moved out of `src/native/app.rs` into `src/native/app/edit_menu_runtime.rs`
- that module now owns catalog add/rename/delete handling and document-category edits used by desktop Settings, desktop surface actions, and terminal edit-menu flows, which keeps that cross-mode editing logic aligned instead of leaving it mixed into the root coordinator
- focused regressions passed for terminal edit-menu add/delete flows and cached edit-menu entry invalidation after the extraction

Additional document-browser-runtime extraction slice:

- document-browser and log helpers moved out of `src/native/app.rs` into `src/native/app/document_browser_runtime.rs`
- that module now owns document-category listing, document-browser opening, log-browser opening, log-name normalization, and log-editor launch flow used by terminal documents/logs screens and prompt handling, which keeps document browsing aligned across the terminal/menu shell without leaving another mixed helper block in the root coordinator
- focused regressions passed for terminal document-browser navigation and log creation/opening behavior after the extraction

Additional final coordinator-helper extraction slice:

- startup/repaint tracing helpers moved out of `src/native/app.rs` into `src/native/app/frame_runtime.rs`
- catalog-launch and manual-file open helpers moved into `src/native/app/launch_runtime.rs`, and terminal document-browser open-with/palette helpers moved into `src/native/app/document_runtime.rs`
- this leaves `src/native/app.rs` much closer to the intended root-coordinator role: state, default construction, a few core runtime helpers, and the test module rather than a mix of utility flows

Additional path-authority compatibility slice:

- `crates/shared/src/config.rs` now exposes explicit logical-root helpers for `state_root`, `core_root`, `system_addons_root`, `logical_user_root`, `user_addons_root`, and `cache_root` instead of only the older `base_dir()` compatibility path
- `src/native/data.rs` now resolves `journal_entries` through the compatibility state root and copies legacy log files forward from the older `base_dir()` location when needed
- this is intentionally a small first migration step: path authority is now explicit in shared config, but broader settings/user/catalog state still remains on older helpers until the next staged pass

Additional shared state-root migration slice:

- shared config state-file helpers now route through a single compatibility-aware state-root helper instead of directly joining onto `base_dir()`
- `users_dir`, fallback `desktop_dir`, `global_settings_file`, `about_file`, and non-user-scoped catalog files now resolve under `state_root` while still copying legacy files and directories forward from the older `base_dir()` location when needed
- this keeps `ROBCOS_BASE_DIR` / `NUCLEON_BASE_DIR` compatibility intact while moving more persistence logic onto the staged path-authority seam

Additional launcher bin-root slice:

- shared config now exposes `bundled_bin_dir()`, with `NUCLEON_BIN_DIR` / `ROBCOS_BIN_DIR` override support and a default of `<core_root>/bin`
- built-in game launch resolution in `crates/native-services/src/desktop_launcher_service.rs` now prefers that configured bin root before sibling-executable and dev workspace fallbacks
- native standalone app launch in `src/native/standalone_launcher.rs` now does the same, which reduces executable-relative assumptions without dropping current dev behavior

Additional session-state path slice:

- shared config now exposes compatibility-aware helpers for `.session` and `installed_package_descriptions.json`
- `crates/shared/src/core/auth.rs` now resolves the login session marker through `session_state_file()` instead of joining directly onto `base_dir()`
- this keeps the session marker on the same staged state-root migration path as settings, users, catalogs, and logs

Additional installer-cache path slice:

- `crates/native-installer-app/src/lib.rs` now resolves `installed_package_descriptions.json` through the shared compatibility helper instead of joining directly onto `base_dir()`
- this completes the current migration pass for the named legacy runtime files that were explicitly called out in the compatibility merge logic: settings, about, `.session`, installed package descriptions, users, and journal entries

Additional scoped-manifest groundwork slice:

- `crates/shared/src/platform/catalog.rs` was added as the first read-only addon manifest discovery layer for externalized metadata
- shared platform can now discover manifest JSON files from three staged roots:
  - bundled: `<core_root>/bundled-addons`
  - system: `system_addons_root`
  - user: `user_addons_root`
- the loader currently accepts direct `*.json` files in those roots and per-addon directories containing `addon.json` or `manifest.json`
- discovered manifests are forced onto the scope implied by their root, so scope comes from install location rather than trusting file contents
- shared platform now exposes a layered-registry builder that applies later-layer precedence, which is the needed seam for `bundled -> system -> user` manifest overrides during the static-to-external migration
- `src/native/addons.rs` now exposes `discovered_addon_manifest_catalog()` and `installed_addon_manifest_registry()` as metadata seams for later addon-manager work, while existing launch/runtime code still stays on the static first-party runtime registry for safety
- this intentionally does not introduce dynamic loading or change launch behavior yet; it only establishes scoped manifest discovery and layering so later stages can externalize first-party manifests without a rewrite

Additional addon install-state slice:

- `crates/shared/src/platform/state.rs` was added as the first shared addon enablement-state model
- shared config now persists addon state overrides in `addon_state.json` under the staged compatibility state root
- the current state model is intentionally narrow: it stores explicit per-addon enabled/disabled overrides keyed by addon id, and leaves package removal/uninstall workflow for a later stage
- `src/native/addons.rs` now exposes:
  - `addon_state_overrides()`
  - `effective_addon_enabled(...)`
  - `installed_enabled_addon_manifest_registry()`
- that gives native code a shared “effective enabled addon catalog” seam layered over:
  - static first-party manifests
  - discovered bundled/system/user manifests
  - persisted enable/disable overrides
- existing launch/runtime resolution still intentionally stays on the static first-party runtime registry and profile policy, so this slice does not yet change launcher behavior
- this is the intended bridge into later addon-manager/installer work: install scope and manifest discovery now exist, and enablement state now has one shared persistence file instead of scattered future flags

Additional installed-addon inventory slice:

- `src/native/addons.rs` now exposes `installed_addon_inventory()` as the first unified read-only inventory surface for addon-manager/installer UI work
- each inventory record now carries:
  - the effective manifest after static/discovered layering
  - optional manifest source path for discovered entries
  - explicit enabled override, if present
  - effective enabled state after applying overrides
- discovered manifests override static fallback manifests by addon id in the inventory, matching the same layered catalog rules already used for registry construction
- inventory ordering is now stable and presentation-friendly: display name first, then addon id
- `set_addon_enabled_override(...)` was added as the first narrow mutation seam for addon-manager UI and policy state changes

Additional installer inventory surface slice:

- the existing Program Installer now exposes an `Installed Addons` view in both terminal mode and desktop mode
- the installer surface is backed by `installed_addon_inventory()` / `installed_addon_inventory_sections()` rather than ad hoc catalog rebuilding
- each row currently shows:
  - addon display name
  - effective enabled/disabled state
  - addon scope (`bundled`, `system`, `user`)
  - subtitle/source metadata in the terminal installer view
- essential addons are now separated from optional addons in the installer inventory:
  - essential addons remain visible, but are always marked as required and cannot be disabled
  - optional addons expose enable/disable actions in both desktop and terminal installer views
- the terminal menu renderer now supports non-selectable section headers (`### ...`) so the essential/optional split can be shown cleanly without turning section labels into menu actions

Additional addon-policy adoption slice:

- first-party launcher/profile policy now consumes the effective enabled addon catalog instead of the old static first-party manifest list
- addon-state overrides are now live behavior: disabling a wired optional first-party addon removes it from the profile-aware registry used for capability visibility and launch resolution
- shell-critical first-party addons are marked essential in their manifests and ignore disable requests
- install-profile policy still applies on top of that state, so profile restrictions such as macOS Connections remain authoritative
- launch-registry status messages now distinguish between:
  - disabled by install profile
  - disabled by addon state
- runtime routing still stays on the static first-party runtime table; this slice only changed policy/availability resolution, not how actual runtime routes are implemented

Additional user-addon removal slice:

- addon lifecycle now includes one narrow removal path: discovered user-scoped addon manifests can be removed through native addon-management helpers
- removal is intentionally bounded:
  - user-scoped discovered manifests only
  - bundled addons are not removable
  - system-scoped addons are not removable in this slice
  - runtime loading remains static
- removing a user-scoped addon now:
  - deletes its discovered manifest file
  - removes now-empty parent addon directories under the user addons root
  - clears any persisted enable/disable override for that addon id
- terminal Program Installer now opens an addon action screen from the Installed Addons inventory instead of toggling state directly from the list row
- desktop Program Installer now exposes inline addon actions:
  - enable/disable for optional addons
  - remove for removable user-scoped discovered addons
  - required marker for essential addons
- this keeps desktop and terminal aligned on the same lifecycle seam without jumping to packaging or dynamic loading yet

Additional user-path helper cleanup slice:

- shared config now centralizes user-scoped settings/catalog paths behind explicit helpers instead of scattering raw `user_dir(...).join(...)` calls
- new/cleaned helper usage now covers:
  - current user catalog files (`apps.json`, `games.json`, `networks.json`, `documents.json`)
  - current settings file lookup
  - per-user settings file lookup
  - default-apps prompt marker path
- native runtime settings-sync code now uses the shared `current_settings_file()` helper instead of rebuilding the per-user settings path locally
- this slice intentionally does not change the directory layout yet; it reduces raw path joins first so later path-authority migration has fewer call sites to update

## Why This Was The Correct First Step

The current codebase already has partial module extraction under `src/native/app/`, so the highest-leverage missing piece was not another `app.rs` split in isolation.

The missing foundation was:

- neutral path modeling
- install-profile modeling
- addon/app manifests
- capability-based registry
- shell launch/action/event vocabulary

Without those contracts, later refactors would keep hardcoding product names, app names, executable-relative paths, and special-case built-ins.

## Current Architectural Reality

These are the important real-code observations from the current repo state:

1. `src/native/app.rs` is no longer the old god file, but it is still the root coordinator and still owns core state, constructor/default wiring, and the test module.
   - major runtime blocks now live in extracted modules under `src/native/app/`
   - this stage of decomposition is done enough; further extraction should now be selective, not churn for its own sake
2. Path handling is materially better, but still mixed.
   - shared config now has explicit logical-root and compatibility state-root helpers
   - several important runtime files already moved behind those helpers
   - but many callers still go through older `config.rs` wrappers rather than native code using `PlatformPaths` directly
3. First-party launch/policy behavior is much cleaner, but not fully detached.
   - capability/addon resolution is real
   - install-profile and addon enabled-state both affect availability
   - but the native shell still contains a static first-party runtime table that maps addon ids to `DesktopWindow`, `TerminalScreen`, and settings-panel routes
4. Desktop and terminal are better aligned than before, but some older terminal-specific navigation/state still exists for non-addon sections.
5. External manifest discovery exists, but runtime loading is still intentionally static.
   - scoped manifest discovery is metadata-only right now
   - first-party runtime behavior still comes from compiled code
6. Current branding and current theme assumptions still leak through product/runtime layers.
   - RobCo strings and Fallout-like naming are still present in many UI surfaces
   - current appearance system is still closer to a single built-in visual language than a neutral theme engine
7. Some direct app-specific references still exist and are expected at this stage.
   - especially editor/file-manager/document flows
   - settings subpanels still host multiple first-party tools inside shell-owned runtime

This means the next phases should continue as migration layers, not a rewrite.

## Target Architecture Summary

### Product Layering

Core platform should own:

- shell/runtime coordination
- session/user system
- terminal integration
- desktop/window management
- addon manager
- theme engine
- settings/config persistence
- permissions/capability resolution
- path abstraction
- app/addon registry

Optional first-party addons should own:

- settings UI
- file manager
- editor
- document browser
- installer UI
- games and novelty apps
- theme/content packs

Later third-party addons should use the same contract, but with reduced trust and explicit permissions.

### Filesystem Model

The logical model is:

- `core_root`
- `system_addons_root`
- `user_root`
- `user_addons_root`
- `cache_root`
- `runtime_root`

Current `ResolvedPlatformPaths` maps that model to:

- `linux-desktop`
- `windows-launcher`
- `mac-launcher`
- `portable-dev`

Important note:

- The new path layer exists, but the rest of the app still mostly uses the old `config.rs` helpers.
- Migration to `PlatformPaths` still needs to happen in later steps.

### App / Addon Model

Apps are moving toward:

- stable `id`
- manifest-backed metadata
- capability-based lookup
- file associations from metadata
- structured shell actions/events

The current manifests intentionally use static routes instead of dynamic loading.

## Detailed Phased Roadmap

## Phase 0: Contract Layer

Status: done

Objective:

- Create the neutral vocabulary for paths, install profiles, manifests, registry, and shell actions.

Exit criteria:

- Shared contracts exist in `robcos-shared`.
- First-party addon catalog exists in product layer.
- No runtime behavior changed yet.

This phase is complete.

## Phase 1: Add Runtime Adapters Without Breaking Existing Flows

Status: complete for the current shell architecture scope

Objective:

- Keep current behavior, but stop launching apps through hardcoded names where a capability can be used instead.

Concrete tasks:

1. Add a small runtime adapter in the native shell that can resolve:
   - `LaunchTarget::Addon { addon_id }`
   - `LaunchTarget::Capability { capability }`
2. Back that adapter with `first_party_addon_registry()`.
3. For now, route resolved addon ids into existing open-window/open-screen functions.
4. Start with one app only.
   - First adopted app: Settings
5. Add focused tests for registry lookup and runtime mapping.

Why Settings first:

- It is central but low-risk.
- It already exists as a distinct app/screen surface.
- It exercises the capability pattern with minimal file-association complexity.

Suggested initial output of this phase:

- one new function or module that maps addon ids/capabilities to the existing runtime open actions
- one launch path converted from app-name special casing to capability lookup

Current Phase 1 progress:

- desktop and terminal addon-backed entry points now mostly route through shared capability/addon resolution
- first-party runtime resolution is centralized in `src/native/addons.rs` + `src/native/app/launch_registry.rs`
- profile-aware availability and addon enabled-state both feed launcher policy
- addon-backed shell visibility should resolve from the same launch-target seam wherever practical, so menu/search visibility does not drift from launch behavior
- desktop payload-carrying shell actions now have a typed launch-payload seam for editor/file-manager path opens and desktop terminal-shell entry, instead of routing those cases only through ad hoc top-level shell action branches
- desktop surface file/folder activation now uses that same typed launch-payload seam for file-manager directory opens and built-in editor fallback opens
- path-open policy is now explicit: desktop-originated path opens stay desktop/windowed, terminal-originated path opens stay terminal-native unless the user explicitly launches a desktop shell/app
- open-with launches now follow the same surface policy: desktop-originated launches use desktop PTY hosting, terminal-originated launches use embedded PTY hosting
- primary PTY ownership is now explicit: embedded terminal PTYs and desktop PTY windows share launch/runtime plumbing but no longer share "is open" state implicitly
- arbitrary argv/PTy launches now resolve through shared shell-command launch helpers, so installer/custom-command/document-open callers do not each reimplement desktop-vs-terminal PTY policy
- settings panel targeting now has a typed desktop launch payload, so opening a specific settings pane no longer bypasses the shell launch-action path
- built-in editor/file-manager path openings now also have shared desktop helpers, and editor path opens have a shared active-surface helper, so path-carrying callers no longer build registry payload actions ad hoc
- terminal `F1` command UI should be treated as a window-local menu strip under the active window header, not as a global top-bar replacement; terminal editor/browser input should be inert while that local strip is open
- settings subtools now have addon identities
- unresolved status now distinguishes “not wired”, “disabled by install profile”, and “disabled by addon state”
- remaining direct paths are now mostly intentional picker/standalone host behavior rather than shell launch-policy duplication

Exit criteria:

- capability/addon routing is the default path for addon-backed surfaces
- no visible behavior regression
- desktop and terminal stay aligned for addon-backed flows
- remaining non-routed direct opens are limited to intentionally stateful picker/standalone host flows

This phase is complete for the current shell architecture scope. Any remaining direct opens here are intentional picker/standalone host behavior, not outstanding launch-routing debt.

## Phase 2: Begin Path Migration

Status: complete for the current scope

Objective:

- Move filesystem layout decisions behind `PlatformPaths` while keeping compatibility.

Concrete tasks:

1. Add a central resolver that chooses:
   - product slug
   - install profile
   - `ResolvedPlatformPaths`
2. Bridge old helpers in `crates/shared/src/config.rs` to the new path model.
3. Preserve `ROBCOS_BASE_DIR` as a temporary compatibility override.
4. Move the worst ad hoc path sites first:
   - `src/native/data.rs`
   - executable-relative runtime data logic
   - current hardcoded journal/doc directories
5. Keep compatibility shims until all important callers migrate.

Important:

- Do not delete the old helpers all at once.
- First make them derive from the new model, then migrate callers gradually.

Exit criteria:

- The logical path model is authoritative.
- Existing path helpers become wrappers or compatibility shims.

Current Phase 2 progress:

- logical roots and install-profile path mapping exist
- compatibility-aware state-root helpers exist in shared config
- named legacy runtime files moved onto the compatibility state-root seam:
  - settings
  - about
  - `.session`
  - installed package descriptions
  - users
  - journal entries
- hardcoded word-processor document storage now has a shared compatibility-aware helper rooted under per-user state paths, with lazy migration from the legacy OS Documents location
- native shell and native-services callers no longer duplicate the `ROBCO Word Processor/<user>` path layout by hand
- native shell snapshot storage now has a shared authoritative helper instead of per-caller `user_dir(...).join("native_shell.json")` logic
- file-manager trash storage now has a shared authoritative helper instead of callers building `.fm_trash` paths ad hoc
- diagnostics and PTY key-debug logs now resolve through shared path helpers instead of hardcoded `~/.local/share/...` and `/tmp/...` fallbacks in product code
- home/documents fallback resolution used by native shell and native-services now comes from shared config instead of duplicated local helpers
- installer package-description cache and native IPC socket paths now resolve through named shared helpers instead of product-local path joins
- desktop-surface path checks and bundled standalone binary paths now go through named helpers instead of shell code joining/inspecting compatibility roots directly
- `RuntimeEnvironment` now exposes named state/runtime path layouts, and the shared config wrappers are starting to collapse onto that layout instead of each path being hand-assembled independently
- user DB, per-user settings, and per-user app/game/network/document catalog files are now named state-layout paths instead of generic filename plumbing
- legacy runtime-state detection and compatibility migration now use the named state layout instead of repeating raw state-file names in migration logic
- no-user desktop and catalog fallbacks now also route through named state-layout paths instead of generic compatibility helper plumbing
- bundled binary resolution now prefers configured bin roots before sibling/dev fallbacks

Remaining future cleanup:

- additional path-domain cleanup can still happen later where it materially helps addon/content architecture work
- any further migration here should be driven by a concrete new feature or architectural target, not generic path churn

This phase is complete for the current scope. Do not reopen it without a concrete new target.

## Phase 3: Convert `src/native/app.rs` Into A Coordinator

Status: done enough for this stage

Objective:

- Reduce `src/native/app.rs` from owner-of-everything to coordinator.

Target grouped state:

- `ShellState`
- `DesktopState`
- `TerminalState`
- `AppWindowsState`
- `CacheState`
- `RuntimeState`

Target module extraction:

- `desktop_runtime.rs`
- `terminal_runtime.rs`
- `session_runtime.rs`
- window-management modules
- cache modules
- shell dispatch/action module
- helper/path modules

How to do it safely:

1. Identify state clusters already operating together inside `app.rs`.
2. Move behavior with the owning state cluster, not random utility chunks.
3. Keep `RobcoNativeApp` as the root coordinator.
4. Avoid moving app-specific logic into the shell during extraction.

Exit criteria:

- `app.rs` mostly composes modules and coordinates state.
- business rules live outside the god file.

Current Phase 3 progress:

- major runtime blocks have been extracted:
  - `addon_policy.rs`
  - `launch_runtime.rs`
  - `session_runtime.rs`
  - `desktop_runtime.rs`
  - `terminal_runtime.rs`
  - `prompt_runtime.rs`
  - `document_runtime.rs`
  - `document_browser_runtime.rs`
  - `edit_menu_runtime.rs`
  - `editor_runtime.rs`
  - `runtime_state.rs`
  - `frame_runtime.rs`
  - `desktop_component_host.rs`
  - `desktop_window_mgmt.rs`
  - `desktop_file_runtime.rs`
  - `asset_helpers.rs`
  - `ui_helpers.rs`
- `app.rs` production code is now a coordinator-sized core rather than the old all-in-one file

Not done yet:

- the named state structs from the final destination are not fully formalized
- some shell-owned host behavior still remains in the coordinator and related modules

This phase is done enough for now.

## Phase 4: Convert Built-Ins To First-Party Addons

Status: structurally well underway, but not complete

Objective:

- Stop treating built-in apps as permanently special.

Recommended migration order:

1. Settings
2. File Manager
3. Editor
4. Document Browser
5. Terminal
6. Installer
7. Programs / Default Apps / Connections / Edit Menus / About
8. Games and novelty tools

Per-app migration checklist:

1. Assign stable addon id.
2. Add or refine manifest.
3. Assign capabilities.
4. Assign permissions.
5. Add file associations where relevant.
6. Route launch through registry lookup.
7. Move app-specific state ownership out of shell if possible.
8. Add tests.

Exit criteria:

- Built-ins launch through addon ids/capabilities.
- Core knows capabilities, not app names.

Current Phase 4 progress:

- built-ins now have stable addon ids and manifests
- launcher/policy behavior is largely capability/addon based
- install-profile rules and addon-state rules now affect effective availability
- installer UI now exposes installed addons and optional enable/disable state
- installer UI now also supports user-scoped addon install paths in both terminal and desktop flows

Not done yet:

- first-party runtimes are still statically compiled and code-registered
- shell still owns the runtime mapping from addon ids to windows/screens/panels
- settings subtools are still shell-hosted panels rather than truly separate packaged runtimes
- some app-specific flows still contain direct first-party assumptions

## Phase 5: External Manifests And Scopes

Status: groundwork done, behavior stage not done

Objective:

- Move from code-only manifests to bundled/system/user manifest sources.

Concrete tasks:

1. Load manifests from:
   - bundled scope
   - system scope
   - user scope
2. Add enable/disable/remove state.
3. Preserve static fallback for first-party manifests during transition.
4. Keep actual binary loading simple.
   - manifest + known entrypoint
   - no complex dynamic plugin system yet

Exit criteria:

- runtime can enumerate addons from scoped manifests
- install/enable/disable/remove are possible without changing core contracts

Current Phase 5 progress:

- scoped manifest discovery exists for bundled/system/user roots
- layered manifest precedence exists
Current addon/runtime state:

- Optional first-party addons are no longer seeded from the built-in manifest catalog. They come from discovered installed manifests or the repository feed.
- Installer/addon-manager state is in place:
  - local manual install supports manifest paths, addon directories, `.ndpkg`, and common archive formats
  - feed-backed install/update/reinstall exists
  - discovery issues are preserved for UI visibility
- Shell-integrated external addon runtime direction:
  - prefer WASM for rich integrated addons
  - keep hosted-process support only as a secondary path
- Shared runtime pieces already exist:
  - hosted-addon contract
  - WASM addon SDK
  - native WASM host with bundle-relative module resolution
  - keyboard input forwarding into hosted addons
  - real image loading from addon bundle assets
  - aspect-preserving and tint-aware hosted rendering
- installed optional addon apps/games now show up through normal shell surfaces:
  - terminal Applications / Games
  - desktop Start menu
  - Spotlight
- desktop and terminal launch paths now intercept installed WASM addon manifests directly instead of only hardcoded runtime-known apps
- hosted addon desktop windows now use the addon title instead of the generic `PTY App` fallback
- the first real host-context bridge exists:
  - shell host can read addon-bundle context files
  - host can fetch native data and pass it into the WASM guest as JSON context on init/update
  - guest refresh can request a host-side refresh cycle
- Optional addons are hosted in the external addon repo as `.ndpkg` packages and built from that repo’s `sources/` workspace, not from core-owned addon crates

## Phase 5 Progress — `.ndpkg` Packaging & External Addon Pipeline (DONE)

All packaging decisions from the prior handoff have been resolved and implemented:

### What was done

1. **`.ndpkg` format defined and implemented:**
   - `.ndpkg` = renamed ZIP archive. Uses the existing `zip = "2"` crate for extraction.
   - Internal layout: single subdirectory containing `manifest.json`, optional `addon.wasm`, optional `assets/`.
   - Added to all 5 dispatch points in `src/native/addons.rs`: archive detection, extraction routing, format whitelist, staging copy, file destination naming.
   - New test: `install_repository_addon_from_index_installs_ndpkg_bundle`.

2. **External addon repository is the source of truth:**
   - Repo: external first-party addon repository (currently `github.com/Stylith/nucleon-desktop-addons`, planned rename: `nucleon-core-addons`)
   - Layout: `index.json` at root, `.ndpkg` files in category folders (`games/`, `tools/`)
   - `index.json` has a `base_url` pointing at the hosted raw content root
   - Artifact URLs are relative to `base_url`

3. **Staging copy removed from RobCoUnified:**
   - `packaging/first-party-addons-repo/` deleted entirely
   - Test converted from `include_str!` against the staging file to inline JSON test data
   - No addon package data remains in RobCoUnified

4. **Remote index auto-fetch implemented:**
   - `spawn_addon_repository_index_refresh()` in `config.rs` runs curl in a background thread at app startup
   - Downloads `index.json` from the external addons repo to the local cache directory
   - 10-minute cache TTL to avoid hammering GitHub
   - `load_addon_repository_index()` reads from cache first, falls back to bundled index
   - Completely non-blocking — UI never waits on network

5. **HTTP download pipeline for relative artifact URLs:**
   - `stage_repository_artifact()` now calls `resolve_repository_url()` before checking `looks_like_http_url()`
   - A relative URL gets `base_url` prepended → full HTTPS URL → curl download
   - SHA-256 verification, extraction, and install all work end-to-end

6. **Packaging result:**
   - `.ndpkg` is practical enough for public feed distribution, but large binary artifacts should ultimately live in release assets rather than git history

### What's in progress — Hardcoded Addon Removal / Runtime Completion

The external pipeline is done. The remaining Phase 5 work is removing all hardcoded references to optional addons from the shell codebase so they load purely through the dynamic addon runtime.

Current practical priority order:

1. hardcoded shell removal
   - the runtime path is now complete for all three optional addons
   - next work is deleting addon-specific ownership from the core repo
2. repo/name migration
   - main repo is intended to become `nucleon-core`
   - external addon repo is intended to become `nucleon-core-addons`
   - built-in shell modules/addons should move toward the `nucleon-` prefix (`nucleon-text`, `nucleon-files`, `nucleon-extension`, etc.)

Current decoupling status:
- `src/native/wasm_addon_runtime.rs` no longer imports an addon-specific crate directly.
- The WASM host now loads host context generically from bundle files:
  - `host-context.json` if present
  - legacy `providers.json` fallback during transition
- `crates/native-services/src/desktop_launcher_service.rs` no longer hardcodes built-in optional game targets.
- `crates/native-services/src/desktop_search_service.rs` now reads game entries from the catalog instead of built-in game constants.
- The stale built-in optional-addon settings toggle was removed from the active settings UI.
- Terminal Games no longer has a separate `RobCo Fun` / `GamesRobcoFun` submenu path.
- Desktop Start -> Games is now one flat generic game list built from:
  - installed hosted game addons
  - configured catalog games
- Terminal Games now uses that same merged game list model.
- `crates/native-services/src/desktop_surface_service.rs` no longer reserves a special shortcut rank for `nuke_codes`.
- External addon source/build now has a real home in the addon repo:
  - the addon repo contains `sources/`
  - `sources/` contains buildable Rust workspaces for current optional addons plus the generic hosted-addon SDK/contract copy needed by those builds
- Core workspace membership no longer includes the addon-specific source/build crates.
- The six addon-specific source/build directories were deleted from this repo after that external workspace was verified:
  - native optional addon crates
  - wasm optional addon crates

Current biggest remaining leaks:
- optional addon package publishing still needs to switch fully onto the external repo workflow
- the host-context bridge is generic at the file-loading layer, but the data model still needs to be generalized further so the core does not carry addon-shaped host data assumptions long-term

**If continuing this work, proceed in this order:**

Phase A — Move addon source/build ownership out of the core repo:
- move the actual source/assets/build scripts for optional addons into the external addons repo (or per-addon repos if desired later)
- make `nucleon-core-addons` own the `.wasm` / `.ndpkg` build pipeline
- after the external repo can build and publish those packages independently, delete the matching workspace crates from this repo

Phase B — Remove workspace crates from core after external ownership exists:
- status: complete
- deleted from this repo:
  - native optional addon crates
  - wasm optional addon crates
- removed from root `Cargo.toml` workspace members/default-members

Phase C — Remove enum variants:
- `DesktopWindow::RedMenace`, `::ZetaInvaders`, `::NukeCodes` in `crates/native-services/src/shared_types.rs`
- `TerminalScreen::RedMenace`, `::ZetaInvaders`, `::NukeCodes` in same file
- `DesktopBuiltinIconKind::NukeCodes` in `crates/native-services/src/desktop_surface_service.rs`
- Fix all cascading match arm compile errors across the codebase

Phase D — Remove app struct fields and draw functions:
- `RedMenaceWindow`, `ZetaInvadersWindow` structs and all associated fields on `RobcoNativeApp`
- `desktop_nuke_codes_open`, `desktop_nuke_codes_wasm`, `terminal_nuke_codes`, `terminal_nuke_codes_wasm`
- `desktop_zeta_invaders_wasm`, `terminal_zeta_invaders_wasm`
- `icon_nuke_codes`, `show_nuke_codes`
- Draw functions: `draw_red_menace_window`, `draw_zeta_invaders_window`, `draw_nuke_codes_window`
- Terminal draw functions: `draw_terminal_red_menace`, `draw_terminal_zeta_invaders`, `draw_terminal_nuke_codes`
- Reset functions: `reset_zeta_invaders_runtime`, `reset_red_menace_runtime`, `reset_nuke_codes_wasm_runtime`
- WASM detection: `zeta_invaders_uses_wasm_addon`, `nuke_codes_uses_wasm_addon`
- Desktop component host registrations in `desktop_component_host.rs` and `desktop_app.rs`

Phase E — Remove launch, menu, search, settings, session references:
- Launch mappings in `name_to_desktop_window()`, `name_to_terminal_screen()`
- Remaining `BUILTIN_NUKE_CODES_APP`, `BUILTIN_RED_MENACE_GAME`, `BUILTIN_ZETA_INVADERS_GAME` constants
- Start menu entries, spotlight entries, search service/system references that still mention these specific addons
- Programs app `OpenNukeCodes` action
- Settings visibility toggles in `config.rs` and `addon_policy.rs`
- Session save/restore fields in `session_management.rs` and `session_runtime.rs`
- Desktop icon registration in `desktop_surface.rs`

Phase F — Clean up tests:
- Remove game state tests in `app.rs`
- Remove mock addon fixtures in `addons.rs` that reference these specific addons
- Keep the generic addon install/inventory tests

**Critical rule:** After each phase, `cargo check --bin robcos-native` must compile. Fix cascading errors before moving to the next phase.

**What must NOT change:** The WASM addon runtime (`wasm_addon_runtime.rs`, `hosted_addon_runtime.rs`), the addon installer UI, the addon discovery/registry system — these are the generic infrastructure that will load these addons dynamically after the hardcoded paths are removed.

## Phase 6: Third-Party Addons

Status: last

Objective:

- Open the same model to external addons with explicit trust and permissions.

Do not start this early.

The codebase should already have:

- stable manifest schema
- stable path model
- stable registry rules
- stable permission flow

## Proposed Module Tree After Refactor

This is the intended destination, not the current repo state.

```text
crates/shared/src/
  platform/
    mod.rs
    profile.rs
    paths.rs
    addons.rs
    registry.rs
    shell.rs

src/native/
  addons.rs
  runtime/
    coordinator.rs
    desktop_runtime.rs
    terminal_runtime.rs
    session_runtime.rs
    app_windows.rs
    cache_runtime.rs
    shell_dispatch.rs
    paths.rs
  windows/
    settings_window.rs
    file_manager_window.rs
    editor_window.rs
    document_browser_window.rs
```

Important note:

- The exact tree can vary, but the ownership pattern matters more than the folder names.

## What Must Remain In Core

Keep in core:

- app/addon registry
- install-profile selection
- path abstraction
- shell action/event dispatch
- session and user model
- terminal runtime
- desktop/window lifecycle
- addon permission/trust handling
- theme engine framework
- settings/config persistence substrate

Do not keep as hardwired core features forever:

- settings UI
- file manager UI
- editor UI
- document browser UI
- default apps UI
- connections UI
- novelty apps and games
- branded content/theme packs

## Architectural Traps To Avoid

1. Replacing app-name hardcoding with addon-id hardcoding in core

Core should resolve by capability whenever possible. Addon ids are still useful, but not as a new universal special-case key.

2. Moving faster on `app.rs` splitting than on contract adoption

If the runtime still uses old path/app assumptions, a prettier module tree will not solve the real coupling.

3. Migrating paths by search-and-replace

The current repo mixes:

- `config.rs` path helpers
- direct `dirs::*`
- current-exe-relative logic
- `CARGO_MANIFEST_DIR` fallbacks

This needs deliberate adaptation, not blind substitution.

4. Introducing dynamic plugin loading too early

Registry + manifests + packaging can go far without dynamic libraries or complex hot-loading.

5. Letting Linux full-environment assumptions leak into Windows/mac launcher code

All three platforms should share one runtime model, with different install-profile resolution and integration level.

## Recommended Next Task

Complete the hardcoded addon removal (Phases A–E described in "Phase 5 Progress" above).

After that:
1. Verify WASM addon runtime discovers and launches all three optional addons from the external repo
2. Continue to Phase 6 (branding/theme extraction) or Phase 7 (third-party addon model)
3. Keep desktop and terminal addon behavior aligned
4. Do not introduce native dynamic library plugin loading

## Suggested Continuation Steps For The Next Session

When resuming:

1. checkout branch `WIP`
2. re-run:
   - `cargo test -p robcos-shared platform`
   - `cargo check --bin robcos-native`
3. inspect:
   - `src/native/addons.rs`
   - `src/native/app/launch_registry.rs`
   - `src/native/installer_screen.rs`
   - `src/native/app/desktop_installer_ui.rs`
   - `crates/shared/src/platform/catalog.rs`
   - `crates/shared/src/platform/state.rs`
4. confirm the current essential-addon rule before changing any addon-manager behavior
5. choose one next bounded slice:
   - user-scoped addon install/remove behavior
   - or remaining path-authority cleanup
6. keep runtime loading static
7. verify desktop and terminal stay aligned

## Suggested Prompt For The Next Session

Copy this into the next session after pointing it at this repo:

```text
You are continuing the neutral-core/addon refactor on branch WIP.

Read docs/NEUTRAL_CORE_HANDOFF.md first — specifically the "Phase 5 Progress" section.

Current state:
- .ndpkg packaging is DONE. External addon repo pipeline is DONE.
- Optional addons are hosted externally in the addon repo as `.ndpkg` packages.
- The app auto-fetches index.json from the external repo at startup and can download/install addons via curl.
- The staging copy (packaging/first-party-addons-repo/) has been removed from RobCoUnified.

Remaining work: remove all hardcoded references to these three addons from the shell.
Follow Phases A–E in the handoff doc. After each phase, cargo check --bin robcos-native must compile.

Do NOT touch: WASM addon runtime, addon installer UI, addon discovery/registry system.
Do NOT introduce native dynamic library plugin loading.
```

## Reference Files To Inspect Next

High priority:

- `crates/shared/src/platform/mod.rs`
- `crates/shared/src/platform/profile.rs`
- `crates/shared/src/platform/paths.rs`
- `crates/shared/src/platform/addons.rs`
- `crates/shared/src/platform/registry.rs`
- `crates/shared/src/platform/shell.rs`
- `src/native/addons.rs`
- `src/native/app.rs`
- `src/native/desktop_app.rs`

Path migration targets:

- `crates/shared/src/config.rs`
- `src/native/data.rs`
- `crates/native-services/src/desktop_launcher_service.rs`
- `src/native/standalone_launcher.rs`

## Final Notes

The important principle is sequence:

1. contracts
2. adapters
3. path authority
4. runtime decomposition
5. app migration
6. manifest externalization
7. packaging and third-party support

Do not skip ahead. The architecture will be cleaner if the migration order stays disciplined.

Also keep these additional realities in mind:

- the codebase is not yet 100% separated; static first-party runtime routing still exists
- there are still shell-owned window/screen/panel enums that know about current first-party addons
- theme and branding extraction are still ahead of us
- future shell-composition work should separate:
  - render mode (`monochrome` vs `color`)
  - color themes
  - per-mode layout profiles
  - shell components (`dock`, `taskbar`, `top menu`, `panel`, `window header`, `launcher`)
  - skins/assets for those components (`macos dock skin`, `windows95 window skin`, icon packs, cursor packs)
  - capability/schema-driven settings for active components and skins
- avoid treating all of the above as one flat `theme` object; that will collapse structure, behavior, and presentation back into hardcoded conditionals
