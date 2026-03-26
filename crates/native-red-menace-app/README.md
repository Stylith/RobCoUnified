# Native Red Menace App

Embeddable `egui` game scaffold for a Red Menace port in RobCoOS.

- Library crate plus standalone preview binary
- Host-driven update and draw flow through `egui`
- Flash-style state flow scaffold: title, intro, transition, level, game over
- Progression model based on the decompiled Flash sources
- Stage 1 is now a playable Rust gameplay slice with Flash-extracted PNG sprites for the hero, boss, girl, bombs, helmet, lives, and pause icon
- Stage 2 and Stage 3 are still explicit placeholders

Minimal host flow:

```rust
let mut game = RedMenaceGame::new(RedMenaceConfig::default());
let input = input_from_ctx(ctx);
game.update(&input, dt_seconds);
game.draw(ui);
```

Standalone preview:

```bash
cargo run -p robcos-native-red-menace-app --bin robcos-red-menace
```

Current controls:

- `Enter` or `Space` advances from title / intro / transition / game over
- In Stage 1, `Left` / `A` and `Right` / `D` move
- In Stage 1, `Up` / `W` and `Down` / `S` climb ladders
- In Stage 1, `Space` jumps
- In Stage 2 and Stage 3 placeholders, `Space` clears the stage and `Enter` loses a life

Source notes:

- The current scaffold is aligned to the decompiled `bhvr/*` state flow from the Flash project.
- The current progression constants in the scaffold are now backed by `RedMenaceConfig.xml`.
- Stage 1 now uses the decompiled timer and bomb formulas from `LevelUpVariables` and the XML config.
- The vendored Stage 1 PNGs live under `assets/png/` and are trimmed at load time so the original Flash symbol padding does not distort layout.
