#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
uniform vec2 u_resolution;
uniform vec2 u_position;

// User uniforms
uniform vec4  u_glow_color;     // glow color (e.g. [0.0, 0.5, 1.0, 1.0])
uniform float u_glow_radius;    // glow size in normalized coords (default: 0.05)
uniform float u_glow_intensity; // glow brightness multiplier (default: 1.0)

void main() {
    vec4 base = texture2D(Texture, uv) * color;

    // Sample in a ring around the pixel to detect edges
    // Use a simple box blur to approximate the glow source
    float glow_alpha = 0.0;
    float samples = 0.0;

    // Aspect-correct sampling
    vec2 pixel = vec2(u_glow_radius, u_glow_radius * u_resolution.x / u_resolution.y);

    // 3x3 extended sampling kernel for glow spread
    for (int x = -2; x <= 2; x++) {
        for (int y = -2; y <= 2; y++) {
            vec2 offset = vec2(float(x), float(y)) * pixel * 0.5;
            vec4 s = texture2D(Texture, uv + offset);
            glow_alpha += s.a;
            samples += 1.0;
        }
    }
    glow_alpha = glow_alpha / samples;

    // Outer glow: visible only where the base is transparent but neighbors are not
    float outer = glow_alpha * (1.0 - base.a) * u_glow_intensity;

    vec3 result = mix(u_glow_color.rgb * outer, base.rgb, base.a);
    float alpha = clamp(base.a + outer * u_glow_color.a, 0.0, 1.0);

    gl_FragColor = vec4(result, alpha);
}
