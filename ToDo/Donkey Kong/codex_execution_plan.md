# Codex Execution Plan

Project:
https://github.com/KendraTam8/DonkeyKong/tree/master

Purpose:
Port the Pygame Donkey Kong clone into an embeddable Rust library module, not a standalone executable.

## Mission

Work in small, reliable stages. Do not try to produce a giant one-shot rewrite without structure.

The final result should be:
- a Rust library crate
- embeddable inside a larger app
- based on macroquad
- using monochrome SVG source assets
- using PNG runtime textures
- supporting runtime theme tinting
- using a centralized sprite registry

## Global Constraints

- Do NOT use `#[macroquad::main]`
- Do NOT assume ownership of the host app main loop
- Do NOT hardcode colors into sprites
- Do NOT scatter asset paths around the codebase
- Keep the code beginner-friendly
- Prefer explicit structs and straightforward logic
- Avoid ECS and overengineering

## Recommended Order of Work

### Phase 1: Analyze the original Pygame repo

Read the Python source and identify:
- player movement logic
- jumping and gravity behavior
- ladder climbing logic
- platform collision rules
- barrel spawning and movement
- score/lives flow
- win/game-over flow
- all image and sound assets currently used

Deliverable for this phase:
- a short migration summary
- a list of game systems
- a list of required assets
- a list of assumptions

### Phase 2: Create crate scaffolding

Create the Rust library layout.

Minimum expected files:
- `Cargo.toml`
- `build.rs`
- `src/lib.rs`
- `src/game.rs`
- `src/assets.rs`
- `src/collision.rs`

Optional folders:
- `src/entities/`
- `src/level/`

Deliverable for this phase:
- compile-ready crate skeleton
- public `DonkeyKongGame` API
- minimal config/theme types

### Phase 3: Define public API for embedding

Create a host-friendly surface like:

```rust
pub struct DonkeyKongConfig {
    pub scale: f32,
    pub theme: Theme,
}

pub struct Theme {
    pub primary: Color,
    pub enemy: Color,
    pub ui: Color,
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

The host app should only need to:
- construct config
- initialize the game
- call update
- call draw
- optionally change theme

Deliverable for this phase:
- stable public API
- clear separation between host app and module internals

### Phase 4: Build asset pipeline

Create `build.rs` to convert SVG source files into PNG runtime textures.

Input:
- `assets/svg/*.svg`

Output:
- `assets/png/*.png`

Suggested crates:
- `resvg`
- `usvg`
- `tiny-skia`

Keep the converter simple and predictable.

Deliverable for this phase:
- working SVG→PNG conversion
- folder conventions
- documented assumptions about sprite export size

### Phase 5: Create sprite registry

Implement a centralized sprite registry.

Recommended shape:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpriteId {
    MarioIdle,
    MarioJump,
    MarioClimb,
    DonkeyIdle,
    Barrel1,
    Barrel2,
    Ladder,
    Girder,
    Princess,
    Heart,
    Background,
}
```

Map every sprite ID to a file path in one place only.

Create an `Assets` registry that loads and stores textures by `SpriteId`.

Optional but preferred:
- `TintRole`
- `SpriteMeta`
- default sprite size metadata

Deliverable for this phase:
- no scattered path strings
- one authoritative asset lookup path
- simple texture loading API

### Phase 6: Implement static level geometry

Recreate:
- platforms
- ladders
- spawn positions
- goal position

Do this with explicit structs and lists, not a complicated level format.

Deliverable for this phase:
- visible level layout
- collision-ready geometry
- player start point and win target

### Phase 7: Implement player movement

Port:
- left/right movement
- jump
- gravity
- falling
- ladder climbing
- state switching between idle/jump/climb sprites

Keep collision dimensions separate from texture dimensions.

Deliverable for this phase:
- playable character controller
- simple, readable movement code
- approximate fidelity to original feel

### Phase 8: Implement collision system

Support:
- player vs platforms
- player vs ladders
- player vs hazards
- floor landing
- falling off edges if appropriate

Keep collision helpers centralized in `collision.rs`.

Deliverable for this phase:
- reliable gameplay collision behavior
- simple helper functions
- minimal duplication

### Phase 9: Implement barrels and hazards

Port:
- barrel spawning
- barrel movement across platforms
- descent or transition logic as needed
- player hit detection

Use a straightforward `Vec<Barrel>` update loop.

Deliverable for this phase:
- hazards on screen
- player can lose lives from collisions
- recognizable Donkey Kong pressure

### Phase 10: Implement game flow

Add:
- score
- lives
- reset after death
- game over state
- win state
- simple UI text

Tint UI to match the host theme where appropriate.

Deliverable for this phase:
- complete start-to-finish gameplay loop
- score/lives visible
- win and lose states handled

### Phase 11: Apply monochrome theme tinting

All sprites are monochrome source art.

At draw time, tint by theme.

Expected pattern:
- player uses `theme.primary`
- hazards/enemies use `theme.enemy`
- hearts/text/UI use `theme.ui`
- neutral/background can use a neutral tint if needed

Deliverable for this phase:
- no color baked into sprite assumptions
- theme can change without asset replacement
- `set_theme` updates rendering behavior cleanly

### Phase 12: Integration notes and cleanup

At the end, provide:
- complete file contents
- a short host integration example
- compile assumptions
- missing pieces or likely manual fixes
- notes on where behavior may differ slightly from Pygame

Deliverable for this phase:
- handoff-ready code
- clear explanation for the host app developer

## Required Output Style

When generating code:
- output complete files
- avoid partial snippets unless explaining a choice
- keep comments practical
- note anything uncertain directly

## Quality Standard

Prefer:
- code that likely compiles after small fixes
over:
- a more ambitious but fragile rewrite

Prefer:
- a simple direct implementation
over:
- highly abstract architecture

If unsure, choose the simplest practical behavior and explain it.
