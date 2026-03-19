# Phase 0: Pre-Migration Cleanup — Codex Task

**Context:** This project (RobCoOS) is preparing to migrate its UI framework from egui to iced. Before that migration begins, we need to clean up the codebase. This document describes four independent cleanup tasks. Each can be done separately and verified with `cargo check` and `cargo test`.

**Important constraints:**
- Do NOT touch anything related to egui or iced. This phase is framework-agnostic.
- Do NOT change any UI behavior or visual appearance.
- Every change must compile cleanly (`cargo check`) with zero errors.
- Run `cargo test` after each task to verify no regressions.
- The 20 pre-existing test failures in `native::app::tests` exist on `main` before any changes — they are the baseline. Do not introduce NEW failures.

---

## Task 1: Archive legacy shell to a branch

The `crates/legacy-shell/` directory and `src/legacy/` directory contain ~22,700 lines of an old ratatui-based terminal UI implementation. It is no longer actively developed and exists only as a reference. Archive it to a Git branch and remove it from the main codebase.

### Steps:

1. Create a branch called `legacy` from the current `main` HEAD:
   ```
   git branch legacy
   ```

2. On `main`, remove the following:
   - Delete the directory `crates/legacy-shell/` entirely
   - Delete the directory `src/legacy/` entirely

3. Update `Cargo.toml` (root workspace manifest):
   - Remove `"crates/legacy-shell"` from the `[workspace] members` array
   - Remove any workspace dependency entries that are ONLY used by legacy-shell (check before removing — if another crate also uses the dependency, keep it)

4. Update `src/lib.rs`:
   - Remove `pub mod legacy;` or any module declaration that references the legacy module
   - Remove any `#[cfg]` blocks or feature flags that gate legacy-only code

5. Search the entire codebase for any remaining references to "legacy" in Rust source files:
   ```
   grep -r "legacy" src/ crates/ --include="*.rs" -l
   ```
   - Remove or update any imports, `use` statements, or `mod` declarations that reference removed legacy code
   - Do NOT remove comments or docs that merely mention "legacy" as a concept

6. Verify:
   ```
   cargo check
   cargo test
   cargo build --release -p robcos-native-shell --bin robcos-native
   ```

7. Commit with message: `Archive legacy shell to legacy branch and remove from main`

---

## Task 2: Fix or delete the 20 failing tests

There are 20 tests in `src/native/app.rs` (in the `mod tests` block near the end of the file) that consistently fail when run together but pass individually. This is caused by shared mutable global state between tests.

### Diagnosis:

Run:
```
cargo test -p robcos --lib -- --test-threads=1
```

If all 20 pass with `--test-threads=1`, the problem is test isolation — tests are sharing global state and interfering with each other when run in parallel.

### Fix approach:

1. Look at the test module in `src/native/app.rs` (near line 9698). There's already a `session_test_guard()` function that uses a `Mutex` to serialize tests. The failing tests likely aren't using this guard.

2. For each of the 20 failing tests, ensure they:
   - Call `session_test_guard()` at the start and hold the guard for the duration of the test
   - Clean up any global state they modify (check for global/static variables in the test helpers)

3. If a test is testing obsolete behavior (references functions or types that no longer exist), delete it.

4. Verify:
   ```
   cargo test -p robcos --lib
   ```
   All tests should pass (target: 149 pass, 0 fail).

5. Commit with message: `Fix test isolation for native app tests`

---

## Task 3: Build-time SVG rasterization for built-in icons

The project already pre-renders Donkey Kong SVG sprites to PNG at build time in `build.rs`. Extend this pattern to pre-render all built-in SVG icons so they don't need runtime SVG rasterization.

### Steps:

1. Read `build.rs` at the project root to understand the existing pattern. It uses `resvg` and `tiny-skia` to render SVG files to PNG and writes them to `OUT_DIR`.

