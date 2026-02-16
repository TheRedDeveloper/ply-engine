#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
uniform vec2 u_resolution;
uniform vec2 u_position;

// User uniforms
uniform float u_time;       // animation time (seconds)
uniform float u_speed;      // animation speed (default: 1.0)
uniform float u_saturation; // rainbow saturation (default: 0.7)

// HSV to RGB conversion
vec3 hsv2rgb(vec3 c) {
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

void main() {
    vec4 base = texture2D(Texture, uv) * color;

    // Angle-dependent hue shift (simulates light diffraction)
    float angle = atan(uv.y - 0.5, uv.x - 0.5);
    float dist = length(uv - vec2(0.5));

    // Hue shifts with position and time
    float hue = angle / 6.28318 + dist * 0.5 + u_time * u_speed * 0.15;
    hue = fract(hue);

    // Wave interference pattern
    float wave1 = sin((uv.x + uv.y) * 8.0 + u_time * u_speed);
    float wave2 = sin((uv.x - uv.y) * 6.0 - u_time * u_speed * 0.7);
    float interference = (wave1 + wave2) * 0.25 + 0.5;

    hue = fract(hue + interference * 0.2);

    vec3 rainbow = hsv2rgb(vec3(hue, u_saturation, 1.0));

    // Blend rainbow with base, preserving luminance
    float luma = dot(base.rgb, vec3(0.299, 0.587, 0.114));
    vec3 result = mix(base.rgb, rainbow * luma + base.rgb * 0.3, 0.5 * base.a);

    gl_FragColor = vec4(result, base.a);
}
