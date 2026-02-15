use macroquad::prelude::*;
use crate::{math::BoundingBox, render_commands::{CornerRadii, RenderCommand, RenderCommandConfig}};

#[cfg(feature = "text-styling")]
use crate::text_styling::{parse_text_lines, render_styled_text, StyledSegment};
#[cfg(feature = "text-styling")]
use std::collections::HashMap;

const PIXELS_PER_POINT: f32 = 2.0;

#[cfg(feature = "text-styling")]
static ANIMATION_TRACKER: std::sync::LazyLock<std::sync::Mutex<HashMap<String, (usize, f64)>>> = std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// Represents an asset that can be loaded as a texture. This can be either a file path or embedded bytes.
#[derive(Debug)]
pub enum Asset {
    Path(&'static str), // For external assets
    Bytes{file_name: &'static str, data: &'static [u8]}, // For embedded assets
}
impl Asset {
    fn get_name(&self) -> &str {
        match self {
            Asset::Path(path) => path,
            Asset::Bytes { file_name, .. } => file_name,
        }
    }
}

/// Global TextureManager. Can also be used outside the renderer to manage your own macroquad textures.
pub static TEXTURE_MANAGER: std::sync::LazyLock<std::sync::Mutex<TextureManager>> = std::sync::LazyLock::new(|| std::sync::Mutex::new(TextureManager::new()));

/// Manages textures, loading and unloading them as needed. No manual management needed.
/// 
/// You can adjust `max_frames_not_used` to control how many frames a texture can go unused before being unloaded.
pub struct TextureManager {
    textures: std::collections::HashMap<String, TextureData>,
    pub max_frames_not_used: usize,
}
struct TextureData {
    pub frames_not_used: usize,
    pub texture: Texture2D,
}
impl TextureManager {
    pub fn new() -> Self {
        Self {
            textures: std::collections::HashMap::new(),
            max_frames_not_used: 1,
        }
    }

    /// Get a cached texture by its key.
    pub fn get(&mut self, path: &str) -> Option<&Texture2D> {
        if let Some(data) = self.textures.get_mut(path) {
            data.frames_not_used = 0;
            Some(&data.texture)
        } else {
            None
        }
    }

    /// Get the cached texture by its key, or load from a file path and cache it.
    pub async fn get_or_load(&mut self, path: &'static str) -> &Texture2D {
        if !self.textures.contains_key(path) {
            let texture = load_texture(path).await.unwrap();
            self.textures.insert(path.to_owned(), TextureData { frames_not_used: 0, texture });
        }

        let entry = self.textures.get_mut(path).unwrap();
        entry.frames_not_used = 0; // Reset frame not used counter
        &entry.texture
    }

    /// Get the cached texture by its key, or create it using the provided function and cache it.
    pub fn get_or_create<F>(&mut self, key: String, create_fn: F) -> &Texture2D 
    where F: FnOnce() -> Texture2D 
    {
        if !self.textures.contains_key(&key) {
            let texture = create_fn();
            self.textures.insert(key.clone(), TextureData { frames_not_used: 0, texture });
        }
        let entry = self.textures.get_mut(&key).unwrap();
        entry.frames_not_used = 0;
        &entry.texture
    }

    pub async fn get_or_create_async<F, Fut>(&mut self, key: String, create_fn: F) -> &Texture2D 
    where F: FnOnce() -> Fut,
          Fut: std::future::Future<Output = Texture2D>
    {
        if !self.textures.contains_key(&key) {
            let texture = create_fn().await;
            self.textures.insert(key.clone(), TextureData { frames_not_used: 0, texture });
        }
        let entry = self.textures.get_mut(&key).unwrap();
        entry.frames_not_used = 0;
        &entry.texture
    }

    /// Cache a texture with the given key.
    pub fn cache(&mut self, key: String, texture: Texture2D) -> &Texture2D {
        self.textures.insert(key.clone(), TextureData { frames_not_used: 0, texture: texture });
        &self.textures.get(&key).unwrap().texture
    }

    pub fn clean(&mut self) {
        self.textures.retain(|_, data| data.frames_not_used <= self.max_frames_not_used);

        for (_, data) in self.textures.iter_mut() {
            data.frames_not_used += 1;
        }
    }

    pub fn size(&self) -> usize {
        self.textures.len()
    }
}

fn ply_to_macroquad_color(ply_color: &crate::color::Color) -> Color {
    Color {
        r: ply_color.r / 255.0,
        g: ply_color.g / 255.0,
        b: ply_color.b / 255.0,
        a: ply_color.a / 255.0,
    }
}

fn draw_good_circle(x: f32, y: f32, r: f32, color: Color) {
    let sides = ((2.0 * std::f32::consts::PI * r) / PIXELS_PER_POINT).max(20.0);
    draw_poly(x, y, sides.min(255.0) as u8, r, 0.0, color);
}

struct RenderState {
    clip: Option<(i32, i32, i32, i32)>,
    #[cfg(feature = "text-styling")]
    style_stack: Vec<String>,
    #[cfg(feature = "text-styling")]
    total_char_index: usize,
}

impl RenderState {
    fn new() -> Self {
        Self {
            clip: None,
            #[cfg(feature = "text-styling")]
            style_stack: Vec::new(),
            #[cfg(feature = "text-styling")]
            total_char_index: 0,
        }
    }
}

fn rounded_rectangle_texture(cr: &CornerRadii, bb: &BoundingBox, clip: &Option<(i32, i32, i32, i32)>) -> Texture2D {
    let render_target = render_target_msaa(bb.width as u32, bb.height as u32);
    render_target.texture.set_filter(FilterMode::Linear);
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, bb.width, bb.height));
    cam.render_target = Some(render_target.clone());
    set_camera(&cam);
    unsafe {
        get_internal_gl().quad_gl.scissor(None);
    };

    // Edges
    // Top edge
    if cr.top_left > 0.0 || cr.top_right > 0.0 {
        draw_rectangle(
            cr.top_left,
            0.0,
            bb.width - cr.top_left - cr.top_right,
            bb.height - cr.bottom_left.max(cr.bottom_right),
            WHITE
        );
    }
    // Left edge
    if cr.top_left > 0.0 || cr.bottom_left > 0.0 {
        draw_rectangle(
            0.0,
            cr.top_left,
            bb.width - cr.top_right.max(cr.bottom_right),
            bb.height - cr.top_left - cr.bottom_left,
            WHITE
        );
    }
    // Bottom edge
    if cr.bottom_left > 0.0 || cr.bottom_right > 0.0 {
        draw_rectangle(
            cr.bottom_left,
            cr.top_left.max(cr.top_right),
            bb.width - cr.bottom_left - cr.bottom_right,
            bb.height - cr.top_left.max(cr.top_right),
            WHITE
        );
    }
    // Right edge
    if cr.top_right > 0.0 || cr.bottom_right > 0.0 {
        draw_rectangle(
            bb.width - cr.top_right,
            cr.top_right,
            bb.width - cr.top_left.max(cr.bottom_left),
            bb.height - cr.top_right - cr.bottom_right,
            WHITE
        );
    }

    // Corners
    // Top-left corner
    if cr.top_left > 0.0 {
        draw_good_circle(
            cr.top_left,
            cr.top_left,
            cr.top_left,
            WHITE,
        );
    }
    // Top-right corner
    if cr.top_right > 0.0 {
        draw_good_circle(
            bb.width - cr.top_right,
            cr.top_right,
            cr.top_right,
            WHITE,
        );
    }
    // Bottom-left corner
    if cr.bottom_left > 0.0 {
        draw_good_circle(
            cr.bottom_left,
            bb.height - cr.bottom_left,
            cr.bottom_left,
            WHITE,
        );
    }
    // Bottom-right corner
    if cr.bottom_right > 0.0 {
        draw_good_circle(
            bb.width - cr.bottom_right,
            bb.height - cr.bottom_right,
            cr.bottom_right,
            WHITE,
        );
    }

    set_default_camera();
    unsafe {
        get_internal_gl().quad_gl.scissor(*clip);
    }
    render_target.texture
}