2. Find all built-in SVG icon files:
   ```
   find assets/ -name "*.svg" -not -path "*/donkey_kong/*"
   ```
   These are the icons used by the file manager, desktop, settings, and other built-in UI.

3. In `build.rs`, add a new function (similar to the Donkey Kong rendering) that:
   - Reads each built-in SVG
   - Renders it to PNG at sizes: 16, 24, 32, 48, 64 pixels (square)
   - Writes to `OUT_DIR` with naming convention: `icon_{name}_{size}.png`
   - Adds `println!("cargo::rerun-if-changed=...")` for each SVG source

4. Create a new file `src/native/builtin_icons.rs` (or similar) that:
   - Uses `include_bytes!()` to embed the pre-rendered PNGs
   - Provides a function to look up an icon by name and size
   - Example:
     ```rust
     pub fn builtin_icon(name: &str, size: u16) -> Option<&'static [u8]> {
         match (name, size) {
             ("file_manager", 32) => Some(include_bytes!(concat!(env!("OUT_DIR"), "/icon_file_manager_32.png"))),
             // ...
         }
     }
     ```

5. Do NOT yet change any runtime icon loading code to use these — that will happen during the iced migration. Just make the pre-rendered icons available.

6. Verify:
   ```
   cargo check
   cargo build --release -p robcos-native-shell --bin robcos-native
   ```

7. Commit with message: `Pre-render built-in SVG icons at build time`

---

## Task 4: Add LRU bounds to caches

Several caches in the codebase use unbounded `HashMap`s that grow indefinitely during long-running sessions. Add LRU eviction.

### Steps:

1. Add `lru = "0.12"` to the `[workspace.dependencies]` section of the root `Cargo.toml`.

2. Add `lru = { workspace = true }` to the `[dependencies]` section of any crate that uses the caches being modified (likely `robcos` in the root or `crates/native-shell`).

3. Find the cache structs. Search for `AssetCache` in `src/native/app.rs`:
   - `AssetCache` struct — likely has `HashMap` fields for textures, SVG icons, etc.
   - `shortcut_icon_cache` field on `RobcoNativeApp`
   - Any other `HashMap` fields that are used as caches (populated lazily, never cleared)

4. Replace the `HashMap` in each cache with `lru::LruCache`:
   - `LruCache::new(NonZeroUsize::new(256).unwrap())` for icon/texture caches
   - `LruCache::new(NonZeroUsize::new(64).unwrap())` for less frequently used caches
   - Note: `LruCache::get()` takes `&mut self` (it updates access order), so some method signatures may need to change from `&self` to `&mut self`

5. Update all call sites:
   - `cache.get(&key)` → `cache.get(&key)` (same API but now `&mut self`)
   - `cache.insert(key, value)` → `cache.put(key, value)` (LruCache uses `put`, not `insert`)
   - `cache.contains_key(&key)` → `cache.contains(&key)`
   - `cache.remove(&key)` → `cache.pop(&key)`

6. Verify:
   ```
   cargo check
   cargo test
   ```

7. Commit with message: `Add LRU eviction bounds to asset and icon caches`

---

## General Notes

- The project uses Rust 2021 edition.
- Workspace structure: root `Cargo.toml` defines shared dependencies, individual crates in `crates/` reference them with `{ workspace = true }`.
- The main binary is built with: `cargo build -p robcos-native-shell --bin robcos-native`
- Tests are run with: `cargo test` (all) or `cargo test -p robcos --lib` (main crate only)
- Read `docs/PROJECT_CONTEXT_FOR_LLM.md` and `docs/START_HERE_FOR_LLM.md` for full project context.
- Read `CLAUDE.md` at the project root for coding conventions.

## Task Order

These tasks are independent and can be done in any order. Recommended order:
1. Archive legacy (biggest impact on build times, simplifies everything else)
2. Fix tests (establishes a green baseline)
3. LRU caches (smallest, self-contained)
4. Build-time SVGs (most complex, can be skipped if time-constrained)
