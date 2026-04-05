# 1.0 → 1.1

## Migration Guide

- `Vec2` support
  - Replace `.offset({x}, {y})` with `.offset(({x}, {y}))`
  - Replace `.pivot({x}, {y})` with `.pivot(({x}, {y}))`
- Toggle-likes
  - Replace `password(true)` with `password()`
  - Replace `multiline(true)` with `multiline()`

## Changelog

### Grow weights and named sizing arguments (#6, #8)

- `grow!(min, max, weight)` now supports a weight argument.
- `grow!` named args: `min:`, `max:`, `weight:`.
- `fit!` named args: `min:`, `max:`.

```rust
ui.element()
  .width(grow!(weight: 2.0))
  .height(fit!(max: 320.0))
  .empty();
```

### Wider `Vec2` support (#13)

- `FloatingBuilder::offset(impl Into<Vector2>)`
- `RotationBuilder::pivot(impl Into<Vector2>)`
- `Vector2` conversions:
  - `From<(f32, f32)> for Vector2`
  - `From<macroquad::prelude::Vec2> for Vector2`

```rust
ui.element()
  .floating(|f| f.offset((12.0, 24.0)))
  .rotation(|r| r.pivot((0.5, 0.5)).degrees(12.0))
  .empty();
```

### Aspect-ratio sizing (#2, #3)

- `ElementBuilder::contain(aspect_ratio: f32)`
- `ElementBuilder::cover(aspect_ratio: f32)`

```rust
ui.element().contain(16.0 / 9.0).empty();
ui.element().cover(16.0 / 9.0).empty();
```

### Wrapping layouts (#31)

- `LayoutBuilder::wrap()`
- `LayoutBuilder::wrap_gap(gap: u16)`

```rust
ui.element()
  .layout(|l| l.wrap().wrap_gap(8))
  .children(|ui| {
    // children wrap to the cross axis when needed
  });
```

### Programmatic scroll position (#27)

- `Ply::set_scroll_position(id, position)` where:
  - `id: impl Into<Id>`
  - `position: impl Into<Vector2>`

- Works for overflow scroll containers.
- Also works for text inputs.

```rust
ply.set_scroll_position("chat_panel", (0.0, 9_999.0));
ply.set_scroll_position("search_input", (120.0, 0.0));
```

### Overflow drag behavior (#11)

- `OverflowBuilder::no_drag_scroll()`

```rust
ui.element()
  .overflow(|o| o.scroll_y().no_drag_scroll())
  .children(|ui| {});
```

### Scrollbar API (#30)

- `OverflowBuilder::scrollbar(...)`
- `TextInputBuilder::scrollbar(...)`
- `ScrollbarBuilder`:
  - `width(f32)`
  - `corner_radius(f32)`
  - `thumb_color(Color)`
  - `track_color(Color)`
  - `min_thumb_size(f32)`
  - `hide_after_frames(u32)`

```rust
ui.element()
  .overflow(|o| {
    o.scroll_y().scrollbar(|s| {
      s.width(4.0)
       .corner_radius(2.0)
       .thumb_color(0xFFFFFF)
       .track_color(0x111111)
       .min_thumb_size(18.0)
       .hide_after_frames(60)
    })
  })
  .children(|ui| {});
```

### Drag-select mode (#10)

- `TextInputBuilder::drag_select()`

- Enables mouse drag text selection.
- Touch drag remains scroll behavior.

```rust
ui.element()
  .text_input(|t| t.multiline().drag_select())
  .empty();
```

### Toggle-like builder simplification (#41)

- `TextInputBuilder::password()` (replaces `password(true)`)
- `TextInputBuilder::multiline()` (replaces `multiline(true)`)

```rust
ui.element()
  .text_input(|t| t.multiline().password())
  .empty();
```

### Press/release one-frame queries (#24, #25)

- `Ui::just_pressed()`
- `Ply::is_just_pressed(id)`
- `Ui::just_released()`
- `Ply::is_just_released(id)`

- `just_pressed`/`is_just_pressed` are true only on press frame.
- `just_released`/`is_just_released` are true only on release frame.
- Keyboard activation (`Enter`/`Space`) is included.

```rust
ui.element().id("save").children(|ui| {
  if ui.just_pressed() {
    // run once on press
  }
  if ui.just_released() {
    // run once on release
  }
});

if ui.is_just_pressed("save") {}
if ui.is_just_released("save") {}
```

### Generic interpolation (#4)

- Trait: `Lerp` with `fn lerp(self, other, t)`
- Implementations: `f32`, `u16`, `Vector2`, `macroquad::Vec2`, `(f32, f32, f32, f32)`, `(u16, u16, u16, u16)`, `Color`
- Color methods:
  - `Color::lerp_srgb(...)`
  - `Color::lerp_oklab(...)`

```rust
let w = 100.0_f32.lerp(300.0, t);
let p = (0_u16, 0, 0, 0).lerp((16, 16, 16, 16), t);
let c = Color::from_hex(0x1E1E1E).lerp_oklab(Color::from_hex(0x4CC9F0), t);
```

### Standard easing set (#5)

- Quad: `ease_in_quad`, `ease_out_quad`, `ease_in_out_quad`
- Cubic: `ease_in_cubic`, `ease_out_cubic`, `ease_in_out_cubic`
- Quart: `ease_in_quart`, `ease_out_quart`, `ease_in_out_quart`
- Sine: `ease_in_sine`, `ease_out_sine`, `ease_in_out_sine`
- Expo: `ease_in_expo`, `ease_out_expo`, `ease_in_out_expo`
- Elastic: `ease_in_elastic`, `ease_out_elastic`, `ease_in_out_elastic`
- Bounce: `ease_in_bounce`, `ease_out_bounce`, `ease_in_out_bounce`
- Back: `ease_in_back`, `ease_out_back`, `ease_in_out_back`

```rust
let t = ease_out_cubic(raw_t);
ui.element().width(fixed!(100.0_f32.lerp(300.0, t))).empty();
```

### Storage API (#29)

- `storage` feature
  - `Storage::new(path).await`
  - `save_string(path, data).await`
  - `save_bytes(path, data).await`
  - `load_string(path).await`
  - `load_bytes(path).await`
  - `remove(path).await`
  - `export(path).await`

- Relative root paths are normalized and sandboxed.

```rust
let storage = Storage::new("my_app/data").await?;
storage.save_string("settings.json", "{\"v\":1}").await?;
let saved = storage.load_string("settings.json").await?;
storage.export("settings.json").await?;
```

### Jobs module (#45)

- `jobs::spawn(id, job, on_complete)`
- `jobs::is_running(id)`
- `jobs::list()`

- `on_complete` runs on the main thread during `ply.begin()`.

```rust
jobs::spawn(
  "save_game",
  || async move { Storage::new("my_app").await },
  |result| {
    // main-thread completion callback
    let _ = result;
  },
)?;

if jobs::is_running("save_game") {
  // show spinner
}
```

### Debug view width API (#46)

- `Ply::set_debug_view_width(width: f32)`

- Debug view got improved in general.

```rust
ply.set_debug_view_width(520.0);
```

### Cursor API in prelude (#19)

- `set_mouse_cursor`
- `CursorIcon`

```rust
use ply_engine::prelude::*;

set_mouse_cursor(CursorIcon::Pointer);
```

# 1.0.2 → 1.0.3

## Changelog

### Improved Borders (#14)

- `BorderBuilder::position(BorderPosition)`

- Border rendering got improved in general.

```rust
ui.element()
  .border(|b| b.color(RED).all(2).position(Inside))
  .empty();
```