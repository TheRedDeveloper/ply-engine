#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
uniform vec2 u_resolution;
uniform vec2 u_position;

// User uniforms
uniform vec4  u_color_a;  // start color (at u_start_angle)
uniform vec4  u_color_b;  // end color (sweeps around back to start)
uniform vec2  u_center;   // center point, normalized 0..1 (default: [0.5, 0.5])
uniform float u_offset;   // rotation offset in radians (default: 0.0)
uniform float u_hardness;  // 0.0 = smooth blend, 1.0 = hard reset at boundary

void main() {
    vec2 delta = uv - u_center;
    // atan returns -PI..PI, remap to 0..1
    float angle = atan(delta.y, delta.x) + 3.14159265;
    // Apply rotation offset
    angle = mod(angle + u_offset, 6.28318530);
    float t = angle / 6.28318530; // 0..1 around the circle

    // Blend: smooth when hardness=0, step-like when hardness=1
    float blend_t = mix(t, step(0.5, t), u_hardness);

    vec4 grad = mix(u_color_a, u_color_b, blend_t);

    vec4 base = texture2D(Texture, uv) * color;
    gl_FragColor = vec4(grad.rgb * grad.a + base.rgb * (1.0 - grad.a), max(grad.a, base.a));
}
