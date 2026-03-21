# Native Space Invaders App

Embeddable `egui` Space Invaders game logic for RobCoOS.

- Library crate plus standalone preview binary
- One PNG file per animation frame under `assets/png/`
- Runtime `egui` tinting defaults to green
- Rendering is host-driven through `egui`

Replace any placeholder frame by editing the matching PNG file in `assets/png/`.

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
