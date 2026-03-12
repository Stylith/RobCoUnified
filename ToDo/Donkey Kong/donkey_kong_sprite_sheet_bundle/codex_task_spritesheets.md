# Codex Task: Port MasterCash/donkey-kong to an Embeddable Rust Library

Repository:
https://github.com/MasterCash/donkey-kong

## Non-negotiable requirements

1. Make it a library crate, not a standalone executable.
2. Do NOT use `#[macroquad::main]`.
3. Do NOT assume control of the host app main loop.
4. Preserve the sprite-sheet / atlas model used by the Python repo.
5. Support monochrome sprite sheets with runtime tinting from the host app theme.
6. Centralize atlas, frame, and animation metadata.
7. Keep the implementation beginner-friendly.
8. Output complete files, not fragments.

## Required public API

```rust
pub struct DonkeyKongConfig {
    pub scale: f32,
    pub theme: Theme,
}

pub struct Theme {
    pub primary: Color,
    pub enemy: Color,
    pub ui: Color,
    pub neutral: Color,
}

pub struct DonkeyKongGame {
    // internal state
}

impl DonkeyKongGame {
    pub async fn new(config: DonkeyKongConfig) -> Self;
    pub fn update(&mut self);
    pub fn draw(&self);
    pub fn set_theme(&mut self, theme: Theme);
}
```

## Required asset system

Use:
- `AtlasId`
- `FrameId`
- `AnimationId`
- `Frame`
- `SpriteCatalog`
- metadata JSON files

## Build pipeline

Implement `build.rs` to convert SVG sheets in `assets/svg/` into PNG sheets in `assets/png/`.
