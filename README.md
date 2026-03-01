<p align="center">
  <img src="https://raw.githubusercontent.com/TheRedDeveloper/ply-website/gh-pages/images/ply_logo.png" alt="Ply" width="200">
</p>

<h3 align="center">Beautiful UIs in Rust. Cross-platform. Dead simple.</h3>

<p align="center">
  <a href="https://plyx.iz.rs">Website</a> · <a href="https://plyx.iz.rs/docs/getting-started/">Docs</a> · <a href="https://plyx.iz.rs/examples/">Examples</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/ply-engine"><img src="https://img.shields.io/crates/v/ply-engine.svg" alt="crates.io"></a>
  <a href="LICENSE.md"><img src="https://img.shields.io/badge/license-0BSD-blue.svg" alt="License: 0BSD"></a>
</p>

---

Ply is an engine for building apps in Rust that run on Linux, macOS, Windows, Android, iOS, and the web. One codebase, every platform. GPU-accelerated rendering, text editing, styling, accessibility, shaders, networking, sound and more, made easy and fast.

```bash
cargo install plyx
plyx init
```

## What you get

```rust
ui.element().width(grow!()).height(grow!())
  .background_color(0x262220)
  .corner_radius(12.0)
  .layout(|l| l.direction(TopToBottom).padding(24).gap(16))
  .children(|ui| {
    ui.text("Hello, Ply!", |t| t.font_size(32).color(0xFFFFFF));
  });
```

Everything is an element. Builder pattern, closure-based children, one import. [Read the docs →](https://plyx.iz.rs/docs/getting-started/)

## Highlights

- **Layout engine**: Flexbox-like sizing, padding, gaps, alignment, scrolling, floating elements
- **Text input**: Cursor, selection, undo/redo, multiline, password mode, keyboard shortcuts
- **Rich text styling**: Inline colors, wave, pulse, gradient, typewriter, fade, works in inputs too
- **Shaders**: GLSL fragment shaders, built-in effects, SPIR-V build pipeline
- **Accessibility**: AccessKit on desktop, JS bridge on web. Screen readers, keyboard nav, focus rings, tab order, live regions
- **Debug view**: Chrome DevTools-style inspector. One line: `ply.set_debug_mode(true)`
- **Networking**: HTTP + WebSocket, polling-based, never blocks the UI, works everywhere
- **Images & vectors**: PNG, TinyVG vectors, `render_to_texture`, procedural vectors
- **Rotation**: Visual (children included) and shape (vertex-level)
- **Sound**: WAV/OGG playback, volume control, looping
- **Interactivity**: `ui.hovered()`, `ui.pressed()`, `ui.focused()` inline, callback events, ID-based queries

## Platforms

| Platform                        | Build command |
|---------------------------------|---------------|
| Desktop (Linux, macOS, Windows) | `cargo build` |
| Web (WASM)                      | `plyx web`    |
| Android                         | `plyx apk`    |
| iOS                             | `plyx ios`    |

## Feature flags

| Feature            | What it adds                                      |
|--------------------|---------------------------------------------------|
| `a11y`             | Screen reader support via AccessKit *(default)*   |
| `text-styling`     | Rich text with inline colors, animations, effects |
| `tinyvg`           | TinyVG vector graphics                            |
| `built-in-shaders` | Pre-made shader effects (foil, glow, CRT, etc.)   |
| `shader-build`     | Shader compilation pipeline (SPIR-V Cross)        |
| `net`              | HTTP and WebSocket                                |
| `net-json`         | JSON deserialization for network responses        |
| `audio`            | Sound playback (WAV, OGG)                         |

## Examples

See the [interactive examples](https://plyx.iz.rs/examples/) on the website:

- **Shader Playground** (207 lines): live GLSL editor with code highlighting
- **Snake** (295 lines)
- **Todo List** (242 lines)

## License

[Zero-Clause BSD](LICENSE.md). Use it for anything. No attribution required.
