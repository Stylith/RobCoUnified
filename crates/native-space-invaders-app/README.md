# Native Zeta Invaders App

Embeddable `egui` Zeta Invaders game logic for RobCoOS.

- Library crate plus standalone preview binary
- One PNG file per animation frame under `assets/png/`
- Runtime `egui` tinting defaults to green
- PNG assets are loaded from disk first on startup
- Rendering is host-driven through `egui`
- Barn cover now uses Zeta-style piece slots: `barn_piece_00.png` through `barn_piece_17.png`

Replace any placeholder frame by editing the matching PNG file in `assets/png/`, then relaunch the app. No `target/` cleanup should be needed.

Title screen:

- `title_01.png` through `title_20.png` are the 20 animated title frames
- the game starts on the title screen
- press `Enter` to begin

Minimal host flow:

```rust
let mut game = SpaceInvadersGame::new(SpaceInvadersConfig::default());
let atlas = AtlasTextures::new(ctx);

let input = input_from_ctx(ctx);
game.update(&input, dt_seconds);
game.draw(ui, &atlas);
```

Standalone preview:

```bash
cargo run -p robcos-native-zeta-invaders-app --bin robcos-zeta-invaders
```

Controls:

- `Enter` to start from the title screen
- `Left` / `A` and `Right` / `D` to move
- `Space` to fire
- `P` or `Esc` to pause and resume during play
