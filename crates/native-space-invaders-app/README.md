# Native Space Invaders App

Embeddable `egui` Space Invaders game logic for RobCoOS.

- Library crate plus standalone preview binary
- One PNG file per animation frame under `assets/png/`
- Runtime `egui` tinting defaults to green
- PNG assets are loaded from disk first on startup
- Rendering is host-driven through `egui`

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
cargo run -p robcos-native-space-invaders-app --bin robcos-space-invaders
```
