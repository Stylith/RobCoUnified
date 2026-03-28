// crt_shader.wgsl
//
// Starter CRT post-process shader for NucleonOS.
// Intended as a single-pass starting point for a WGPU/egui pipeline.
//
// Features included:
// - curvature / barrel distortion
// - scanlines
// - vignette
// - animated noise
// - simple phosphor glow approximation
//
// Notes:
// - This is a practical starter shader, not a physically accurate CRT emulator.
// - Bloom/glow here is intentionally lightweight.
// - Ghosting/persistence is NOT included in this file; that usually needs a previous-frame texture.
// - Parameter ranges should be clamped by the host app.
//
// Expected pipeline:
// 1. Render egui UI to offscreen texture
// 2. Run this shader using that texture as input
// 3. Present final image to the screen

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct CrtUniforms {
    time: f32,
    curvature: f32,
    scanlines: f32,
    glow: f32,

    vignette: f32,
    noise: f32,
    brightness: f32,
    contrast: f32,

    screen_width: f32,
    screen_height: f32,
    phosphor_softness: f32,
    pad0: f32,
};

@group(0) @binding(0)
var input_tex: texture_2d<f32>;

@group(0) @binding(1)
var input_sampler: sampler;

@group(0) @binding(2)
var<uniform> crt: CrtUniforms;

// Fullscreen triangle / quad vertex shader.
// If your host already provides a fullscreen vertex stage, you may replace this.
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

fn clamp_uv(uv: vec2<f32>) -> bool {
    return uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
}

// Slight barrel distortion to emulate CRT curvature.
fn apply_curvature(uv: vec2<f32>, amount: f32) -> vec2<f32> {
    let centered = uv * 2.0 - vec2<f32>(1.0, 1.0);
    let r2 = dot(centered, centered);
    let warped = centered * (1.0 + amount * r2);
    return warped * 0.5 + vec2<f32>(0.5, 0.5);
}

// Stable-ish hash noise.
fn hash21(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

// Small luminance helper.
fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// Cheap glow approximation: average neighboring samples around bright pixels.
// This is intentionally lightweight and not a true bloom pipeline.
fn glow_sample(uv: vec2<f32>, texel: vec2<f32>) -> vec3<f32> {
    let c0 = textureSample(input_tex, input_sampler, uv).rgb;
    let c1 = textureSample(input_tex, input_sampler, uv + vec2<f32>( texel.x, 0.0)).rgb;
    let c2 = textureSample(input_tex, input_sampler, uv + vec2<f32>(-texel.x, 0.0)).rgb;
    let c3 = textureSample(input_tex, input_sampler, uv + vec2<f32>(0.0,  texel.y)).rgb;
    let c4 = textureSample(input_tex, input_sampler, uv + vec2<f32>(0.0, -texel.y)).rgb;
    let c5 = textureSample(input_tex, input_sampler, uv + vec2<f32>( texel.x,  texel.y)).rgb;
    let c6 = textureSample(input_tex, input_sampler, uv + vec2<f32>(-texel.x,  texel.y)).rgb;
    let c7 = textureSample(input_tex, input_sampler, uv + vec2<f32>( texel.x, -texel.y)).rgb;
    let c8 = textureSample(input_tex, input_sampler, uv + vec2<f32>(-texel.x, -texel.y)).rgb;

    let avg = (c0 + c1 + c2 + c3 + c4 + c5 + c6 + c7 + c8) / 9.0;
    let glow_mask = smoothstep(0.45, 1.0, luminance(avg));
    return avg * glow_mask;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = vec2<f32>(1.0 / max(crt.screen_width, 1.0), 1.0 / max(crt.screen_height, 1.0));

    // 1. Distort coordinates for curvature.
    let curved_uv = apply_curvature(in.uv, crt.curvature);

    // If warped outside the source image, return black.
    if (!clamp_uv(curved_uv)) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // 2. Sample base color.
    var color = textureSample(input_tex, input_sampler, curved_uv).rgb;

    // 3. Cheap phosphor softness using tiny cross blur.
    if (crt.phosphor_softness > 0.0) {
        let s1 = textureSample(input_tex, input_sampler, curved_uv + vec2<f32>( texel.x, 0.0)).rgb;
        let s2 = textureSample(input_tex, input_sampler, curved_uv + vec2<f32>(-texel.x, 0.0)).rgb;
        let s3 = textureSample(input_tex, input_sampler, curved_uv + vec2<f32>(0.0,  texel.y)).rgb;
        let s4 = textureSample(input_tex, input_sampler, curved_uv + vec2<f32>(0.0, -texel.y)).rgb;
        let blur = (s1 + s2 + s3 + s4) * 0.25;
        color = mix(color, blur, clamp(crt.phosphor_softness, 0.0, 1.0) * 0.35);
    }

    // 4. Add simple glow.
    if (crt.glow > 0.0) {
        let glow_col = glow_sample(curved_uv, texel * 1.5);
        color += glow_col * crt.glow * 0.5;
    }

    // 5. Scanlines.
    if (crt.scanlines > 0.0) {
        let y = curved_uv.y * crt.screen_height;
        let scan = 0.5 + 0.5 * cos(y * 3.14159265);
        let scan_mix = mix(1.0, 0.72 + 0.28 * scan, clamp(crt.scanlines, 0.0, 1.0));
        color *= scan_mix;
    }

    // 6. Vignette.
    if (crt.vignette > 0.0) {
        let centered = in.uv * 2.0 - vec2<f32>(1.0, 1.0);
        let dist = dot(centered, centered);
        let vig = 1.0 - dist * crt.vignette * 0.55;
        color *= clamp(vig, 0.0, 1.0);
    }

    // 7. Animated noise.
    if (crt.noise > 0.0) {
        let n = hash21(curved_uv * vec2<f32>(crt.screen_width, crt.screen_height) + vec2<f32>(crt.time * 23.17, crt.time * 11.31));
        let noise_val = (n - 0.5) * crt.noise * 0.08;
        color += vec3<f32>(noise_val);
    }

    // 8. Contrast / brightness shaping.
    color = (color - vec3<f32>(0.5)) * crt.contrast + vec3<f32>(0.5);
    color *= crt.brightness;

    // 9. Clamp final output.
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    return vec4<f32>(color, 1.0);
}
