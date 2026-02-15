## Example 1
### Old syntax:
```rust
for &(label, level) in &[("Road", 1), ("Wall", 2), ("Tower", 3)] {
    clay.with(
        &Declaration::new()
            .layout()
                .width(grow!())
                .height(fixed!(w * 0.06))
                .direction(LayoutDirection::LeftToRight)
                .child_gap((w * 0.02) as u16)
                .child_alignment(Alignment::new(LayoutAlignmentX::Left, LayoutAlignmentY::Center))
            .end(),
        |clay| {
            clay.text_literal(label,
                TextConfig::new().font_size((w * 0.03) as u16).color(white).end());
            clay.with(
                &Declaration::new()
                    .layout().width(grow!()).height(fixed!(w * 0.03)).end()
                    .corner_radius().all(w * 0.015).end()
                    .background_color(color_from_hex_rgb!(0x555555)),
                |clay| {
                    clay.with(
                        &Declaration::new()
                            .layout()
                                .width(fixed!(w * 0.5 * level as f32 / 3.0))
                                .height(grow!())
                            .end()
                            .corner_radius().all(w * 0.015).end()
                            .background_color(green),
                        |_| {},
                    );
                },
            );
        },
    );
}
```
### New syntax:
```rust
for &(label, level) in &[("Road", 1), ("Wall", 2), ("Tower", 3)] {
    ui.element().width(grow!()).height(fixed!(w * 0.06)) // Width and height are moved out of the layout
        .layout(|l| l // Instead of .end(), we use closures now
            .direction(LeftToRight)
            .gap(w * 0.02)
            .align(Left, Center)
        )
        .children(|ui| {
            ui.text(label, |t| t // Closure-based text configuration as well
                .font_size(w * 0.03)
                .color(white)
            );
            ui.element().width(grow!()).height(fixed!(w * 0.03))
                .corner_radius(w * 0.015) // This accepts an Into<CornerRadii>, so you could set them individually if you wanted to
                .color(0x555555) // This accepts Into<Color> naturally and i32s are hex colors
                .children(|ui| {
                    ui.element()
                        .width(fixed!(w * 0.5 * level as f32 / 3.0))
                        .height(grow!())
                        .corner_radius(w * 0.015)
                        .color(green)
                        .empty(); // This is the alternative for no children
                });
        }); // When the children closure ends it adds the element to the parent, the builder can be ended with .empty() or .children(...)
}
```
## Example 2
### Old syntax:
```rust
if let Some(ref anim) = state.charm_trash_anim {
    let elapsed = (get_time() - anim.start_time) as f32;
    if let Some((ax, ay, scale)) = anim.eval(elapsed) {
        let sz = player_charm_sz * scale;
        if sz > 0.5 {
            let anim_color = charm_color(&anim.charm);
            let anim_label = charm_label(&anim.charm);
            let anim_font = (sz * 0.3) as u16;
            layout.with(
                &Declaration::new()
                    .layout()
                        .width(fixed!(sz))
                        .height(fixed!(sz))
                        .child_alignment(Alignment::new(LayoutAlignmentX::Center, LayoutAlignmentY::Center))
                    .end()
                    .floating()
                        .attach_to(FloatingAttachToElement::Root)
                        .attach_points(FloatingAttachPointType::CenterCenter, FloatingAttachPointType::LeftTop)
                        .offset(Vector2::new(ax, ay))
                        .pointer_capture_mode(PointerCaptureMode::Passthrough)
                        .z_index(110)
                    .end()
                    .corner_radius().all(sz / 2.0).end()
                    .background_color(anim_color),
                |layout| {
                    if anim_font > 0 {
                        layout.text_literal(
                            anim_label,
                            TextConfig::new()
                                .font_size(anim_font)
                                .color(white_text)
                                .end(),
                        );
                    }
                },
            );
        }
    }
}
```
### New syntax:
```rust
if let Some(ref anim) = state.charm_trash_anim {
    let elapsed = (get_time() - anim.start_time) as f32;
    if let Some((ax, ay, scale)) = anim.eval(elapsed) {
        let sz = player_charm_sz * scale;
        if sz > 0.5 {
            let anim_color = charm_color(&anim.charm);
            let anim_label = charm_label(&anim.charm);
            let anim_font = (sz * 0.3) as u16;
            ui.element().width(fixed!(sz)).height(fixed!(sz))
                .layout(|l| l.align(Center, Center))
                .floating(|f| f
                    .attach(Root)
                    .anchor(CenterCenter, LeftTop)
                    .offset(ax, ay)
                    .passthrough()
                    .z_index(110)
                )
                .corner_radius(sz / 2.0)
                .background_color(anim_color)
                .children(|ui| {
                    if anim_font > 0 {
                        ui.text(anim_label, |t| t
                            .font_size(anim_font)
                            .color(white_text)
                        );
                    }
                });
        }
    }
}
```
