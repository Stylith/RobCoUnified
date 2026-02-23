# Sound Files

Drop your audio files here (WAV, MP3, or OGG/Vorbis format):

| Filename        | When it plays                        |
|-----------------|--------------------------------------|
| startup.wav     | During the boot animation            |
| login.wav       | After a successful login             |
| error.wav       | On authentication failure            |
| select.wav      | When selecting a menu item           |
| keypress.wav    | On keystrokes (optional/lightweight) |

Then open `src/sound.rs` and uncomment the matching `include_bytes!(...)` line
for each file you added. The file is then compiled directly into the binary —
no external assets needed at runtime.

Example (in sound.rs):
```rust
Sound::Login => Some(include_bytes!("sounds/login.wav")),
```
