#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
uniform vec2 u_resolution;
uniform vec2 u_position;

uniform float u_threshold;
uniform vec4  u_edge_color;
uniform float u_edge_width;
uniform float u_seed;

// Polynomial hashes — no sin(), precision-safe
vec2 phash2(vec2 p) {
    vec3 p3 = fract(vec3(p.x, p.y, p.x) * vec3(0.1031, 0.1030, 0.0973) + u_seed * 0.01);
    float d = dot(p3, vec3(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    p3 = p3 + vec3(d, d, d);
    return fract(vec2((p3.x + p3.y) * p3.z, (p3.y + p3.z) * p3.x));
}

float phash(vec2 p) {
    vec3 p3 = fract(vec3(p.x, p.y, p.x) * 0.1031 + u_seed * 0.01);
    float d = dot(p3, vec3(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    p3 = p3 + vec3(d, d, d);
    return fract((p3.x + p3.y) * p3.z);
}

float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    float a = phash(i);
    float b = phash(i + vec2(1.0, 0.0));
    float c = phash(i + vec2(0.0, 1.0));
    float d = phash(i + vec2(1.0, 1.0));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

float voronoi(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    float best = 1.0;
    for (int y = -1; y <= 1; y++) {
        for (int x = -1; x <= 1; x++) {
            vec2 nb = vec2(float(x), float(y));
            vec2 pt = phash2(i + nb);
            vec2 diff = nb + pt - f;
            float d = dot(diff, diff);
            if (d < best) best = d;
        }
    }
    return sqrt(best);
}

void main() {
    vec4 base = texture2D(Texture, uv) * color;

    // Two-level domain warp to fully destroy grid regularity
    vec2 w1 = vec2(noise(uv * 3.0 + vec2(0.0, 3.7)), noise(uv * 3.0 + vec2(7.3, 0.0)));
    vec2 uv1 = uv + (w1 - 0.5) * 0.6;
    vec2 w2 = vec2(noise(uv1 * 4.5 + vec2(1.7, 9.2)), noise(uv1 * 4.5 + vec2(8.3, 2.8)));
    vec2 warped = uv1 + (w2 - 0.5) * 0.3;

    // Non-integer Voronoi scales prevent grid alignment between layers
    float v = voronoi(warped * 3.7) * 0.5
            + voronoi(warped * 8.3 + vec2(3.7, 8.1)) * 0.3
            + noise(warped * 15.0) * 0.2;

    float dissolve_map = clamp(v, 0.0, 1.0);

    // Smooth threshold with edge glow
    float alive = smoothstep(u_threshold - u_edge_width, u_threshold, dissolve_map);
    float inner = smoothstep(u_threshold, u_threshold + u_edge_width, dissolve_map);

    vec3 result = mix(u_edge_color.rgb, base.rgb, inner);
    float glow = (1.0 - inner) * alive;
    result = result + u_edge_color.rgb * glow * 2.5;

    float alpha = base.a * alive;
    gl_FragColor = vec4(result * alpha, alpha);
}
