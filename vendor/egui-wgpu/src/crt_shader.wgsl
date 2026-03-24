struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct CrtUniforms {
    params0: vec4<f32>, // time, curvature, scanlines, glow
    params1: vec4<f32>, // vignette, noise, brightness, contrast
    params2: vec4<f32>, // screen_width, screen_height, phosphor_softness, flicker
    params3: vec4<f32>, // bloom, burn_in, jitter, glow_line
    params4: vec4<f32>, // glow_line_speed, theme_r, theme_g, theme_b
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) history: vec4<f32>,
};

@group(0) @binding(0)
var input_tex: texture_2d<f32>;

@group(0) @binding(1)
var input_sampler: sampler;

@group(0) @binding(2)
var<uniform> crt: CrtUniforms;

@group(0) @binding(3)
var history_tex: texture_2d<f32>;

@group(0) @binding(4)
var bloom_tex: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    if (vertex_index == 0u) {
        out.position = vec4<f32>(-1.0, -3.0, 0.0, 1.0);
        out.uv = vec2<f32>(0.0, 2.0);
    } else if (vertex_index == 1u) {
        out.position = vec4<f32>(-1.0, 1.0, 0.0, 1.0);
        out.uv = vec2<f32>(0.0, 0.0);
    } else {
        out.position = vec4<f32>(3.0, 1.0, 0.0, 1.0);
        out.uv = vec2<f32>(2.0, 0.0);
    }
    return out;
}

fn clamp_uv(uv: vec2<f32>) -> bool {
    return uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
}

