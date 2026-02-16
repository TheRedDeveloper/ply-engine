use crate::shaders::ShaderAsset;

/// **Foil** — Balatro-style metallic shimmer.
///
/// A diagonal sweeping highlight with subtle secondary shimmer for a metallic feel.
///
/// | Uniform       | Type    | Default | Description               |
/// |---------------|---------|---------|---------------------------|
/// | `u_time`      | `float` | —       | Animation time in seconds |
/// | `u_speed`     | `float` | `1.0`   | Sweep speed multiplier    |
/// | `u_intensity` | `float` | `0.3`   | Highlight strength        |
pub const FOIL: ShaderAsset = ShaderAsset::Source {
    file_name: "ply_builtin_foil",
    fragment: include_str!("foil.frag.glsl"),
};

/// **Holographic** — Rainbow diffraction shift.
///
/// Simulates light diffraction with angle-dependent hue shifting and wave interference.
///
/// | Uniform        | Type    | Default | Description                |
/// |----------------|---------|---------|----------------------------|
/// | `u_time`       | `float` | —       | Animation time in seconds  |
/// | `u_speed`      | `float` | `1.0`   | Animation speed multiplier |
/// | `u_saturation` | `float` | `0.7`   | Rainbow color saturation   |
pub const HOLOGRAPHIC: ShaderAsset = ShaderAsset::Source {
    file_name: "ply_builtin_holographic",
    fragment: include_str!("holographic.frag.glsl"),
};

/// **Dissolve** — Noise-based transparency with expanding regions.
///
/// Uses voronoi noise to create organic dissolve regions that expand
/// from certain areas based on a threshold. Includes a configurable glowing edge.
///
/// | Uniform        | Type    | Default | Description                                           |
/// |----------------|---------|---------|-------------------------------------------------------|
/// | `u_threshold`  | `float` | —       | Dissolve progress: `0.0` = visible, `1.0` = dissolved |
/// | `u_edge_color` | `vec4`  | —       | Color of the dissolve edge glow                       |
/// | `u_edge_width` | `float` | `0.05`  | Width of the glowing edge                             |
/// | `u_seed`       | `float` | `0.0`   | Noise seed for different patterns                     |
pub const DISSOLVE: ShaderAsset = ShaderAsset::Source {
    file_name: "ply_builtin_dissolve",
    fragment: include_str!("dissolve.frag.glsl"),
};

/// **Glow** — Outer glow / bloom effect.
///
/// Samples neighboring pixels to create a glow around opaque regions.
///
/// | Uniform            | Type    | Default | Description                              |
/// |--------------------|---------|---------|------------------------------------------|
/// | `u_glow_color`     | `vec4`  | —       | Glow color (e.g. `[0.0, 0.5, 1.0, 1.0]`) |
/// | `u_glow_radius`    | `float` | `0.05`  | Glow spread in normalized coords         |
/// | `u_glow_intensity` | `float` | `1.0`   | Glow brightness multiplier               |
pub const GLOW: ShaderAsset = ShaderAsset::Source {
    file_name: "ply_builtin_glow",
    fragment: include_str!("glow.frag.glsl"),
};

/// **CRT** — Retro CRT scanline effect.
///
/// Simulates CRT display with scanlines, subtle flicker, and chromatic aberration.
///
/// | Uniform        | Type    | Default | Description                     |
/// |----------------|---------|---------|---------------------------------|
/// | `u_line_count` | `float` | `100.0` | Number of scanlines             |
/// | `u_intensity`  | `float` | `0.3`   | Scanline darkness (`0.0`–`1.0`) |
/// | `u_time`       | `float` | —       | Animation time for flicker      |
pub const CRT: ShaderAsset = ShaderAsset::Source {
    file_name: "ply_builtin_crt",
    fragment: include_str!("crt.frag.glsl"),
};

/// **Gradient (Linear)** — Angle-based linear gradient.
///
/// Blends between two colors along a direction defined by an angle.
///
/// | Uniform     | Type    | Default | Description                                                  |
/// |-------------|---------|---------|--------------------------------------------------------------|
/// | `u_color_a` | `vec4`  | —       | Start color                                                  |
/// | `u_color_b` | `vec4`  | —       | End color                                                    |
/// | `u_angle`   | `float` | `0.0`   | Angle in radians (`0.0` = left→right, `1.5708` = top→bottom) |
pub const GRADIENT_LINEAR: ShaderAsset = ShaderAsset::Source {
    file_name: "ply_builtin_gradient_linear",
    fragment: include_str!("gradient_linear.frag.glsl"),
};

/// **Gradient (Radial)** — Circle-based radial gradient.
///
/// Blends from an inner color at the center to an outer color at the edge.
///
/// | Uniform     | Type    | Default      | Description                      |
/// |-------------|---------|--------------|----------------------------------|
/// | `u_color_a` | `vec4`  | —            | Inner color (at center)          |
/// | `u_color_b` | `vec4`  | —            | Outer color (at edge)            |
/// | `u_center`  | `vec2`  | `[0.5, 0.5]` | Center point (normalized `0..1`) |
/// | `u_radius`  | `float` | `0.5`        | Radius (normalized)              |
pub const GRADIENT_RADIAL: ShaderAsset = ShaderAsset::Source {
    file_name: "ply_builtin_gradient_radial",
    fragment: include_str!("gradient_radial.frag.glsl"),
};

/// **Gradient (Conic)** — Sweep-around circular gradient.
///
/// Colors sweep around a center point. Resets back to the start color after a
/// full rotation. Use `u_hardness` to control blend smoothness.
///
/// | Uniform      | Type    | Default      | Description                                          |
/// |--------------|---------|--------------|------------------------------------------------------|
/// | `u_color_a`  | `vec4`  | —            | Start color                                          |
/// | `u_color_b`  | `vec4`  | —            | End color (sweeps around to start)                   |
/// | `u_center`   | `vec2`  | `[0.5, 0.5]` | Center point (normalized `0..1`)                     |
/// | `u_offset`   | `float` | `0.0`        | Rotation offset in radians                           |
/// | `u_hardness` | `float` | `0.0`        | `0.0` = smooth blend, `1.0` = hard reset at boundary |
pub const GRADIENT_CONIC: ShaderAsset = ShaderAsset::Source {
    file_name: "ply_builtin_gradient_conic",
    fragment: include_str!("gradient_conic.frag.glsl"),
};
