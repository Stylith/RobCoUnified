# RobCoUnified CRT Display Effects System

## Overview

This document describes the CRT display effects system for the
RobCoUnified desktop environment.

The goal is to reproduce a retro CRT monitor aesthetic similar to
COOL‑RETRO‑TERM, while maintaining good performance and allowing full
user customization.

CRT effects must: - Apply to the entire application output - Be
optional - Be configurable - Be performance scalable - Be implemented as
a rendering layer, not baked into UI widgets

Rendering model:

egui UI → offscreen render texture → CRT post‑processing shader → final
display

egui renders the UI normally, and a shader pass applies CRT effects to
the final frame.

------------------------------------------------------------------------

# Architecture

## Rendering Pipeline

Normal rendering

egui UI\
→ GPU render target\
→ present

CRT enabled

egui UI\
→ offscreen framebuffer\
→ CRT shader pass\
→ screen

Because GPUs cannot read and write the same texture simultaneously,
intermediate buffers may be required for multi‑pass effects.

------------------------------------------------------------------------

# Supported CRT Effects

## Curvature

Simulates curved CRT glass using barrel distortion.

Parameters: - curvature_strength

------------------------------------------------------------------------

## Scanlines

Dark horizontal lines representing raster scanning.

Parameters: - scanline_strength - scanline_spacing - scanline_softness

------------------------------------------------------------------------

## Phosphor Glow / Bloom

Bright pixels bleed into surrounding pixels.

Implementation: bright_pixels → blur → composite back

------------------------------------------------------------------------

## Vignette

Darkens screen edges.

Parameters: - vignette_strength - vignette_radius

------------------------------------------------------------------------

## Noise / Static

Adds subtle animated noise simulating electrical interference.

Parameter: - noise_strength

------------------------------------------------------------------------

## Flicker

Subtle brightness fluctuation over time.

Parameter: - flicker_strength

------------------------------------------------------------------------

## Phosphor Persistence / Ghosting

Blends previous frame with current frame to simulate phosphor decay.

Implementation: final_pixel = mix(current_frame, previous_frame,
persistence_strength)

------------------------------------------------------------------------

## Phosphor Mask (Optional)

Simulates physical phosphor patterns or aperture grille.

Parameter: - phosphor_mask_strength

------------------------------------------------------------------------

## Edge Falloff

Reduces brightness near screen edges.

Parameter: - edge_falloff

------------------------------------------------------------------------

## Glass Reflection (Optional)

Adds faint reflections or glare overlay.

Parameter: - glass_reflection_strength

------------------------------------------------------------------------

# Effects Pipeline Order

Recommended order:

1.  sample base frame
2.  apply curvature distortion
3.  apply bloom / glow
4.  apply scanlines
5.  apply vignette
6.  apply noise
7.  apply flicker
8.  final color shaping

------------------------------------------------------------------------

# Configuration System

Example Rust configuration:

``` rust
#[derive(Clone, Serialize, Deserialize)]
pub struct DisplayEffectsSettings {
    pub enabled: bool,
    pub preset: CrtPreset,

    pub curvature: f32,
    pub scanlines: f32,
    pub glow: f32,
    pub blur: f32,
    pub vignette: f32,
    pub noise: f32,
    pub flicker: f32,
    pub ghosting: f32,

    pub phosphor_mask: f32,
    pub edge_falloff: f32,
    pub glass_reflection: f32,
}
```

------------------------------------------------------------------------

# Presets

``` rust
pub enum CrtPreset {
    Off,
    Subtle,
    RobCoStandard,
    WornTerminal,
    ExtremeRetro,
    Custom,
}
```

Descriptions:

Off\
No CRT effects.

Subtle\
Minimal scanlines and vignette.

RobCo Standard\
Recommended default with moderate CRT effects.

Worn Terminal\
Adds noise, stronger vignette, light ghosting.

Extreme Retro\
Heavy distortion, glow, scanlines, and noise.

------------------------------------------------------------------------

# Settings UI Example

Display Effects

Enable CRT Effects

Preset - Off - Subtle - RobCo Standard - Worn Terminal - Extreme Retro -
Custom

Advanced Settings - Curvature - Scanlines - Glow - Blur - Noise -
Flicker - Ghosting - Vignette - Glass Reflection - Edge Falloff

Changing sliders automatically switches preset to Custom.

------------------------------------------------------------------------

# Performance Considerations

Performance depends on: - screen resolution - number of shader passes -
animation frequency

Low cost effects: - scanlines - vignette - palette shaping - mild noise

Medium cost effects: - curvature distortion - blur - glow

High cost effects: - bloom - ghosting / persistence - phosphor mask -
multi-pass blur

------------------------------------------------------------------------

# Performance Optimization

Skip inactive effects:

if glow == 0 → skip glow pass\
if ghosting == 0 → skip persistence buffer

Use half resolution for bloom and glow passes.

Avoid unnecessary animation.

Provide a low‑power preset.

------------------------------------------------------------------------

# Performance Tiers

Off → minimal GPU cost\
Subtle → very low cost\
RobCoStandard → moderate cost\
WornTerminal → moderate‑high cost\
ExtremeRetro → high cost

When effects are disabled, bypass the CRT pipeline entirely.

------------------------------------------------------------------------

# Optional Advanced Features

Effects Scope

Allow effects to apply to: - Entire App - Desktop Only - Terminal
Windows Only - Boot Screen Only

------------------------------------------------------------------------

# Hotkey Toggle

Example:

Ctrl + Shift + E

Instantly enable or disable CRT effects.

------------------------------------------------------------------------

# Design Philosophy

CRT effects should balance:

visual authenticity\
+ usability\
+ performance stability

Users should be able to tune the display for atmosphere, clarity, or
performance.

CRT effects should behave like a display profile rather than a forced
visual theme.

------------------------------------------------------------------------

# Implementation Priority

1.  Offscreen framebuffer rendering
2.  Scanline and vignette shader
3.  Curvature distortion
4.  Glow / bloom
5.  Noise and flicker
6.  Ghosting / persistence
7.  Phosphor mask and advanced effects
