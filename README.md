# Ply Engine

> [!WARNING]  
> There will probably be some breaking changes soon.

A pure Rust UI layout engine built on [macroquad](https://github.com/not-fl3/macroquad), inspired by [Clay](https://github.com/nicbarker/clay). Blazingly fast, safe, and ready for desktop and web.

## Features

- **Pure Rust**: no C bindings, no FFI, no irremovable `unsafe` code
- **Macroquad renderer**: antialiased rounded rectangles, borders, clipping, images, and text out of the box
- **Shader effects**: shaders directly in layout with [Slang](https://shader-slang.com/), GLSL and other languages with build pipline
- **TextureManager**: automatic GPU texture caching with configurable eviction, supports file paths and embedded bytes
- **TinyVG support**: render resolution-independent vector graphics via the `tinyvg` feature
- **Text styling**: rich inline markup for color, effects (wave, jitter, gradient, …), and animations (type-in, fade, scale) via the `text-styling` feature
- **WebAssembly**: ship to the browser, guide below.
- **Debug view**: inspect the layout tree

## Installation

```toml
[dependencies]
ply-engine = "0.4"
macroquad = { version = "0.4", git = "https://github.com/TheRedDeveloper/macroquad-fix" } # You have to this temporarily until the PR is merged to fix dragging issues

# Optional features:
# ply-engine = { version = "0.4", features = ["text-styling", "tinyvg", "built-in-shaders"] }
```

## Quick Start

```rust
use macroquad::prelude::*;
use ply_engine::{
    fixed, grow,
    Color, Ply,
    layout::{LayoutAlignmentX, LayoutAlignmentY, LayoutDirection},
};

// Configure the window, I recommend these settings
fn window_conf() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: miniquad::conf::Conf {
            window_title: "My App".to_owned(),
            window_width: 500,
            window_height: 500,
            high_dpi: true,
            sample_count: 4,
            platform: miniquad::conf::Platform {
                // WebGL2 is required for TinyVG and shader support
                webgl_version: miniquad::conf::WebGLVersion::WebGL2,
                ..Default::default()
            },
            ..Default::default()
        },
        draw_call_vertex_capacity: 100000,
        draw_call_index_capacity: 100000,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Load fonts
    let fonts = vec![load_ttf_font("assets/lexend.ttf").await.unwrap()];

    // Create the engine
    let mut ply = Ply::<()>::new(fonts);

    loop {
        clear_background(BLACK);

        // Begin layout
        let mut ui = ply.begin();

        // A column filling the screen
        ui.element().width(grow!()).height(grow!())
            .layout(|l| l
                .direction(LayoutDirection::TopToBottom)
                .gap(16)
                .align(LayoutAlignmentX::Center, LayoutAlignmentY::Center)
            )
            .children(|ui| {
                // A colored box
                ui.element().width(fixed!(200.0)).height(fixed!(100.0))
                    .corner_radius(8.0)
                    .background_color(0x45A85A)
                    .empty();

                // Some text below that box
                ui.text("Hello, Ply!", |t| t
                    .font_size(32)
                    .color(0xFFFFFF)
                );
            });

        ui.show(|_| {}).await;

        next_frame().await;
    }
}
```

## Layout API

Layouts are built with a closure-based nesting API. The `element()` builder configures each element's sizing, alignment, borders, background color, images, and more.

```rust
ui.element().width(fixed!(250.0)).height(grow!())
    .id("sidebar")
    .layout(|l| l
        .direction(LayoutDirection::TopToBottom)
        .gap(8)
        .padding(16)
    )
    .border(|b| b
        .color(0x333333)
        .right(2)
    )
    .background_color(0x1A1A2E)
    .children(|ui| {
        // children go here
    });
```

Sizing helpers: `fixed!(px)`, `grow!()`, `fit!()`, and `percent!(0.0..=1.0)`.

## TextureManager

`TEXTURE_MANAGER` is a global, thread-safe texture cache. The renderer uses it automatically for images and TinyVG assets — you can also use it directly.

```rust
// Load and cache a texture (async)
let tex = TEXTURE_MANAGER.lock().unwrap().get_or_load("sprites/hero.png").await;

// Embed bytes at compile time
static LOGO: GraphicAsset = GraphicAsset::Bytes {
    file_name: "logo.png",
    data: include_bytes!("../assets/logo.png"),
};

// Adjust eviction (default: textures are freed after 1 unused frame)
TEXTURE_MANAGER.lock().unwrap().max_frames_not_used = 5;
```

## Text Styling

Enable with `features = ["text-styling"]`. Inline tags style individual runs of text:

```text
{color=red|Red text} and {wave_a=0.3|wavy text}
{type_in_id=intro_speed=12_cursor=\||Typewriter effect}
{fade_in_id=hello_speed=5|Fading in}
{gradient_stops=0:#FF0000,5:#00FF00|Rainbow gradient}
```

Tags stack, inner tags override outer, and special characters are escaped with `\`.
See [text-styling.md](text-styling.md) for the full reference.

## TinyVG Support

Enable with `features = ["tinyvg"]`. Reference `.tvg` files through the `GraphicAsset` enum and they render at any resolution. Convert your `.svg`s into `.tvg`s with the [official tools](https://tinyvg.tech/) and enjoy ultra-compact assets with blazingly fast rendering.

```rust
static ICON: GraphicAsset = GraphicAsset::Bytes {
    file_name: "icon.tvg",
    data: include_bytes!("../assets/icon.tvg"),
};
```

## Shaders

Ply supports GPU shader effects on individual elements and groups of elements. Write fragment shaders in [Slang](https://shader-slang.com/) or plain GLSL ES 1.00 and apply them declaratively.

### Per-Element Effects

Apply a shader to a single element with `.effect()`:

```rust
use ply_engine::shaders::ShaderAsset;

static GRADIENT: ShaderAsset = ShaderAsset::Source {
    file_name: "gradient.glsl",
    fragment: include_str!("../assets/build/shaders/gradient.frag.glsl"),
};

ui.element().width(fixed!(150.0)).height(fixed!(100.0))
    .corner_radius(12.0)
    .background_color(0xFFFFFF)
    .effect(&GRADIENT, |s| {
        s.uniform("color_a", [0.2f32, 0.6, 1.0, 1.0])
         .uniform("color_b", [1.0f32, 0.3, 0.5, 1.0]);
    })
    .empty();
```

### Group Shaders

Apply a shader to an element **and all its children** with `.shader()`. The entire subtree is rendered to an offscreen texture, then the shader processes the result:

```rust
static WAVE: ShaderAsset = ShaderAsset::Source {
    file_name: "wave.glsl",
    fragment: include_str!("../assets/build/shaders/wave.frag.glsl"),
};

ui.element().width(fixed!(200.0)).height(fixed!(200.0))
    .shader(&WAVE, |s| {
        s.uniform("time", get_time() as f32);
    })
    .children(|ui| {
        ui.text("Wobbly!", |t| t.font_size(24).color(0xFFFFFF));
    });
```

Multiple `.shader()` and `.effect()` calls chain — each stage feeds into the next.

### Built-in Shaders

Enable `features = ["built-in-shaders"]` for ready-to-use effects:

```rust
use ply_engine::built_in_shaders::{FOIL, HOLOGRAPHIC, DISSOLVE, GRADIENT_LINEAR};

ui.element()
    .shader(&HOLOGRAPHIC, |s| s.uniform("u_time", time).uniform("u_speed", 1.0f32).uniform("u_saturation", 0.7f32))
    .children(|ui| { /* ... */ });
```

**Available**: `FOIL`, `HOLOGRAPHIC`, `DISSOLVE`, `GLOW`, `CRT`, `GRADIENT_LINEAR`, `GRADIENT_RADIAL`, `GRADIENT_CONIC`, add your own through a PR!

### Build Pipeline

Compile shaders at build time with a one-line `build.rs`. Source files in `shaders/` are auto-detected and output as GLSL ES 1.00 to `assets/build/shaders/`:

```rust
// build.rs
fn main() {
    ply_engine::shader_build::ShaderBuild::new()
        .source_dir("shaders/")
        .output_dir("assets/build/shaders/")
        .build();
}
```

```toml
[build-dependencies]
ply-engine = { version = "0.3", features = ["shader-build"] }
```

The `shader-build` feature bundles [spirv-cross2](https://crates.io/crates/spirv-cross2) so Slang/HLSL → GLSL conversion needs no CLI tools beyond `slangc`. Plain `.glsl` / `.frag` files are copied through with no extra tooling needed.

## Let's Get Technical

### Writing Fragment Shaders

Fragment shaders receive these from macroquad's internal vertex shader:

```glsl
varying vec2 uv;            // UV coordinates (0–1)
varying vec4 color;         // Vertex color
uniform sampler2D Texture;  // Element/RT texture
```

Ply auto-injects these uniforms:

| Uniform        | Type   | Description                         |
|----------------|--------|-------------------------------------|
| `u_resolution` | `vec2` | Element bounding box size in pixels |
| `u_position`   | `vec2` | Element position in screen space    |

User uniforms are set via `.uniform()` with these types: `f32`, `[f32; 2]`, `[f32; 3]`, `[f32; 4]`, `i32`, `[[f32; 4]; 4]`.

### MaterialManager

Compiled shader materials are cached on the GPU by `MaterialManager`, which lives inside the renderer. Materials are keyed by their fragment source + uniform values, so identical configurations reuse the same GPU program. Unused materials are automatically evicted after 60 frames of inactivity to keep GPU memory lean.

### Getting Started with Slang

[Slang](https://shader-slang.com/) is a modern shading language developed by NVIDIA. It offers modules, generics, interfaces, and a familiar C-like syntax — all of which make it a great choice for writing maintainable shader code. Slang compiles to SPIR-V, which Ply's build pipeline then cross-compiles to GLSL ES 1.00.

**1. Install slangc**

Download the latest release from the [Slang GitHub releases](https://github.com/shader-slang/slang/releases) and add the `bin/` directory to your PATH, or point the build pipeline to it directly:

```rust
// build.rs
ply_engine::shader_build::ShaderBuild::new()
    .slangc_path("/path/to/slangc")
    .build();
```

**2. Write a `.slang` shader**

Create `shaders/glow.slang`:

```slang
uniform float4 tint;
uniform float  intensity;

[shader("fragment")]
float4 main(float2 uv : TEXCOORD0, float4 color : COLOR0) : SV_Target
{
    float glow = smoothstep(0.5, 0.0, length(uv - 0.5));
    return lerp(color, tint, glow * intensity);
}
```

**3. Use the compiled shader**

The build pipeline outputs `assets/build/shaders/glow.frag.glsl`. Include it:

```rust
static GLOW: ShaderAsset = ShaderAsset::Source {
    file_name: "glow.glsl",
    fragment: include_str!("../assets/build/shaders/glow.frag.glsl"),
};

ui.element().width(fixed!(120.0)).height(fixed!(120.0))
    .effect(&GLOW, |s| {
        s.uniform("tint", [1.0f32, 0.8, 0.2, 1.0])
         .uniform("intensity", 0.7f32);
    })
    .empty();
```

### Build Pipeline Details

| Extension         | Pipeline                                      | External Tools |
|-------------------|-----------------------------------------------|----------------|
| `.slang`          | slangc → SPIR-V → spirv-cross2 → GLSL ES 1.00 | `slangc`       |
| `.hlsl`           | slangc → SPIR-V → spirv-cross2 → GLSL ES 1.00 | `slangc`       |
| `.glsl` / `.frag` | Copy with generated header                    | —              |

Content hashes in `build/shaders/hashes.json` enable incremental builds — only changed files recompile.

Custom languages can be added with `.override_file_type_handler()`:

```rust
ply_engine::shader_build::ShaderBuild::new()
    .override_file_type_handler(".wgsl", |file_path, output_dir| {
        my_compiler::compile(file_path, output_dir);
        vec!["shaders/includes/**/*.wgsl".to_string()] // dependency globs
    })
    .build();
```

## WebAssembly

Here is a quick bash script to bring your ply-engine app to the web, be sure to replace [APPNAME] with the name of your app:
```bash
# Builds a folder build/web containing
# - assets/
# - index.html
# - app.wasm (built by cargo)
# - ply_bundle.js (downloaded from https://github.com/TheRedDeveloper/ply-engine/blob/main/js/ply_bundle.js)
#!/bin/bash
set -e
cargo build --release --target wasm32-unknown-unknown
mkdir -p build/web
cp -r assets build/web/
cp index.html build/web/
cp target/wasm32-unknown-unknown/release/[APPNAME].wasm build/web/app.wasm
curl https://raw.githubusercontent.com/TheRedDeveloper/ply-engine/refs/heads/main/js/ply_bundle.js -o build/web/ply_bundle.js
```

You'll need to make an index.html:
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>My App</title>
    <style>
        html,
        body,
        canvas {
            margin: 0;
            padding: 0;
            width: 100%;
            height: 100%;
            overflow: hidden;
            position: absolute;
            background: black;
            z-index: 0;
        }
    </style>
</head>
<body>
    <canvas id="glcanvas" tabindex="0"></canvas>
    <script src="ply_bundle.js"></script>
    <script>load("app.wasm");</script>
</body>
</html>
```

## License

[Zero-Clause BSD](LICENSE.md)
