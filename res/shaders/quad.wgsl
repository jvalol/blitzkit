struct Screen {
    // Window size in physical pixels. zw is padding.
    size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;

// The same group the mesh pass binds a texture in, with the same layout, so
// one bind group per image serves both. Spec 0024.
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

// Positions are in pixels with the origin at the top-left and y pointing down,
// the same space text is drawn in.
@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
) -> VertexOutput {
    let clip = vec2<f32>(
        position.x / screen.size.x * 2.0 - 1.0,
        1.0 - position.y / screen.size.y * 2.0,
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(clip, 0.0, 1.0);
    out.color = color;
    out.uv = uv;
    return out;
}

// **The colour multiplies what is sampled**, the rule meshes already follow.
// A white quad shows the image untouched and a coloured one tints it. A quad
// that asked for no texture is sampling the one white pixel, so this is its
// colour exactly and there is no second pipeline for it.
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color * textureSample(image, image_sampler, in.uv);
}