/// Render a TinyVG image to a Texture2D, scaled to fit the given dimensions.
#[cfg(feature = "tinyvg")]
fn render_tinyvg_texture(
    tvg_data: &[u8],
    dest_width: f32,
    dest_height: f32,
    clip: &Option<(i32, i32, i32, i32)>,
) -> Option<Texture2D> {
    use tinyvg::{Decoder, format::{Command, Style, Segment, SegmentCommandKind, Point as TvgPoint, Color as TvgColor}};
    use kurbo::{BezPath, Point as KurboPoint, Vec2 as KurboVec2, ParamCurve, SvgArc, Arc as KurboArc, PathEl};
    use lyon::tessellation::{FillTessellator, FillOptions, VertexBuffers, BuffersBuilder, FillVertex, FillRule};
    use lyon::path::Path as LyonPath;
    use lyon::math::point as lyon_point;
    
    fn tvg_to_kurbo(p: TvgPoint) -> KurboPoint {
        KurboPoint::new(p.x, p.y)
    }
    
    let decoder = Decoder::new(std::io::Cursor::new(tvg_data));
    let image = match decoder.decode() {
        Ok(img) => img,
        Err(_) => return None,
    };
    
    let tvg_width = image.header.width as f32;
    let tvg_height = image.header.height as f32;
    let scale_x = dest_width / tvg_width;
    let scale_y = dest_height / tvg_height;
    
    let render_target = render_target_msaa(dest_width as u32, dest_height as u32);
    render_target.texture.set_filter(FilterMode::Linear);
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, dest_width, dest_height));
    cam.render_target = Some(render_target.clone());
    set_camera(&cam);
    unsafe {
        get_internal_gl().quad_gl.scissor(None);
    }
    
    let tvg_to_mq_color = |c: &TvgColor| -> Color {
        let (r, g, b, a) = c.as_rgba();
        Color::new(r as f32, g as f32, b as f32, a as f32)
    };
    
    let style_to_color = |style: &Style, color_table: &[TvgColor]| -> Color {
        match style {
            Style::FlatColor { color_index } => {
                color_table.get(*color_index).map(|c| tvg_to_mq_color(c)).unwrap_or(WHITE)
            }
            Style::LinearGradient { color_index_0, .. } |
            Style::RadialGradient { color_index_0, .. } => {
                color_table.get(*color_index_0).map(|c| tvg_to_mq_color(c)).unwrap_or(WHITE)
            }
        }
    };
    
    let draw_filled_path_lyon = |bezpath: &BezPath, color: Color| {
        let mut builder = LyonPath::builder();
        let mut subpath_started = false;
        
        for el in bezpath.iter() {
            match el {
                PathEl::MoveTo(p) => {
                    if subpath_started {
                        builder.end(false);
                    }
                    builder.begin(lyon_point((p.x * scale_x as f64) as f32, (p.y * scale_y as f64) as f32));
                    subpath_started = true;
                }
                PathEl::LineTo(p) => {
                    builder.line_to(lyon_point((p.x * scale_x as f64) as f32, (p.y * scale_y as f64) as f32));
                }
                PathEl::QuadTo(c, p) => {
                    builder.quadratic_bezier_to(
                        lyon_point((c.x * scale_x as f64) as f32, (c.y * scale_y as f64) as f32),
                        lyon_point((p.x * scale_x as f64) as f32, (p.y * scale_y as f64) as f32),
                    );
                }
                PathEl::CurveTo(c1, c2, p) => {
                    builder.cubic_bezier_to(
                        lyon_point((c1.x * scale_x as f64) as f32, (c1.y * scale_y as f64) as f32),
                        lyon_point((c2.x * scale_x as f64) as f32, (c2.y * scale_y as f64) as f32),
                        lyon_point((p.x * scale_x as f64) as f32, (p.y * scale_y as f64) as f32),
                    );
                }
                PathEl::ClosePath => {
                    builder.end(true);
                    subpath_started = false;
                }
            }
        }
        
        if subpath_started {
            builder.end(true);
        }
        
        let lyon_path = builder.build();
        
        let mut geometry: VertexBuffers<[f32; 2], u16> = VertexBuffers::new();
        let mut tessellator = FillTessellator::new();
        
        let fill_options = FillOptions::default().with_fill_rule(FillRule::NonZero);
        
        let result = tessellator.tessellate_path(
            &lyon_path,
            &fill_options,
            &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
                vertex.position().to_array()
            }),
        );
        
        if result.is_err() || geometry.indices.is_empty() {
            return;
        }
        
        let color_bytes = [(color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8, (color.a * 255.0) as u8];
        
        let vertices: Vec<Vertex> = geometry.vertices.iter().map(|pos| {
            Vertex {
                position: Vec3::new(pos[0], pos[1], 0.0),
                uv: Vec2::ZERO,
                color: color_bytes,
                normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
            }
        }).collect();
        
        let mesh = Mesh {
            vertices,
            indices: geometry.indices,
            texture: None,
        };
        draw_mesh(&mesh);
    };
    
    let draw_filled_polygon_tvg = |points: &[TvgPoint], color: Color| {
        if points.len() < 3 {
            return;
        }
        
        let mut builder = LyonPath::builder();
        builder.begin(lyon_point(points[0].x as f32 * scale_x, points[0].y as f32 * scale_y));
        for point in &points[1..] {
            builder.line_to(lyon_point(point.x as f32 * scale_x, point.y as f32 * scale_y));
        }
        builder.end(true);
        let lyon_path = builder.build();
        
        let mut geometry: VertexBuffers<[f32; 2], u16> = VertexBuffers::new();
        let mut tessellator = FillTessellator::new();
        
        let result = tessellator.tessellate_path(
            &lyon_path,
            &FillOptions::default(),
            &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
                vertex.position().to_array()
            }),
        );
        
        if result.is_err() || geometry.indices.is_empty() {
            return;
        }
        
        let color_bytes = [(color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8, (color.a * 255.0) as u8];
        
        let vertices: Vec<Vertex> = geometry.vertices.iter().map(|pos| {
            Vertex {
                position: Vec3::new(pos[0], pos[1], 0.0),
                uv: Vec2::ZERO,
                color: color_bytes,
                normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
            }
        }).collect();
        
        let mesh = Mesh {
            vertices,
            indices: geometry.indices,
            texture: None,
        };
        draw_mesh(&mesh);
    };
    
    let build_bezpath = |segments: &[Segment]| -> BezPath {
        let mut bezier = BezPath::new();
        for segment in segments {
            let start = tvg_to_kurbo(segment.start);
            let mut pen = start;
            bezier.move_to(pen);
            
            for cmd in &segment.commands {
                match &cmd.kind {
                    SegmentCommandKind::Line { end } => {
                        let end_k = tvg_to_kurbo(*end);
                        bezier.line_to(end_k);
                        pen = end_k;
                    }
                    SegmentCommandKind::HorizontalLine { x } => {
                        let end = KurboPoint::new(*x, pen.y);
                        bezier.line_to(end);
                        pen = end;
                    }
                    SegmentCommandKind::VerticalLine { y } => {
                        let end = KurboPoint::new(pen.x, *y);
                        bezier.line_to(end);
                        pen = end;
                    }
                    SegmentCommandKind::CubicBezier { control_0, control_1, point_1 } => {
                        let c0 = tvg_to_kurbo(*control_0);
                        let c1 = tvg_to_kurbo(*control_1);
                        let p1 = tvg_to_kurbo(*point_1);
                        bezier.curve_to(c0, c1, p1);
                        pen = p1;
                    }
                    SegmentCommandKind::QuadraticBezier { control, point_1 } => {
                        let c = tvg_to_kurbo(*control);
                        let p1 = tvg_to_kurbo(*point_1);
                        bezier.quad_to(c, p1);
                        pen = p1;
                    }
                    SegmentCommandKind::ArcEllipse { large, sweep, radius_x, radius_y, rotation, target } => {
                        let target_k = tvg_to_kurbo(*target);
                        let svg_arc = SvgArc {
                            from: pen,
                            to: target_k,
                            radii: KurboVec2::new(*radius_x, *radius_y),
                            x_rotation: *rotation,
                            large_arc: *large,
                            sweep: *sweep,
                        };
                        if let Some(arc) = KurboArc::from_svg_arc(&svg_arc) {
                            for seg in arc.append_iter(0.2) {
                                bezier.push(seg);
                            }
                        }
                        pen = target_k;
                    }
                    SegmentCommandKind::ClosePath => {
                        bezier.close_path();
                        pen = start;
                    }
                }
            }
        }
        bezier
    };
    
    let line_scale = (scale_x + scale_y) / 2.0;
    
    for cmd in &image.commands {
        match cmd {
            Command::FillPath { fill_style, path, outline } => {
                let fill_color = style_to_color(fill_style, &image.color_table);
                let bezpath = build_bezpath(path);
                draw_filled_path_lyon(&bezpath, fill_color);
                
                if let Some(outline_style) = outline {
                    let line_color = style_to_color(&outline_style.line_style, &image.color_table);
                    let line_width = outline_style.line_width as f32 * line_scale;
                    for segment in path {
                        let start = segment.start;
                        let mut pen = start;
                        for cmd in &segment.commands {
                            match &cmd.kind {
                                SegmentCommandKind::Line { end } => {
                                    draw_line(
                                        pen.x as f32 * scale_x, pen.y as f32 * scale_y,
                                        end.x as f32 * scale_x, end.y as f32 * scale_y,
                                        line_width, line_color
                                    );
                                    pen = *end;
                                }
                                SegmentCommandKind::HorizontalLine { x } => {
                                    let end = TvgPoint { x: *x, y: pen.y };
                                    draw_line(
                                        pen.x as f32 * scale_x, pen.y as f32 * scale_y,
                                        end.x as f32 * scale_x, end.y as f32 * scale_y,
                                        line_width, line_color
                                    );
                                    pen = end;
                                }
                                SegmentCommandKind::VerticalLine { y } => {
                                    let end = TvgPoint { x: pen.x, y: *y };
                                    draw_line(
                                        pen.x as f32 * scale_x, pen.y as f32 * scale_y,
                                        end.x as f32 * scale_x, end.y as f32 * scale_y,
                                        line_width, line_color
                                    );
                                    pen = end;
                                }
                                SegmentCommandKind::ClosePath => {
                                    draw_line(
                                        pen.x as f32 * scale_x, pen.y as f32 * scale_y,
                                        start.x as f32 * scale_x, start.y as f32 * scale_y,
                                        line_width, line_color
                                    );
                                    pen = start;
                                }
                                SegmentCommandKind::CubicBezier { control_0, control_1, point_1 } => {
                                    let c0 = tvg_to_kurbo(*control_0);
                                    let c1 = tvg_to_kurbo(*control_1);
                                    let p1 = tvg_to_kurbo(*point_1);
                                    let p0 = tvg_to_kurbo(pen);
                                    let cubic = kurbo::CubicBez::new(p0, c0, c1, p1);
                                    let steps = 16usize;
                                    let mut prev = p0;
                                    for i in 1..=steps {
                                        let t = i as f64 / steps as f64;
                                        let next = cubic.eval(t);
                                        draw_line(
                                            prev.x as f32 * scale_x, prev.y as f32 * scale_y,
                                            next.x as f32 * scale_x, next.y as f32 * scale_y,
                                            line_width, line_color
                                        );
                                        prev = next;
                                    }
                                    pen = *point_1;
                                }
                                SegmentCommandKind::QuadraticBezier { control, point_1 } => {
                                    let c = tvg_to_kurbo(*control);
                                    let p1 = tvg_to_kurbo(*point_1);
                                    let p0 = tvg_to_kurbo(pen);
                                    let quad = kurbo::QuadBez::new(p0, c, p1);
                                    let steps = 12usize;
                                    let mut prev = p0;
                                    for i in 1..=steps {
                                        let t = i as f64 / steps as f64;
                                        let next = quad.eval(t);
                                        draw_line(
                                            prev.x as f32 * scale_x, prev.y as f32 * scale_y,
                                            next.x as f32 * scale_x, next.y as f32 * scale_y,
                                            line_width, line_color
                                        );
                                        prev = next;
                                    }
                                    pen = *point_1;
                                }
                                SegmentCommandKind::ArcEllipse { large, sweep, radius_x, radius_y, rotation, target } => {
                                    let target_k = tvg_to_kurbo(*target);
                                    let p0 = tvg_to_kurbo(pen);
                                    let svg_arc = SvgArc {
                                        from: p0,
                                        to: target_k,
                                        radii: KurboVec2::new(*radius_x, *radius_y),
                                        x_rotation: *rotation,
                                        large_arc: *large,
                                        sweep: *sweep,
                                    };
                                    if let Some(arc) = KurboArc::from_svg_arc(&svg_arc) {
                                        let mut prev = p0;
                                        for seg in arc.append_iter(0.2) {
                                            match seg {
                                                PathEl::LineTo(p) | PathEl::MoveTo(p) => {
                                                    draw_line(
                                                        prev.x as f32 * scale_x, prev.y as f32 * scale_y,
                                                        p.x as f32 * scale_x, p.y as f32 * scale_y,
                                                        line_width, line_color
                                                    );
                                                    prev = p;
                                                }
                                                PathEl::CurveTo(c0, c1, p) => {
                                                    // Flatten the curve
                                                    let cubic = kurbo::CubicBez::new(prev, c0, c1, p);
                                                    let steps = 8usize;
                                                    let mut prev_pt = prev;
                                                    for j in 1..=steps {
                                                        let t = j as f64 / steps as f64;
                                                        let next = cubic.eval(t);
                                                        draw_line(
                                                            prev_pt.x as f32 * scale_x, prev_pt.y as f32 * scale_y,
                                                            next.x as f32 * scale_x, next.y as f32 * scale_y,
                                                            line_width, line_color
                                                        );
                                                        prev_pt = next;
                                                    }
                                                    prev = p;
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    pen = *target;
                                }
                            }
                        }
                    }
                }
            }
            Command::FillRectangles { fill_style, rectangles, outline } => {
                let fill_color = style_to_color(fill_style, &image.color_table);
                for rect in rectangles {
                    draw_rectangle(
                        rect.x0 as f32 * scale_x,
                        rect.y0 as f32 * scale_y,
                        rect.width() as f32 * scale_x,
                        rect.height() as f32 * scale_y,
                        fill_color
                    );
                }
                
                if let Some(outline_style) = outline {
                    let line_color = style_to_color(&outline_style.line_style, &image.color_table);
                    let line_width = outline_style.line_width as f32 * line_scale;
                    for rect in rectangles {
                        draw_rectangle_lines(
                            rect.x0 as f32 * scale_x,
                            rect.y0 as f32 * scale_y,
                            rect.width() as f32 * scale_x,
                            rect.height() as f32 * scale_y,
                            line_width, line_color
                        );
                    }
                }
            }
            Command::FillPolygon { fill_style, polygon, outline } => {
                let fill_color = style_to_color(fill_style, &image.color_table);
                draw_filled_polygon_tvg(polygon, fill_color);
                
                if let Some(outline_style) = outline {
                    let line_color = style_to_color(&outline_style.line_style, &image.color_table);
                    let line_width = outline_style.line_width as f32 * line_scale;
                    for i in 0..polygon.len() {
                        let next = (i + 1) % polygon.len();
                        draw_line(
                            polygon[i].x as f32 * scale_x, polygon[i].y as f32 * scale_y,
                            polygon[next].x as f32 * scale_x, polygon[next].y as f32 * scale_y,
                            line_width, line_color
                        );
                    }
                }
            }
            Command::DrawLines { line_style, line_width, lines } => {
                let line_color = style_to_color(line_style, &image.color_table);
                for line in lines {
                    draw_line(
                        line.p0.x as f32 * scale_x, line.p0.y as f32 * scale_y,
                        line.p1.x as f32 * scale_x, line.p1.y as f32 * scale_y,
                        *line_width as f32 * line_scale, line_color
                    );
                }
            }
            Command::DrawLineLoop { line_style, line_width, close_path, points } => {
                let line_color = style_to_color(line_style, &image.color_table);
                for i in 0..points.len().saturating_sub(1) {
                    draw_line(
                        points[i].x as f32 * scale_x, points[i].y as f32 * scale_y,
                        points[i+1].x as f32 * scale_x, points[i+1].y as f32 * scale_y,
                        *line_width as f32 * line_scale, line_color
                    );
                }
                if *close_path && points.len() >= 2 {
                    let last = points.len() - 1;
                    draw_line(
                        points[last].x as f32 * scale_x, points[last].y as f32 * scale_y,
                        points[0].x as f32 * scale_x, points[0].y as f32 * scale_y,
                        *line_width as f32 * line_scale, line_color
                    );
                }
            }
            Command::DrawLinePath { line_style, line_width, path } => {
                let line_color = style_to_color(line_style, &image.color_table);
                let scaled_line_width = *line_width as f32 * line_scale;
                // Draw line path by tracing segments directly
                for segment in path {
                    let start = segment.start;
                    let mut pen = start;
                    for cmd in &segment.commands {
                        match &cmd.kind {
                            SegmentCommandKind::Line { end } => {
                                draw_line(
                                    pen.x as f32 * scale_x, pen.y as f32 * scale_y,
                                    end.x as f32 * scale_x, end.y as f32 * scale_y,
                                    scaled_line_width, line_color
                                );
                                pen = *end;
                            }
                            SegmentCommandKind::HorizontalLine { x } => {
                                let end = TvgPoint { x: *x, y: pen.y };
                                draw_line(
                                    pen.x as f32 * scale_x, pen.y as f32 * scale_y,
                                    end.x as f32 * scale_x, end.y as f32 * scale_y,
                                    scaled_line_width, line_color
                                );
                                pen = end;
                            }
                            SegmentCommandKind::VerticalLine { y } => {
                                let end = TvgPoint { x: pen.x, y: *y };
                                draw_line(
                                    pen.x as f32 * scale_x, pen.y as f32 * scale_y,
                                    end.x as f32 * scale_x, end.y as f32 * scale_y,
                                    scaled_line_width, line_color
                                );
                                pen = end;
                            }
                            SegmentCommandKind::ClosePath => {
                                draw_line(
                                    pen.x as f32 * scale_x, pen.y as f32 * scale_y,
                                    start.x as f32 * scale_x, start.y as f32 * scale_y,
                                    scaled_line_width, line_color
                                );
                                pen = start;
                            }
                            // For curves, we need to flatten them for line drawing
                            SegmentCommandKind::CubicBezier { control_0, control_1, point_1 } => {
                                let c0 = tvg_to_kurbo(*control_0);
                                let c1 = tvg_to_kurbo(*control_1);
                                let p1 = tvg_to_kurbo(*point_1);
                                let p0 = tvg_to_kurbo(pen);
                                let cubic = kurbo::CubicBez::new(p0, c0, c1, p1);
                                let steps = 16usize;
                                let mut prev = p0;
                                for i in 1..=steps {
                                    let t = i as f64 / steps as f64;
                                    let next = cubic.eval(t);
                                    draw_line(
                                        prev.x as f32 * scale_x, prev.y as f32 * scale_y,
                                        next.x as f32 * scale_x, next.y as f32 * scale_y,
                                        scaled_line_width, line_color
                                    );
                                    prev = next;
                                }
                                pen = *point_1;
                            }
                            SegmentCommandKind::QuadraticBezier { control, point_1 } => {
                                let c = tvg_to_kurbo(*control);
                                let p1 = tvg_to_kurbo(*point_1);
                                let p0 = tvg_to_kurbo(pen);
                                let quad = kurbo::QuadBez::new(p0, c, p1);
                                let steps = 12usize;
                                let mut prev = p0;
                                for i in 1..=steps {
                                    let t = i as f64 / steps as f64;
                                    let next = quad.eval(t);
                                    draw_line(
                                        prev.x as f32 * scale_x, prev.y as f32 * scale_y,
                                        next.x as f32 * scale_x, next.y as f32 * scale_y,
                                        scaled_line_width, line_color
                                    );
                                    prev = next;
                                }
                                pen = *point_1;
                            }
                            SegmentCommandKind::ArcEllipse { large, sweep, radius_x, radius_y, rotation, target } => {
                                let target_k = tvg_to_kurbo(*target);
                                let p0 = tvg_to_kurbo(pen);
                                let svg_arc = SvgArc {
                                    from: p0,
                                    to: target_k,
                                    radii: KurboVec2::new(*radius_x, *radius_y),
                                    x_rotation: *rotation,
                                    large_arc: *large,
                                    sweep: *sweep,
                                };
                                if let Some(arc) = KurboArc::from_svg_arc(&svg_arc) {
                                    let mut prev = p0;
                                    for seg in arc.append_iter(0.2) {
                                        match seg {
                                            PathEl::LineTo(p) | PathEl::MoveTo(p) => {
                                                draw_line(
                                                    prev.x as f32 * scale_x, prev.y as f32 * scale_y,
                                                    p.x as f32 * scale_x, p.y as f32 * scale_y,
                                                    scaled_line_width, line_color
                                                );
                                                prev = p;
                                            }
                                            PathEl::CurveTo(c0, c1, p) => {
                                                // Flatten the curve
                                                let cubic = kurbo::CubicBez::new(prev, c0, c1, p);
                                                let steps = 8usize;
                                                let mut prev_pt = prev;
                                                for j in 1..=steps {
                                                    let t = j as f64 / steps as f64;
                                                    let next = cubic.eval(t);
                                                    draw_line(
                                                        prev_pt.x as f32 * scale_x, prev_pt.y as f32 * scale_y,
                                                        next.x as f32 * scale_x, next.y as f32 * scale_y,
                                                        scaled_line_width, line_color
                                                    );
                                                    prev_pt = next;
                                                }
                                                prev = p;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                pen = *target;
                            }
                        }
                    }
                }
            }
        }
    }
    
    set_default_camera();
    unsafe {
        get_internal_gl().quad_gl.scissor(*clip);
    }
    
    Some(render_target.texture)
}

fn resize(texture: &Texture2D, height: f32, width: f32, clip: &Option<(i32, i32, i32, i32)>) -> Texture2D {
    let render_target = render_target_msaa(width as u32, height as u32);
    render_target.texture.set_filter(FilterMode::Linear);
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, width, height));
    cam.render_target = Some(render_target.clone());
    set_camera(&cam);
    unsafe {
        get_internal_gl().quad_gl.scissor(None);
    };
    draw_texture_ex(
        texture,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(Vec2::new(width, height)),
            flip_y: true,
            ..Default::default()
        },
    );
    set_default_camera();
    unsafe {
        get_internal_gl().quad_gl.scissor(*clip);
    }
    render_target.texture
}

pub async fn render<'a, CustomElementData: 'a>(
    commands: impl Iterator<Item = RenderCommand<'a, CustomElementData>>,
    fonts: &[Font],
    handle_custom_command: impl Fn(&RenderCommand<'a, CustomElementData>),
) {
    let mut state = RenderState::new();
    for command in commands {
        match &command.config {
            RenderCommandConfig::Image(image) => {
                let bb = command.bounding_box;
                let cr = &image.corner_radii;
                let mut tint = ply_to_macroquad_color(&image.background_color);
                if tint == Color::new(0.0, 0.0, 0.0, 0.0) {
                    tint = Color::new(1.0, 1.0, 1.0, 1.0);
                }
                
                let mut manager = TEXTURE_MANAGER.lock().unwrap();

                #[cfg(feature = "tinyvg")]
                let is_tvg = image.data.get_name().to_lowercase().ends_with(".tvg");
                #[cfg(not(feature = "tinyvg"))]
                let is_tvg = false;

                #[cfg(feature = "tinyvg")]
                if is_tvg {
                    let key = format!(
                        "tvg:{}:{}:{}:{}:{}:{}:{}:{:?}",
                        image.data.get_name(),
                        bb.width,
                        bb.height,
                        cr.top_left,
                        cr.top_right,
                        cr.bottom_left,
                        cr.bottom_right,
                        state.clip
                    );
                    let has_corner_radii = cr.top_left > 0.0 || cr.top_right > 0.0 || cr.bottom_left > 0.0 || cr.bottom_right > 0.0;
                    let texture = if !has_corner_radii {
                        match image.data {
                            Asset::Path(path) => {
                                manager.get_or_create_async(key, async || {
                                    match load_file(path).await {
                                        Ok(tvg_bytes) => {
                                            if let Some(tvg_texture) = render_tinyvg_texture(&tvg_bytes, bb.width, bb.height, &state.clip) {
                                                tvg_texture
                                            } else {
                                                warn!("Failed to load TinyVG image: {}", path);
                                                Texture2D::from_rgba8(1, 1, &[0, 0, 0, 0])
                                            }
                                        }
                                        Err(error) => {
                                            warn!("Failed to load TinyVG file: {}. Error: {}", path, error);
                                            Texture2D::from_rgba8(1, 1, &[0, 0, 0, 0])
                                        }
                                    }
                                }).await
                            }
                            Asset::Bytes { file_name, data: tvg_bytes } => {
                                manager.get_or_create(key, || {
                                    if let Some(tvg_texture) = render_tinyvg_texture(&tvg_bytes, bb.width, bb.height, &state.clip) {
                                        tvg_texture
                                    } else {
                                        warn!("Failed to load TinyVG image: {}", file_name);
                                        Texture2D::from_rgba8(1, 1, &[0, 0, 0, 0])
                                    }
                                })
                            }
                        }
                        
                    } else {
                        let zerocr_key = format!(
                            "tvg:{}:{}:{}:{}:{}:{}:{}:{:?}",
                            image.data.get_name(),
                            bb.width,
                            bb.height,
                            0.0,
                            0.0,
                            0.0,
                            0.0,
                            state.clip
                        );
                        
                        let base_texture = if let Some(cached) = manager.get(&zerocr_key) {
                            cached
                        } else {
                            match image.data {
                                Asset::Path(path) => {
                                    match load_file(path).await {
                                        Ok(tvg_bytes) => {
                                            let texture = if let Some(tvg_texture) = render_tinyvg_texture(&tvg_bytes, bb.width, bb.height, &state.clip) {
                                                tvg_texture
                                            } else {
                                                warn!("Failed to load TinyVG image: {}", path);
                                                Texture2D::from_rgba8(1, 1, &[0, 0, 0, 0])
                                            };
                                            manager.cache(zerocr_key.clone(), texture)
                                        }
                                        Err(error) => {
                                            warn!("Failed to load TinyVG file: {}. Error: {}", path, error);
                                            manager.cache(zerocr_key.clone(), Texture2D::from_rgba8(1, 1, &[0, 0, 0, 0]))
                                        }
                                    }
                                }
                                Asset::Bytes { file_name, data: tvg_bytes } => {
                                    let texture = if let Some(tvg_texture) = render_tinyvg_texture(&tvg_bytes, bb.width, bb.height, &state.clip) {
                                        tvg_texture
                                    } else {
                                        warn!("Failed to load TinyVG image: {}", file_name);
                                        Texture2D::from_rgba8(1, 1, &[0, 0, 0, 0])
                                    };
                                    manager.cache(zerocr_key.clone(), texture)
                                }
                            }
                        }.clone();
                        
                        manager.get_or_create(key, || {
                            let mut tvg_image: Image = base_texture.get_texture_data();
                            let rounded_rect: Image = rounded_rectangle_texture(cr, &bb, &state.clip).get_texture_data();
                            
                            for i in 0..tvg_image.bytes.len()/4 {
                                let this_alpha = tvg_image.bytes[i * 4 + 3] as f32 / 255.0;
                                let mask_alpha = rounded_rect.bytes[i * 4 + 3] as f32 / 255.0;
                                tvg_image.bytes[i * 4 + 3] = (this_alpha * mask_alpha * 255.0) as u8;
                            }
                            Texture2D::from_image(&tvg_image)
                        })
                    };
                    
                    draw_texture_ex(
                        texture,
                        bb.x,
                        bb.y,
                        tint,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(bb.width, bb.height)),
                            flip_y: true,
                            ..Default::default()
                        },
                    );
                    continue;
                }

                if !is_tvg && cr.top_left == 0.0 && cr.top_right == 0.0 && cr.bottom_left == 0.0 && cr.bottom_right == 0.0 {
                    let texture = match image.data {
                        Asset::Path(path) => manager.get_or_load(path).await,
                        Asset::Bytes { file_name, data } => {
                            manager.get_or_create(file_name.to_string(), || {
                                Texture2D::from_file_with_format(data, None)
                            })
                        }
                    };
                    draw_texture_ex(
                        texture,
                        bb.x,
                        bb.y,
                        tint,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(bb.width, bb.height)),
                            ..Default::default()
                        },
                    );
                } else {
                    let source_texture = match image.data {
                        Asset::Path(path) => manager.get_or_load(path).await.clone(),
                        Asset::Bytes { file_name, data } => {
                            manager.get_or_create(file_name.to_string(), || {
                                Texture2D::from_file_with_format(data, None)
                            }).clone()
                        }
                    };
                    
                    let key = format!(
                        "image:{}:{}:{}:{}:{}:{}:{}:{:?}",
                        image.data.get_name(),
                        bb.width,
                        bb.height,
                        cr.top_left,
                        cr.top_right,
                        cr.bottom_left,
                        cr.bottom_right,
                        state.clip
                    );

                    let texture = manager.get_or_create(key, || {
                        let mut resized_image: Image = resize(&source_texture, bb.height, bb.width, &state.clip).get_texture_data();
                        let rounded_rect: Image = rounded_rectangle_texture(cr, &bb, &state.clip).get_texture_data();

                        for i in 0..resized_image.bytes.len()/4 {
                            let this_alpha = resized_image.bytes[i * 4 + 3] as f32 / 255.0;
                            let mask_alpha = rounded_rect.bytes[i * 4 + 3] as f32 / 255.0;
                            resized_image.bytes[i * 4 + 3] = (this_alpha * mask_alpha * 255.0) as u8;
                        }

                        Texture2D::from_image(&resized_image)
                    });

                    draw_texture_ex(
                        texture,
                        bb.x,
                        bb.y,
                        tint,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(bb.width, bb.height)),
                            ..Default::default()
                        },
                    );
                }
            }
            RenderCommandConfig::Rectangle(config) => {
                let bb = command.bounding_box;
                let color = ply_to_macroquad_color(&config.color);
                let cr = &config.corner_radii;

                if cr.top_left == 0.0 && cr.top_right == 0.0 && cr.bottom_left == 0.0 && cr.bottom_right == 0.0 {
                    draw_rectangle(
                        bb.x,
                        bb.y,
                        bb.width,
                        bb.height,
                        color
                    );
                } else if color.a == 1.0 {
                    // Edges
                    // Top edge
                    if cr.top_left > 0.0 || cr.top_right > 0.0 {
                        draw_rectangle(
                            bb.x + cr.top_left,
                            bb.y,
                            bb.width - cr.top_left - cr.top_right,
                            bb.height - cr.bottom_left.max(cr.bottom_right),
                            color
                        );
                    }
                    // Left edge
                    if cr.top_left > 0.0 || cr.bottom_left > 0.0 {
                        draw_rectangle(
                            bb.x,
                            bb.y + cr.top_left,
                            bb.width - cr.top_right.max(cr.bottom_right),
                            bb.height - cr.top_left - cr.bottom_left,
                            color
                        );
                    }
                    // Bottom edge
                    if cr.bottom_left > 0.0 || cr.bottom_right > 0.0 {
                        draw_rectangle(
                            bb.x + cr.bottom_left,
                            bb.y + cr.top_left.max(cr.top_right),
                            bb.width - cr.bottom_left - cr.bottom_right,
                            bb.height - cr.top_left.max(cr.top_right),
                            color
                        );
                    }
                    // Right edge
                    if cr.top_right > 0.0 || cr.bottom_right > 0.0 {
                        draw_rectangle(
                            bb.x + cr.top_left.max(cr.bottom_left),
                            bb.y + cr.top_right,
                            bb.width - cr.top_left.max(cr.bottom_left),
                            bb.height - cr.top_right - cr.bottom_right,
                            color
                        );
                    }

                    // Corners
                    // Top-left corner
                    if cr.top_left > 0.0 {
                        draw_good_circle(
                            bb.x + cr.top_left,
                            bb.y + cr.top_left,
                            cr.top_left,
                            color,
                        );
                    }
                    // Top-right corner
                    if cr.top_right > 0.0 {
                        draw_good_circle(
                            bb.x + bb.width - cr.top_right,
                            bb.y + cr.top_right,
                            cr.top_right,
                            color,
                        );
                    }
                    // Bottom-left corner
                    if cr.bottom_left > 0.0 {
                        draw_good_circle(
                            bb.x + cr.bottom_left,
                            bb.y + bb.height - cr.bottom_left,
                            cr.bottom_left,
                            color,
                        );
                    }
                    // Bottom-right corner
                    if cr.bottom_right > 0.0 {
                        draw_good_circle(
                            bb.x + bb.width - cr.bottom_right,
                            bb.y + bb.height - cr.bottom_right,
                            cr.bottom_right,
                            color,
                        );
                    }
                } else {
                    let mut manager = TEXTURE_MANAGER.lock().unwrap();
                    let key = format!(
                        "rect:{}:{}:{}:{}:{}:{}:{:?}",
                        bb.width,
                        bb.height,
                        cr.top_left,
                        cr.top_right,
                        cr.bottom_left,
                        cr.bottom_right,
                        state.clip
                    );

                    let texture = manager.get_or_create(key, || {
                        rounded_rectangle_texture(cr, &bb, &state.clip)
                    });

                    draw_texture_ex(
                        texture,
                        bb.x,
                        bb.y,
                        color,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(bb.width, bb.height)),
                            flip_y: true,
                            ..Default::default()
                        },
                    );
                }
            }
            #[cfg(feature = "text-styling")]
            RenderCommandConfig::Text(config) => {
                let bb = command.bounding_box;
                let font_size = config.font_size as f32;
                let font = Some(&fonts[config.font_id as usize]);
                let default_color = ply_to_macroquad_color(&config.color);

                let normal_render = || {
                    let x_scale = if config.letter_spacing > 0 {
                        bb.width / measure_text(
                            config.text,
                            font,
                            config.font_size as u16,
                            1.0
                        ).width
                    } else {
                        1.0
                    };
                    draw_text_ex(
                        config.text,
                        bb.x,
                        bb.y + bb.height,
                        TextParams {
                            font_size: config.font_size as u16,
                            font,
                            font_scale: 1.0,
                            font_scale_aspect: x_scale,
                            rotation: 0.0,
                            color: default_color
                        }
                    );
                };
                
                let mut in_style_def = false;
                let mut escaped = false;
                let mut failed = false;
                
                let mut text_buffer = String::new();
                let mut style_buffer = String::new();

                let line = config.text.to_string();
                let mut segments: Vec<StyledSegment> = Vec::new();

                for c in line.chars() {
                    if escaped {
                        if in_style_def {
                            style_buffer.push(c);
                        } else {
                            text_buffer.push(c);
                        }
                        escaped = false;
                        continue;
                    }

                    match c {
                        '\\' => {
                            escaped = true;
                        }
                        '{' => {
                            if in_style_def {
                                style_buffer.push(c); 
                            } else {
                                if !text_buffer.is_empty() {
                                    segments.push(StyledSegment {
                                        text: text_buffer.clone(),
                                        styles: state.style_stack.clone(),
                                    });
                                    text_buffer.clear();
                                }
                                in_style_def = true;
                            }
                        }
                        '|' => {
                            if in_style_def {
                                state.style_stack.push(style_buffer.clone());
                                style_buffer.clear();
                                in_style_def = false;
                            } else {
                                text_buffer.push(c);
                            }
                        }
                        '}' => {
                            if in_style_def {
                                style_buffer.push(c);
                            } else {
                                if !text_buffer.is_empty() {
                                    segments.push(StyledSegment {
                                        text: text_buffer.clone(),
                                        styles: state.style_stack.clone(),
                                    });
                                    text_buffer.clear();
                                }
                                
                                if state.style_stack.pop().is_none() {
                                    failed = true;
                                    break;
                                }
                            }
                        }
                        _ => {
                            if in_style_def {
                                style_buffer.push(c);
                            } else {
                                text_buffer.push(c);
                            }
                        }
                    }
                }
                if !(failed || in_style_def) {
                    if !text_buffer.is_empty() {
                        segments.push(StyledSegment {
                            text: text_buffer.clone(),
                            styles: state.style_stack.clone(),
                        });
                    }
                    
                    let time = get_time();
                    
                    let cursor_x = std::cell::Cell::new(bb.x);
                    let cursor_y = bb.y + bb.height;
                    let mut pending_renders = Vec::new();
                    
                    let x_scale = if config.letter_spacing > 0 {
                        bb.width / measure_text(
                            config.text,
                            Some(&fonts[config.font_id as usize]),
                            config.font_size as u16,
                            1.0
                        ).width
                    } else {
                        1.0
                    };
                    {
                        let mut tracker = ANIMATION_TRACKER.lock().unwrap();
                        render_styled_text(
                            &segments,
                            time,
                            font_size,
                            &mut *tracker,
                            &mut state.total_char_index,
                            |text, tr, style_color| {
                                let text_string = text.to_string();
                                let text_width = measure_text(&text_string, font, config.font_size as u16, 1.0).width;
                                
                                let color = Color::new(style_color.r, style_color.g, style_color.b, style_color.a);
                                let x = cursor_x.get();
                                
                                pending_renders.push((x, text_string, tr, color));
                                
                                cursor_x.set(x + text_width*x_scale);
                            },
                            |text, tr, style_color| {
                                let text_string = text.to_string();
                                let color = Color::new(style_color.r, style_color.g, style_color.b, style_color.a);
                                let x = cursor_x.get();
                                
                                draw_text_ex(
                                    &text_string,
                                    x + tr.x*x_scale,
                                    cursor_y + tr.y,
                                    TextParams {
                                        font_size: config.font_size as u16,
                                        font,
                                        font_scale: tr.scale_y.max(0.01),
                                        font_scale_aspect: if tr.scale_y > 0.01 { tr.scale_x / tr.scale_y * x_scale } else { x_scale },
                                        rotation: tr.rotation.to_radians(),
                                        color
                                    }
                                );
                            }
                        );
                    }
                    for (x, text_string, tr, color) in pending_renders {
                        draw_text_ex(
                            &text_string,
                            x + tr.x*x_scale,
                            cursor_y + tr.y,
                            TextParams {
                                font_size: config.font_size as u16,
                                font,
                                font_scale: tr.scale_y.max(0.01),
                                font_scale_aspect: if tr.scale_y > 0.01 { tr.scale_x / tr.scale_y * x_scale } else { x_scale },
                                rotation: tr.rotation.to_radians(),
                                color
                            }
                        );
                    }
                } else {
                    if in_style_def {
                        warn!("Style definition didn't end! Here is what we tried to render: {}", config.text);
                    } else if failed {
                        warn!("Encountered }} without opened style! Make sure to escape curly braces with \\. Here is what we tried to render: {}", config.text);
                    }
                    normal_render();
                }
            }
            #[cfg(not(feature = "text-styling"))]
            RenderCommandConfig::Text(config) => {
                let bb = command.bounding_box;
                let color = ply_to_macroquad_color(&config.color);

                let x_scale = if config.letter_spacing > 0 {
                    bb.width / measure_text(
                        config.text,
                        Some(&fonts[config.font_id as usize]),
                        config.font_size as u16,
                        1.0
                    ).width
                } else {
                    1.0
                };
                draw_text_ex(
                    &config.text,
                    bb.x,
                    bb.y + bb.height,
                    TextParams {
                        font_size: config.font_size as u16,
                        font: Some(&fonts[config.font_id as usize]),
                        font_scale: 1.0,
                        font_scale_aspect: x_scale,
                        rotation: 0.0,
                        color
                    }
                );
            }
            RenderCommandConfig::Border(config) => {
                let bb = command.bounding_box;
                let bw = &config.width;
                let cr = &config.corner_radii;
                let color = ply_to_macroquad_color(&config.color);
                if cr.top_left == 0.0 && cr.top_right == 0.0 && cr.bottom_left == 0.0 && cr.bottom_right == 0.0 {
                    if bw.left == bw.right && bw.left == bw.top && bw.left == bw.bottom {
                        let border_width = bw.left as f32;
                        draw_rectangle_lines(
                            bb.x - border_width / 2.0,
                            bb.y - border_width / 2.0,
                            bb.width + border_width,
                            bb.height + border_width,
                            border_width,
                            color
                        );
                    } else {
                        // Top edge
                        draw_line(
                            bb.x,
                            bb.y - bw.top as f32 / 2.0,
                            bb.x + bb.width,
                            bb.y - bw.top as f32 / 2.0,
                            bw.top as f32,
                            color
                        );
                        // Left edge
                        draw_line(
                            bb.x - bw.left as f32 / 2.0,
                            bb.y,
                            bb.x - bw.left as f32 / 2.0,
                            bb.y + bb.height,
                            bw.left as f32,
                            color
                        );
                        // Bottom edge
                        draw_line(
                            bb.x,
                            bb.y + bb.height + bw.bottom as f32 / 2.0,
                            bb.x + bb.width,
                            bb.y + bb.height + bw.bottom as f32 / 2.0,
                            bw.bottom as f32,
                            color
                        );
                        // Right edge
                        draw_line(
                            bb.x + bb.width + bw.right as f32 / 2.0,
                            bb.y,
                            bb.x + bb.width + bw.right as f32 / 2.0,
                            bb.y + bb.height,
                            bw.right as f32,
                            color
                        );
                    }
                } else {
                    // Edges
                    // Top edge
                    draw_line(
                        bb.x + cr.top_left,
                        bb.y - bw.top as f32 / 2.0,
                        bb.x + bb.width - cr.top_right,
                        bb.y - bw.top as f32 / 2.0,
                        bw.top as f32,
                        color
                    );
                    // Left edge
                    draw_line(
                        bb.x - bw.left as f32 / 2.0,
                        bb.y + cr.top_left,
                        bb.x - bw.left as f32 / 2.0,
                        bb.y + bb.height - cr.bottom_left,
                        bw.left as f32,
                        color
                    );
                    // Bottom edge
                    draw_line(
                        bb.x + cr.bottom_left,
                        bb.y + bb.height + bw.bottom as f32 / 2.0,
                        bb.x + bb.width - cr.bottom_right,
                        bb.y + bb.height + bw.bottom as f32 / 2.0,
                        bw.bottom as f32,
                        color
                    );
                    // Right edge
                    draw_line(
                        bb.x + bb.width + bw.right as f32 / 2.0,
                        bb.y + cr.top_right,
                        bb.x + bb.width + bw.right as f32 / 2.0,
                        bb.y + bb.height - cr.bottom_right,
                        bw.right as f32,
                        color
                    );

                    // Corners
                    // Top-left corner
                    if cr.top_left > 0.0 {
                        let width = bw.left.max(bw.top) as f32;
                        let points = ((std::f32::consts::PI * (cr.top_left + width)) / 2.0 / PIXELS_PER_POINT).max(5.0);
                        draw_arc(
                            bb.x + cr.top_left,
                            bb.y + cr.top_left,
                            points as u8,
                            cr.top_left,
                            180.0,
                            bw.left as f32,
                            90.0,
                            color
                        );
                    }
                    // Top-right corner
                    if cr.top_right > 0.0 {
                        let width = bw.top.max(bw.right) as f32;
                        let points = ((std::f32::consts::PI * (cr.top_right + width)) / 2.0 / PIXELS_PER_POINT).max(5.0);
                        draw_arc(
                            bb.x + bb.width - cr.top_right,
                            bb.y + cr.top_right,
                            points as u8,
                            cr.top_right,
                            270.0,
                            bw.top as f32,
                            90.0,
                            color
                        );
                    }
                    // Bottom-left corner
                    if cr.bottom_left > 0.0 {
                        let width = bw.left.max(bw.bottom) as f32;
                        let points = ((std::f32::consts::PI * (cr.bottom_left + width)) / 2.0 / PIXELS_PER_POINT).max(5.0);
                        draw_arc(
                            bb.x + cr.bottom_left,
                            bb.y + bb.height - cr.bottom_left,
                            points as u8,
                            cr.bottom_left,
                            90.0,
                            bw.bottom as f32,
                            90.0,
                            color
                        );
                    }
                    // Bottom-right corner
                    if cr.bottom_right > 0.0 {
                        let width = bw.bottom.max(bw.right) as f32;
                        let points = ((std::f32::consts::PI * (cr.bottom_right + width)) / 2.0 / PIXELS_PER_POINT).max(5.0);
                        draw_arc(
                            bb.x + bb.width - cr.bottom_right,
                            bb.y + bb.height - cr.bottom_right,
                            points as u8,
                            cr.bottom_right,
                            0.0,
                            bw.right as f32,
                            90.0,
                            color
                        );
                    }
                }
            }
            RenderCommandConfig::ScissorStart() => {
                let bb = command.bounding_box;
                state.clip = Some((
                    bb.x as i32,
                    bb.y as i32,
                    bb.width as i32,
                    bb.height as i32,
                ));
                unsafe {
                    get_internal_gl().quad_gl.scissor(state.clip);
                }
            }
            RenderCommandConfig::ScissorEnd() => {
                state.clip = None;
                unsafe {
                    get_internal_gl().quad_gl.scissor(None);
                }
            }
            RenderCommandConfig::Custom(_) => {
                handle_custom_command(&command);
            }
            RenderCommandConfig::None() => {}
        }
    }
    TEXTURE_MANAGER.lock().unwrap().clean();
}

