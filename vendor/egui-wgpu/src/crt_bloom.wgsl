struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0)
var source_tex: texture_2d<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

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

fn saturate_uv(uv: vec2<f32>) -> vec2<f32> {
    return clamp(uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
}

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn bright_pass(c: vec3<f32>) -> vec3<f32> {
    let l = luminance(c);
    let threshold = smoothstep(0.08, 0.34, l);
    return c * threshold * threshold;
}

fn sample_source(uv: vec2<f32>) -> vec3<f32> {
    return textureSample(source_tex, source_sampler, saturate_uv(uv)).rgb;
}

fn texel_size() -> vec2<f32> {
    let dims = max(vec2<f32>(textureDimensions(source_tex)), vec2<f32>(1.0, 1.0));
    return 1.0 / dims;
}

@fragment
fn extract_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = texel_size();
    let c0 = bright_pass(sample_source(in.uv));
    let c1 = bright_pass(sample_source(in.uv + vec2<f32>( texel.x,  texel.y)));
    let c2 = bright_pass(sample_source(in.uv + vec2<f32>(-texel.x,  texel.y)));
    let c3 = bright_pass(sample_source(in.uv + vec2<f32>( texel.x, -texel.y)));
    let c4 = bright_pass(sample_source(in.uv + vec2<f32>(-texel.x, -texel.y)));
    let extract = (c0 * 2.0 + c1 + c2 + c3 + c4) / 6.0;
    return vec4<f32>(extract, 1.0);
}

fn gaussian_blur(uv: vec2<f32>, axis: vec2<f32>) -> vec3<f32> {
    let texel = texel_size() * axis;
    let c0 = sample_source(uv) * 0.22702703;
    let c1 = sample_source(uv + texel * 1.0) * 0.19459459;
    let c2 = sample_source(uv - texel * 1.0) * 0.19459459;
    let c3 = sample_source(uv + texel * 2.0) * 0.12162162;
    let c4 = sample_source(uv - texel * 2.0) * 0.12162162;
    let c5 = sample_source(uv + texel * 3.0) * 0.05405405;
    let c6 = sample_source(uv - texel * 3.0) * 0.05405405;
    let c7 = sample_source(uv + texel * 4.0) * 0.01621622;
    let c8 = sample_source(uv - texel * 4.0) * 0.01621622;
    return c0 + c1 + c2 + c3 + c4 + c5 + c6 + c7 + c8;
}

@fragment
fn blur_h_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(gaussian_blur(in.uv, vec2<f32>(1.0, 0.0)), 1.0);
}

@fragment
fn blur_v_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(gaussian_blur(in.uv, vec2<f32>(0.0, 1.0)), 1.0);
}
