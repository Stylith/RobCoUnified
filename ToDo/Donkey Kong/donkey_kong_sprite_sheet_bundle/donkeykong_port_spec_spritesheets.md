# Donkey Kong Pygame → Rust Port Specification (Sprite Sheet Version)

Repository:
https://github.com/MasterCash/donkey-kong

Goal: Port this Pygame Donkey Kong project to Rust as an embeddable library module, not a standalone executable.

## Key architectural decisions

- Use a Rust library crate
- Do NOT use `#[macroquad::main]`
- Host app controls the main loop
- Use macroquad
- Preserve the repo's sprite-sheet / atlas model
- Use monochrome sprite sheets with runtime tinting from the host app theme
- Centralize frame, atlas, and animation metadata

## Public API

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
    state: GameState,
    assets: Assets,
    catalog: SpriteCatalog,
    theme: Theme,
}

impl DonkeyKongGame {
    pub async fn new(config: DonkeyKongConfig) -> Self;
    pub fn update(&mut self);
    pub fn draw(&self);
    pub fn set_theme(&mut self, theme: Theme);
}
```

## Asset model

Use sprite sheets, not one image per sprite. Keep art monochrome and tint at draw time.

## Asset layout

```text
assets/
  svg/
    mario_sheet.svg
    barrels_sheet.svg
    level_sheet.svg
    ui_sheet.svg
    effects_sheet.svg
  png/
    mario_sheet.png
    barrels_sheet.png
    level_sheet.png
    ui_sheet.png
    effects_sheet.png
  meta/
    mario_sheet.json
    barrels_sheet.json
    level_sheet.json
    ui_sheet.json
    effects_sheet.json
```

## Preserve these systems

- player movement
- jump and gravity
- ladders
- platform collisions
- barrel motion
- score
- lives
- win and game-over flow
- tick-based animation sequencing
