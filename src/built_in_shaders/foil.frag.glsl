#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
uniform vec2 u_resolution;
uniform vec2 u_position;

// User uniforms
uniform float u_time;      // animation time (seconds)
uniform float u_speed;     // sweep speed (default: 1.0)
uniform float u_intensity; // highlight strength (default: 0.3)

void main() {
    vec4 base = texture2D(Texture, uv) * color;

    // Diagonal sweep: a bright band moving across the surface
    float diag = uv.x + uv.y; // 0..2 diagonal
    float sweep = sin(diag * 3.14159 * 2.0 - u_time * u_speed * 2.0);
    sweep = sweep * 0.5 + 0.5;            // remap to 0..1
    sweep = smoothstep(0.4, 0.6, sweep);  // sharpen into a band

    // Secondary shimmer for metallic feel
    float shimmer = sin(uv.x * 12.0 + u_time * u_speed * 1.5) *
                    sin(uv.y * 10.0 - u_time * u_speed * 0.8);
    shimmer = shimmer * 0.5 + 0.5;
    shimmer = shimmer * 0.3; // subtle

    float highlight = (sweep + shimmer) * u_intensity;

    gl_FragColor = vec4(base.rgb + vec3(highlight) * base.a, base.a);
}
