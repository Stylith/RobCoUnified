# crt_shader.wgsl usage notes

Suggested starting uniform values for a Nucleon-style preset:

- curvature: 0.06
- scanlines: 0.28
- glow: 0.22
- vignette: 0.18
- noise: 0.03
- brightness: 1.0
- contrast: 1.08
- phosphor_softness: 0.12

Important implementation notes:

1. Render egui to an offscreen texture first.
2. Use that texture as `input_tex`.
3. Provide `screen_width` and `screen_height` in physical pixels for the offscreen texture.
4. If CRT effects are disabled, bypass this shader entirely.
5. Ghosting/persistence should be implemented later using a previous-frame texture and a second pass or feedback buffer.
6. For stronger bloom, downsample + blur in additional passes instead of increasing `glow` too far.

Recommended next steps:

- Add a previous-frame texture for persistence / ghosting
- Add optional phosphor mask
- Add per-theme color shaping / gamma tuning
- Add preset mapping in user settings
