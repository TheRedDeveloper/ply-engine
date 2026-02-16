#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
uniform vec2 u_resolution;
uniform vec2 u_position;

// User uniforms
uniform vec4  u_color_a; // inner color (at center)
uniform vec4  u_color_b; // outer color (at edge)
uniform vec2  u_center;  // center point, normalized 0..1 (default: [0.5, 0.5])
uniform float u_radius;  // radius, normalized (default: 0.5)

void main() {
    float dist = length(uv - u_center);
    float t = clamp(dist / max(u_radius, 0.001), 0.0, 1.0);

    vec4 grad = mix(u_color_a, u_color_b, t);

    vec4 base = texture2D(Texture, uv) * color;
    gl_FragColor = vec4(grad.rgb * grad.a + base.rgb * (1.0 - grad.a), max(grad.a, base.a));
}