fn saturate_uv(uv: vec2<f32>) -> vec2<f32> {
    return clamp(uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
}

fn apply_curvature(uv: vec2<f32>, amount: f32) -> vec2<f32> {
    let centered = uv * 2.0 - vec2<f32>(1.0, 1.0);
    let r2 = dot(centered, centered);
    let warped = centered * (1.0 + amount * r2);
    return warped * 0.5 + vec2<f32>(0.5, 0.5);
}

fn hash21(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn bright_pass(c: vec3<f32>) -> vec3<f32> {
    let l = luminance(c);
    let mask = smoothstep(0.06, 0.32, l);
    return c * mask * (0.65 + 0.35 * l);
}

fn wrapped_vertical_distance(y: f32, center: f32) -> f32 {
    let d0 = abs(y - center);
    let d1 = abs(y - (center + 1.0));
    let d2 = abs(y - (center - 1.0));
    return min(d0, min(d1, d2));
}

fn glow_sample(uv: vec2<f32>, texel: vec2<f32>, strength: f32) -> vec3<f32> {
    let radius_near = texel * (1.5 + strength * 3.0);
    let radius_far = texel * (3.0 + strength * 8.0);
    let c0 = textureSample(input_tex, input_sampler, uv).rgb;
    let n1 = textureSample(input_tex, input_sampler, saturate_uv(uv + vec2<f32>( radius_near.x, 0.0))).rgb;
    let n2 = textureSample(input_tex, input_sampler, saturate_uv(uv + vec2<f32>(-radius_near.x, 0.0))).rgb;
    let n3 = textureSample(input_tex, input_sampler, saturate_uv(uv + vec2<f32>(0.0,  radius_near.y))).rgb;
    let n4 = textureSample(input_tex, input_sampler, saturate_uv(uv + vec2<f32>(0.0, -radius_near.y))).rgb;
    let n5 = textureSample(input_tex, input_sampler, saturate_uv(uv + vec2<f32>( radius_near.x,  radius_near.y))).rgb;
    let n6 = textureSample(input_tex, input_sampler, saturate_uv(uv + vec2<f32>(-radius_near.x,  radius_near.y))).rgb;
    let n7 = textureSample(input_tex, input_sampler, saturate_uv(uv + vec2<f32>( radius_near.x, -radius_near.y))).rgb;
    let n8 = textureSample(input_tex, input_sampler, saturate_uv(uv + vec2<f32>(-radius_near.x, -radius_near.y))).rgb;

    let f1 = textureSample(input_tex, input_sampler, saturate_uv(uv + vec2<f32>( radius_far.x, 0.0))).rgb;
    let f2 = textureSample(input_tex, input_sampler, saturate_uv(uv + vec2<f32>(-radius_far.x, 0.0))).rgb;
    let f3 = textureSample(input_tex, input_sampler, saturate_uv(uv + vec2<f32>(0.0,  radius_far.y))).rgb;
    let f4 = textureSample(input_tex, input_sampler, saturate_uv(uv + vec2<f32>(0.0, -radius_far.y))).rgb;

    let near_avg = (c0 + n1 + n2 + n3 + n4 + n5 + n6 + n7 + n8) / 9.0;
    let far_avg = (f1 + f2 + f3 + f4) * 0.25;
    let glow_mask = smoothstep(
        0.08,
        0.7,
        max(luminance(c0), luminance(near_avg) * 1.15),
    );
    return (near_avg * 0.8 + far_avg * 0.45) * glow_mask;
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let time = crt.params0.x;
    let curvature = crt.params0.y;
    let scanlines = crt.params0.z;
    let glow = crt.params0.w;
    let vignette = crt.params1.x;
    let noise = crt.params1.y;
    let brightness = crt.params1.z;
    let contrast = crt.params1.w;
    let screen_width = max(crt.params2.x, 1.0);
    let screen_height = max(crt.params2.y, 1.0);
    let phosphor_softness = crt.params2.z;
    let flicker = crt.params2.w;
    let bloom = crt.params3.x;
    let burn_in = crt.params3.y;
    let jitter = crt.params3.z;
    let glow_line = crt.params3.w;
    let glow_line_speed = max(crt.params4.x, 0.05);
    let theme_tint = clamp(crt.params4.yzw, vec3<f32>(0.0), vec3<f32>(1.0));
    let phosphor_tint = max(theme_tint, vec3<f32>(0.001));

    let texel = vec2<f32>(1.0 / screen_width, 1.0 / screen_height);
    let curved_uv = apply_curvature(in.uv, curvature);
    if (!clamp_uv(curved_uv)) {
        let black = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        return FragmentOutput(black, black);
    }

    var sample_uv = curved_uv;
    if (jitter > 0.0) {
        let frame_tick = floor(time * 260.0);
        let line_tick = floor(time * 420.0);
        let frame_shift = (hash21(vec2<f32>(frame_tick, 3.0)) - 0.5) * texel.x * jitter * 40.0;
        let line_shift =
            (hash21(vec2<f32>(floor(in.position.y * 0.5), line_tick)) - 0.5)
                * texel.x
                * jitter
                * 28.0;
        let vertical_shift =
            (hash21(vec2<f32>(frame_tick, 13.0)) - 0.5) * texel.y * jitter * 12.0;
        sample_uv = saturate_uv(curved_uv + vec2<f32>(frame_shift + line_shift, vertical_shift));
    }

    var color = textureSample(input_tex, input_sampler, sample_uv).rgb;
    let history_color = textureSample(history_tex, input_sampler, sample_uv).rgb;
    let source_bright = bright_pass(color);
    let screen_noise = hash21(
        vec2<f32>(floor(in.position.x * 0.75), floor(in.position.y * 0.75) + floor(time * 6.0)),
    );
    let screen_breath = 0.5 + 0.5 * sin(in.uv.y * 12.0 + time * 0.8);
    var phosphor_bg = phosphor_tint * 0.018 * (1.0 + burn_in * 0.5 + noise * 1.4);
    phosphor_bg += phosphor_tint * 0.011 * screen_breath * 0.45;
    phosphor_bg +=
        phosphor_tint * 0.008 * max(screen_noise - 0.5, 0.0) * (0.5 + noise * 1.8);
    color = max(color, phosphor_bg);

    let persistence = clamp(0.22 + burn_in * 0.7, 0.0, 0.95);
    if (persistence > 0.0) {
        let history_tail = bright_pass(history_color);
        let history_smear_a = bright_pass(
            textureSample(
                history_tex,
                input_sampler,
                saturate_uv(sample_uv + vec2<f32>(texel.x * 7.0, 0.0)),
            ).rgb,
        );
        let history_smear_b = bright_pass(
            textureSample(
                history_tex,
                input_sampler,
                saturate_uv(sample_uv - vec2<f32>(texel.x * 3.0, 0.0)),
            ).rgb,
        );
        let retained = mix(history_tail, (history_tail + history_smear_a + history_smear_b) / 3.0, 0.4);
        color = max(color, retained * persistence);
    }

    if (phosphor_softness > 0.0) {
        let s1 = textureSample(input_tex, input_sampler, saturate_uv(sample_uv + vec2<f32>( texel.x, 0.0))).rgb;
        let s2 = textureSample(input_tex, input_sampler, saturate_uv(sample_uv + vec2<f32>(-texel.x, 0.0))).rgb;
        let s3 = textureSample(input_tex, input_sampler, saturate_uv(sample_uv + vec2<f32>(0.0,  texel.y))).rgb;
        let s4 = textureSample(input_tex, input_sampler, saturate_uv(sample_uv + vec2<f32>(0.0, -texel.y))).rgb;
        let blur = (s1 + s2 + s3 + s4) * 0.25;
        color = mix(color, blur, clamp(phosphor_softness, 0.0, 1.0) * 0.75);
    }

    if (glow > 0.0) {
        let glow_col = glow_sample(sample_uv, texel, glow);
        color += glow_col * (0.25 + glow * 0.95);
    }

    if (bloom > 0.0) {
        let bloom_col = textureSample(bloom_tex, input_sampler, sample_uv).rgb;
        color += bloom_col * (0.2 + bloom * 1.6);
    }

    if (burn_in > 0.0) {
        let wear_noise = hash21(floor(in.uv * vec2<f32>(84.0, 60.0)));
        let wear_lines = 0.5 + 0.5 * sin(in.uv.y * 42.0 + in.uv.x * 3.0);
        let centered = sample_uv * 2.0 - vec2<f32>(1.0, 1.0);
        let radial = dot(centered, centered);
        let wear = 1.0
            - burn_in
                * (0.04 + radial * 0.18 + (wear_noise - 0.5) * 0.04 + wear_lines * 0.03);
        color *= clamp(wear, 0.65, 1.0);
    }

    if (scanlines > 0.0) {
        let scan_phase = in.position.y * 3.14159265;
        let scan = 0.5 + 0.5 * cos(scan_phase);
        let scan_dark = pow(1.0 - scan, 1.5);
        let scan_mix = 1.0 - clamp(scanlines, 0.0, 1.0) * (0.24 + 0.76 * scan_dark);
        color *= max(scan_mix, 0.05);
    }

    if (vignette > 0.0) {
        let centered = in.uv * 2.0 - vec2<f32>(1.0, 1.0);
        let dist = dot(centered, centered);
        let vig = pow(clamp(1.0 - dist * vignette * 0.9, 0.0, 1.0), 1.4);
        color *= vig;
    }

    if (glow_line > 0.0) {
        let sweep_head = fract(time * glow_line_speed);
        let sweep_y = curved_uv.y;
        let sweep_core_dist = wrapped_vertical_distance(sweep_y, sweep_head);
        let sweep_core = exp(-pow(sweep_core_dist * (240.0 + glow_line * 240.0), 2.0));
        let trail_dist = fract(sweep_head - sweep_y + 1.0);
        let sweep_trail = exp(-trail_dist * (8.0 + glow_line * 14.0))
            * (1.0 - smoothstep(0.22, 0.94, trail_dist));
        let sweep = sweep_core * 1.0 + sweep_trail * 0.8;
        let sweep_tint =
            phosphor_tint * (0.12 + glow_line * 0.2) + phosphor_bg * 7.5 + bright_pass(color) * 0.18;
        color += sweep_tint * sweep * glow_line * 0.78;
    }

    if (noise > 0.0) {
        let n = hash21(
            sample_uv * vec2<f32>(screen_width, screen_height)
                + vec2<f32>(time * 23.17, time * 11.31),
        );
        let line_n = hash21(
            vec2<f32>(floor(in.position.y * 0.5), time * 17.0),
        );
        let noise_val = ((n - 0.5) * 0.7 + (line_n - 0.5) * 0.3) * noise * 0.28;
        color += vec3<f32>(noise_val);
    }

    if (flicker > 0.0) {
        let flicker_fast = 0.5 + 0.5 * sin(time * 360.0);
        let flicker_rand = hash21(vec2<f32>(floor(time * 240.0), 19.0));
        let flicker_mix = flicker_fast * 0.7 + flicker_rand * 0.3;
        let flicker_wave = 1.0 - flicker * (0.1 + 0.9 * flicker_mix);
        color *= flicker_wave;
    }

    color = (color - vec3<f32>(0.5)) * contrast + vec3<f32>(0.5);
    color *= brightness;
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    let history_seed = max(
        source_bright * (0.4 + burn_in * 0.35),
        bright_pass(color) * (0.08 + burn_in * 0.18),
    );
    let history_decay = 0.76 + burn_in * 0.18;
    let history_out = max(history_seed, history_color * history_decay);

    let final_color = vec4<f32>(color, 1.0);
    return FragmentOutput(final_color, vec4<f32>(history_out, 1.0));
}
