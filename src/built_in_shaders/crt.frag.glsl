#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
uniform vec2 u_resolution;
uniform vec2 u_position;

// User uniforms
uniform float u_line_count; // number of scanlines (default: 100.0)
uniform float u_intensity;  // scanline darkness, 0.0-1.0 (default: 0.3)
uniform float u_time;       // optional flicker animation (seconds)

void main() {
    vec4 base = texture2D(Texture, uv) * color;

    // Scanlines: darken every other pixel row
    float line = uv.y * u_line_count;
    float scanline = sin(line * 3.14159) * 0.5 + 0.5;
    scanline = pow(scanline, 1.5); // sharpen the lines
    float darken = 1.0 - u_intensity * (1.0 - scanline);

    // Subtle flicker over time
    float flicker = 1.0 - 0.02 * sin(u_time * 8.0);

    // Slight RGB offset for chromatic aberration
    float offset = 0.001;
    float r = texture2D(Texture, uv + vec2(offset, 0.0)).r * color.r;
    float g = base.g;
    float b = texture2D(Texture, uv - vec2(offset, 0.0)).b * color.b;

    vec3 result = vec3(r, g, b) * darken * flicker;

    gl_FragColor = vec4(result, base.a);
}
