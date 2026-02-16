#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
uniform vec2 u_resolution;
uniform vec2 u_position;

// User uniforms
uniform vec4  u_color_a; // start color
uniform vec4  u_color_b; // end color
uniform float u_angle;   // angle in radians (0.0 = left-to-right, 1.5708 = top-to-bottom)

void main() {
    // Project uv onto gradient direction
    vec2 dir = vec2(cos(u_angle), sin(u_angle));
    float t = dot(uv - vec2(0.5), dir) + 0.5;
    t = clamp(t, 0.0, 1.0);

    vec4 grad = mix(u_color_a, u_color_b, t);

    // Blend: gradient replaces background where gradient alpha > 0
    vec4 base = texture2D(Texture, uv) * color;
    gl_FragColor = vec4(grad.rgb * grad.a + base.rgb * (1.0 - grad.a), max(grad.a, base.a));
}