pub fn create_measure_text_function(
    fonts: Vec<Font>,
) -> impl Fn(&str, &crate::TextConfig) -> crate::Dimensions + 'static {
    move |text: &str, config: &crate::TextConfig| {
        #[cfg(feature = "text-styling")]
        let cleaned_text = {
            // Remove macroquad_text_styling tags, handling escapes
            let mut result = String::new();
            let mut in_style_def = false;
            let mut escaped = false;
            for c in text.chars() {
                if escaped {
                    result.push(c);
                    escaped = false;
                    continue;
                }
                match c {
                    '\\' => {
                        escaped = true;
                    }
                    '{' => {
                        in_style_def = true;
                    }
                    '|' => {
                        if in_style_def {
                            in_style_def = false;
                        } else {
                            result.push(c);
                        }
                    }
                    '}' => {
                        // Nothing
                    }
                    _ => {
                        if !in_style_def {
                            result.push(c);
                        }
                    }
                }
            }
            if in_style_def {
                warn!("Ended inside a style definition while cleaning text for measurement! Make sure to escape curly braces with \\. Here is what we tried to measure: {}", text);
            }
            result
        };
        #[cfg(not(feature = "text-styling"))]
        let cleaned_text = text.to_string();
        let measured = macroquad::text::measure_text(
            &cleaned_text,
            Some(&fonts[config.font_id as usize]),
            config.font_size,
            1.0,
        );
        let added_space = (text.chars().count().max(1) - 1) as f32 * config.letter_spacing as f32;
        crate::Dimensions::new(measured.width + added_space, measured.height)
    }
}