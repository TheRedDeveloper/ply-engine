# Ply Engine

A pure Rust UI layout engine built on [macroquad](https://github.com/not-fl3/macroquad), inspired by [Clay](https://github.com/nicbarker/clay). Blazingly fast, safe, and ready for desktop and web.

## Features

- **Pure Rust** — no C bindings, no FFI
- **Macroquad renderer** — antialiased rounded rectangles, borders, clipping, images, and text out of the box
- **TextureManager** — automatic GPU texture caching with configurable eviction, supports file paths and embedded bytes
- **TinyVG support** — render resolution-independent vector graphics via the `tinyvg` feature
- **Text styling** — rich inline markup for color, effects (wave, jitter, gradient, …), and animations (type-in, fade, scale) via the `text-styling` feature
- **WebAssembly** — ship to the browser, guide below.
- **Debug view** — detailed layout overlay

## Installation

```toml
[dependencies]
ply-engine = "0.1"

# Optional features:
# ply-engine = { version = "0.1", features = ["text-styling", "tinyvg"] }
```

## Quick Start

```rust
use macroquad::prelude::*;
use ply_engine::{
    fixed, grow,
    Color, Declaration, Ply,
    layout::{Alignment, LayoutAlignmentX, LayoutAlignmentY, LayoutDirection},
    renderer::render,
    text::TextConfig,
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
                // WebGL2 is required for TinyVG support
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
    let mut ply = Ply::new((screen_width(), screen_height()).into(), fonts.clone());

    loop {
        clear_background(BLACK);

        // Begin layout
        let mut ply = ply.begin::<()>();

        // A column filling the screen
        ply.with(
            &Declaration::new()
                .layout()
                    .width(grow!())
                    .height(grow!())
                    .direction(LayoutDirection::TopToBottom)
                    .child_gap(16)
                    .child_alignment(Alignment::new(
                        LayoutAlignmentX::Center,
                        LayoutAlignmentY::Center,
                    ))
                .end(),
            |ply| {
                // A colored box
                ply.with(
                    &Declaration::new()
                        .layout()
                            .width(fixed!(200.0))
                            .height(fixed!(100.0))
                        .end()
                        .corner_radius().all(8.0).end()
                        .background_color(Color::u_rgb(0x45, 0xA8, 0x5A)),
                    |_| {},
                );

                // Some text below that box
                ply.text_literal(
                    "Hello, Ply!",
                    TextConfig::new()
                        .font_size(32)
                        .color(Color::u_rgb(0xFF, 0xFF, 0xFF))
                        .end(),
                );
            },
        );

        // Render all commands through the macroquad renderer
        render(ply.end(), &fonts, |_| {}).await;

        next_frame().await;
    }
}
```

## Layout API

Layouts are built with a closure-based nesting API. The `Declaration` builder configures each element's sizing, alignment, borders, background color, images, and more.

```rust
ply.with(
    &Declaration::new()
        .id(ply.id("sidebar"))
        .layout()
            .width(fixed!(250.0))
            .height(grow!())
            .direction(LayoutDirection::TopToBottom)
            .child_gap(8)
            .padding(Padding::all(16))
        .end()
        .border()
            .color(Color::u_rgb(0x33, 0x33, 0x33))
            .right(2)
        .end()
        .background_color(Color::u_rgb(0x1A, 0x1A, 0x2E)),
    |ply| {
        // children go here
    },
);
```

Sizing helpers: `fixed!(px)`, `grow!()`, `fit!()`, and `Sizing::Percent(0.0..=1.0)`.

## TextureManager

`TEXTURE_MANAGER` is a global, thread-safe texture cache. The renderer uses it automatically for images and TinyVG assets — you can also use it directly.

```rust
// Load and cache a texture (async)
let tex = TEXTURE_MANAGER.lock().unwrap().get_or_load("sprites/hero.png").await;

// Embed bytes at compile time
static LOGO: Asset = Asset::Bytes {
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

Enable with `features = ["tinyvg"]`. Reference `.tvg` files through the `Asset` enum and they render at any resolution. Convert your svgs into `.tvg`s with the [official tools](https://tinyvg.tech/) and enjoy ultra-compact assets with blazingly fast rendering.

```rust
static ICON: Asset = Asset::Bytes {
    file_name: "icon.tvg",
    data: include_bytes!("../assets/icon.tvg"),
};
```

## WebAssembly

Here is a quick bash script to bring your ply-engine app to the web:
```bash
# Builds a folder build/web containing
# - assets/
# - index.html
# - client.wasm (built by cargo)
# - mq_js_bundle.js (downloaded from https://github.com/not-fl3/macroquad/blob/master/js/mq_js_bundle.js)
#!/bin/bash
set -e
cargo build --release --target wasm32-unknown-unknown
mkdir -p build/web
cp -r assets build/web/
cp index.html build/web/
cp target/wasm32-unknown-unknown/release/client.wasm build/web/client.wasm
curl https://raw.githubusercontent.com/not-fl3/macroquad/refs/heads/master/js/mq_js_bundle.js -o build/web/mq_js_bundle.js
```

You'll need to make an index.html, be sure to replace [APPNAME] with the name of your app:
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Stratum</title>
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
    <canvas id="glcanvas"></canvas>
    <script src="mq_js_bundle.js"></script>
    <script>load("[APPNAME].wasm");</script>
</body>
</html>
```

## License

[Zero-Clause BSD](LICENSE.md)
