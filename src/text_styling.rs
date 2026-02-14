use std::collections::HashMap;
use std::f32::consts::PI;

#[derive(Debug, Clone)]
pub struct StyledSegment {
    pub text: String,
    pub styles: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct Color { 
    pub r: f32, 
    pub g: f32, 
    pub b: f32, 
    pub a: f32 
}

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub x: f32, 
    pub y: f32,
    pub scale_x: f32, 
    pub scale_y: f32,
    pub rotation: f32,
}

impl Default for Transform {
    fn default() -> Self { 
        Self { 
            x: 0.0, 
            y: 0.0, 
            scale_x: 1.0, 
            scale_y: 1.0, 
            rotation: 0.0 
        } 
    }
}

pub fn parse_text_lines(lines: Vec<String>) -> Result<Vec<Vec<StyledSegment>>, String> {
    let mut result_lines: Vec<Vec<StyledSegment>> = Vec::new();
    let mut style_stack: Vec<String> = Vec::new();
    
    let mut in_style_def = false;
    let mut escaped = false;
    
    let mut text_buffer = String::new();
    let mut style_buffer = String::new();

    for line in lines {
        let mut line_segments: Vec<StyledSegment> = Vec::new();
        
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
                            line_segments.push(StyledSegment {
                                text: text_buffer.clone(),
                                styles: style_stack.clone(),
                            });
                            text_buffer.clear();
                        }
                        in_style_def = true;
                    }
                }
                '|' => {
                    if in_style_def {
                        style_stack.push(style_buffer.clone());
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
                            line_segments.push(StyledSegment {
                                text: text_buffer.clone(),
                                styles: style_stack.clone(),
                            });
                            text_buffer.clear();
                        }
                        
                        if style_stack.pop().is_none() {
                            return Err(format!("Error: '}}' found with no open style on this line: {}", line));
                        }
                    }
                }
                ' ' => {
                    if in_style_def {
                        return Err(format!("Error: Whitespace not allowed in style definition on this line: {}", line));
                    } else {
                        text_buffer.push(c);
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

        if !text_buffer.is_empty() {
            line_segments.push(StyledSegment {
                text: text_buffer.clone(),
                styles: style_stack.clone(),
            });
            text_buffer.clear();
        }
        
        result_lines.push(line_segments);
    }

    if in_style_def {
        return Err("Error: Ended inside a style definition.".to_string());
    }
    if !style_stack.is_empty() {
        return Err(format!("Error: Ended with {} unclosed styles.", style_stack.len()));
    }

    Ok(result_lines)
}

pub fn render_styled_text<F1, F2>(
    segments: &[StyledSegment], 
    time: f64, 
    font_size: f32,
    animation_tracker: &mut HashMap<String, (usize, f64)>,
    total_char_index: &mut usize,
    mut render_fn: F1,
    mut render_shadow_fn: F2
) where 
    F1: FnMut(&str, Transform, Color),
    F2: FnMut(&str, Transform, Color)
{
    let named_colors: HashMap<String, Color> = [
        ("white", (1.0, 1.0, 1.0)), ("black", (0.0, 0.0, 0.0)),
        ("lightgray", (0.75, 0.75, 0.75)), ("darkgray", (0.37, 0.37, 0.37)),
        ("red", (0.9, 0.0, 0.0)), ("orange", (1.0, 0.55, 0.0)),
        ("yellow", (1.0, 0.84, 0.0)), ("lime", (0.0, 0.8, 0.0)),
        ("green", (0.0, 0.5, 0.0)), ("cyan", (0.0, 0.8, 0.8)),
        ("lightblue", (0.2, 0.6, 1.0)), ("blue", (0.0, 0.2, 0.8)),
        ("purple", (0.45, 0.15, 0.77)), ("magenta", (0.8, 0.0, 0.8)),
        ("brown", (0.54, 0.27, 0.07)), ("pink", (1.0, 0.4, 0.66)),
    ].iter().map(|(k, (r,g,b))| (k.to_string(), Color{r:*r, g:*g, b:*b, a:1.0})).collect();

    let parse_float = |s: &str| s.parse::<f32>().unwrap_or(0.0);
    let parse_color = |s: &str| -> Color {
        if let Some(c) = named_colors.get(&s.to_lowercase()) { return *c; }
        if s.starts_with('#') {
            let hex = s.trim_start_matches('#');
            if let Ok(val) = u32::from_str_radix(hex, 16) {
                let r = ((val >> 16) & 0xFF) as f32 / 255.0;
                let g = ((val >> 8) & 0xFF) as f32 / 255.0;
                let b = (val & 0xFF) as f32 / 255.0;
                return Color { r, g, b, a: 1.0 };
            }
        }
        if s.starts_with('(') && s.ends_with(')') {
            let inner = &s[1..s.len()-1];
            let parts: Vec<f32> = inner.split(',').map(|p| parse_float(p.trim())).collect();
            if parts.len() >= 3 {
                return Color { r: parts[0] / 255.0, g: parts[1] / 255.0, b: parts[2] / 255.0, a: 1.0 };
            }
        }
        Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }
    };

    for segment in segments {
        let mut has_effects = false;
        for style_str in &segment.styles {
            let mut parts = style_str.split('_');
            let first_part = parts.next().unwrap_or("");
            let (cmd, _) = if let Some(idx) = first_part.find('=') {
                (&first_part[..idx], Some(&first_part[idx+1..]))
            } else {
                (first_part, None)
            };
            if cmd != "color" && cmd != "opacity" && !cmd.is_empty() {
                has_effects = true;
                break;
            }
        }

        if !has_effects {
            let mut color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
            let mut opacity_mult = 1.0;
            
            for style_str in &segment.styles {
                let mut parts = style_str.split('_');
                let first_part = parts.next().unwrap_or("");
                let (cmd, first_arg_val) = if let Some(idx) = first_part.find('=') {
                    (&first_part[..idx], Some(&first_part[idx+1..]))
                } else {
                    (first_part, None)
                };
                
                let val = if let Some(v) = first_arg_val {
                    v
                } else {
                    parts.next().unwrap_or("")
                };

                if cmd == "color" {
                    color = parse_color(val);
                } else if cmd == "opacity" {
                    opacity_mult *= parse_float(val);
                }
            }
            
            color.a *= opacity_mult;
            render_fn(&segment.text, Transform::default(), color);
            *total_char_index += segment.text.chars().count();
            continue;
        }

        for char_obj in segment.text.chars() {
            let global_char_idx = *total_char_index as f32;

            let mut tr = Transform::default();
            let mut color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
            let mut opacity_mult = 1.0;
            let mut shadow_opts: Option<(Color, f32, f32, f32, f32)> = None;
            let mut skip_render = false;
            let mut render_char = char_obj.to_string();
            
            for style_str in &segment.styles {
                let mut parts = style_str.split('_');
                let first_part = parts.next().unwrap_or("");
                
                let (cmd, first_arg_val) = if let Some(idx) = first_part.find('=') {
                    (&first_part[..idx], Some(&first_part[idx+1..]))
                } else {
                    (first_part, None)
                };

                let mut args: HashMap<&str, &str> = parts.map(|arg| {
                    let mut kv = arg.split('=');
                    (kv.next().unwrap_or(""), kv.next().unwrap_or(""))
                }).collect();
                
                if let Some(val) = first_arg_val {
                    args.insert("", val);
                }

                let get_f = |k: &str, def: f32| args.get(k).map(|v| parse_float(v)).unwrap_or(def);

                if cmd == "hide" { skip_render = true; break; }

                let anim_id = args.get("id").unwrap_or(&"");
                if !anim_id.is_empty() {
                    let anim_key = anim_id.to_string();
                    let (start_index, start_time) = {
                        let entry = animation_tracker.entry(anim_key.clone()).or_insert((*total_char_index, time));
                        (entry.0, entry.1)
                    };
                    if *total_char_index < start_index {
                        animation_tracker.insert(anim_key, (*total_char_index, start_time));
                    }
                    let delay = get_f("delay", 0.0);
                    
                    let elapsed = ((time - start_time) as f32 - delay).max(0.0);
                    let relative_idx = global_char_idx - start_index as f32;
                    
                    let is_in = args.contains_key("in");
                    let is_out = args.contains_key("out");
                    
                    if is_in || is_out {
                        match cmd {
                            "type" => {
                                let speed = get_f("speed", 8.0);
                                let chars_processed = elapsed * speed;
                                let cursor = args.get("cursor").unwrap_or(&"");
                                if is_in {
                                    if relative_idx >= chars_processed {
                                        if !cursor.is_empty() && relative_idx > 0.0 && (relative_idx - 1.0) < chars_processed {
                                            render_char = cursor.to_string();
                                        } else {
                                            skip_render = true;
                                        }
                                    }
                                } else {
                                    if relative_idx < chars_processed { skip_render = true; }
                                }
                            },
                            "fade" => {
                                let speed = get_f("speed", 3.0);
                                let trail = get_f("trail", 3.0);
                                let progress = (elapsed * speed - relative_idx) / trail;
                                let mut alpha = progress.clamp(0.0, 1.0);
                                if is_out { alpha = 1.0 - alpha; }
                                opacity_mult *= alpha;
                            },
                            "scale" => {
                                let speed = get_f("speed", 3.0);
                                let trail = get_f("trail", 3.0);
                                let progress = (elapsed * speed - relative_idx) / trail;
                                let mut s = progress.clamp(0.0, 1.0);
                                if is_out { s = 1.0 - s; }
                                tr.scale_x *= s;
                                tr.scale_y *= s;
                            }
                            _ => {}
                        }
                    } else {
                        panic!("Animation style '{}' requires either 'in' or 'out' argument.", cmd);
                    }
                }

                if skip_render { break; }

                if cmd == "transform" {
                    if let Some(v) = args.get("translate") {
                        let nums: Vec<f32> = v.split(',').map(parse_float).collect();
                        tr.x += nums.get(0).unwrap_or(&0.0) * font_size; 
                        tr.y += nums.get(1).unwrap_or(&0.0) * font_size;
                    }
                    if let Some(v) = args.get("scale") {
                        let nums: Vec<f32> = v.split(',').map(parse_float).collect();
                        tr.scale_x *= nums.get(0).unwrap_or(&1.0);
                        tr.scale_y *= nums.get(1).unwrap_or(nums.get(0).unwrap_or(&1.0));
                    }
                    tr.rotation += get_f("rotate", 0.0);
                }

                if cmd == "wave" {
                    let w = get_f("w", 3.0);
                    let f = if args.contains_key("s") { get_f("s", 0.0) / w } else { get_f("f", 0.5) };
                    let a = get_f("a", 0.3) * font_size;
                    let p = get_f("p", 0.0);
                    let r = get_f("r", 0.0);
                    
                    let arg = 2.0 * PI * (f * time as f32 + global_char_idx / w + p);
                    let disp = arg.cos() * a;
                    
                    let rad = r.to_radians();
                    tr.x += -disp * rad.sin();
                    tr.y += disp * rad.cos();
                }

                if cmd == "pulse" {
                    let w = get_f("w", 2.0);
                    let f = if args.contains_key("s") { get_f("s", 0.0) / w } else { get_f("f", 0.6) };
                    let a = get_f("a", 0.15);
                    let p = get_f("p", 0.0);
                    
                    let arg = 2.0 * PI * (f * time as f32 + global_char_idx / w + p);
                    let scale_delta = 1.0 + arg.cos() * a;
                    tr.scale_x *= scale_delta;
                    tr.scale_y *= scale_delta;
                }

                if cmd == "swing" {
                    let w = get_f("w", 3.0);
                    let f = if args.contains_key("s") { get_f("s", 0.0) / w } else { get_f("f", 0.5) };
                    let a = get_f("a", 8.0);
                    let p = get_f("p", 0.0);
                    
                    let arg = 2.0 * PI * (f * time as f32 + global_char_idx / w + p);
                    tr.rotation += arg.sin() * a;
                }

                if cmd == "jitter" {
                    let seed = (time as f32 * 20.0).floor() + global_char_idx * 13.37;
                    let rand_x = (seed.sin() * 43758.5453).fract();
                    let rand_y = ((seed + 7.1).cos() * 23421.632).fract();
                    
                    let radii_str = args.get("radii").unwrap_or(&"0.1,0.1");
                    let rads: Vec<f32> = radii_str.split(',').map(parse_float).collect();
                    let rx = rads.get(0).unwrap_or(&0.5) * font_size;
                    let ry = rads.get(1).unwrap_or(rads.get(0).unwrap_or(&0.5)) * font_size;
                    let rot = get_f("rotation", 0.0).to_radians();

                    let jx = (rand_x - 0.5) * 2.0 * rx;
                    let jy = (rand_y - 0.5) * 2.0 * ry;

                    tr.x += jx * rot.cos() - jy * rot.sin();
                    tr.y += jx * rot.sin() + jy * rot.cos();
                }

                if cmd == "gradient" {
                    let speed = get_f("speed", 1.0);
                    let stops_str = args.get("stops").unwrap_or(&"0:#FF0000,1:#FF9A00,2:#D0DE21,3:#4FDC4A,4:#3FDAD8,5:#2FC9E2,6:#1C7FEE,7:#5F15F2,8:#BA0CF8,9:#FB07D9,10:#FF0000");
                    
                    let stops: Vec<(f32, Color)> = stops_str.split(',').map(|pair| {
                        let mut kv = pair.split(':');
                        let pos = kv.next().unwrap_or("0").parse::<f32>().unwrap_or(0.0);
                        let col = parse_color(kv.next().unwrap_or("white"));
                        (pos, col)
                    }).collect();
                    
                    if !stops.is_empty() {
                        let cycle_len = stops.last().unwrap().0;
                        let current_pos = (global_char_idx - time as f32 * speed).rem_euclid(cycle_len);
                        
                        let mut c1 = stops[0].1;
                        let mut c2 = stops[0].1;
                        let mut t = 0.0;

                        for i in 0..stops.len()-1 {
                            if current_pos >= stops[i].0 && current_pos <= stops[i+1].0 {
                                c1 = stops[i].1;
                                c2 = stops[i+1].1;
                                let span = stops[i+1].0 - stops[i].0;
                                t = if span > 0.0 { (current_pos - stops[i].0) / span } else { 0.0 };
                                break;
                            }
                        }

                        if current_pos > stops.last().unwrap().0 {
                            c1 = stops.last().unwrap().1;
                            c2 = stops[0].1;
                            let span = cycle_len - stops.last().unwrap().0;
                            t = (current_pos - stops.last().unwrap().0) / span;
                        }
                        
                        color.r = c1.r + (c2.r - c1.r) * t;
                        color.g = c1.g + (c2.g - c1.g) * t;
                        color.b = c1.b + (c2.b - c1.b) * t;
                    }
                }

                if cmd == "opacity" {
                    if let Some(v) = args.get("") {
                        opacity_mult *= parse_float(v);
                    }
                }

                if cmd == "color" {
                    if let Some(v) = args.get("") {
                        color = parse_color(v);
                    }
                }

                if cmd == "shadow" {
                    let color_str = args.get("color").unwrap_or(&"black");
                    let sc = parse_color(color_str);
                    let off_str = args.get("offset").unwrap_or(&"-0.3,0.3");
                    let offs: Vec<f32> = off_str.split(',').map(parse_float).collect();
                    let ox = offs.get(0).unwrap_or(&-0.3) * font_size;
                    let oy = offs.get(1).unwrap_or(&0.3) * font_size;
                    
                    let scl_str = args.get("scale").unwrap_or(&"1");
                    let scls: Vec<f32> = scl_str.split(',').map(parse_float).collect();
                    let sx = *scls.get(0).unwrap_or(&1.0);
                    let sy = *scls.get(1).unwrap_or(&sx);
                    
                    shadow_opts = Some((sc, ox, oy, sx, sy));
                }
            }

            if !skip_render {
                color.a *= opacity_mult;
                
                if let Some((sc, ox, oy, ssx, ssy)) = shadow_opts {
                    let mut shadow_tr = tr;
                    shadow_tr.x += ox;
                    shadow_tr.y += oy;
                    shadow_tr.scale_x *= ssx;
                    shadow_tr.scale_y *= ssy;
                    
                    let shadow_final_color = Color { r: sc.r, g: sc.g, b: sc.b, a: sc.a * opacity_mult };
                    render_shadow_fn(&render_char, shadow_tr, shadow_final_color);
                }
                
                render_fn(&render_char, tr, color);
            }
            *total_char_index += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_text() {
        let lines = vec!["Hello".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 1, "Rendered length should be 1 for 'Hello' (optimized)");
        assert_eq!(rendered[0].0, "Hello", "First text should be 'Hello'");
        assert_eq!(rendered[0].1.scale_x, 1.0, "Default scale_x should be 1.0");
        assert_eq!(rendered[0].1.scale_y, 1.0, "Default scale_y should be 1.0");
    }

    #[test]
    fn test_render_color_named() {
        let lines = vec!["{color=red|R}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered[0].0, "R", "Char should be 'R'");
        assert!((rendered[0].2.r - 0.9).abs() < 0.01, "Named color red r value wrong? {:?}", rendered);
        assert!(rendered[0].2.g < 0.01, "Named color red g value wrong? {:?}", rendered);
        assert!(rendered[0].2.b < 0.01, "Named color red b value wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_color_hex() {
        let lines = vec!["{color=#FF0000|R}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!((rendered[0].2.r - 1.0).abs() < 0.01, "Hex color r value wrong? {:?}", rendered);
        assert!(rendered[0].2.g < 0.01, "Hex color g value wrong? {:?}", rendered);
        assert!(rendered[0].2.b < 0.01, "Hex color b value wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_color_rgb() {
        let lines = vec!["{color=(255,128,0)|O}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!((rendered[0].2.r - 1.0).abs() < 0.01, "RGB color r value wrong? {:?}", rendered);
        assert!((rendered[0].2.g - 128.0/255.0).abs() < 0.01, "RGB color g value wrong? {:?}", rendered);
        assert!(rendered[0].2.b < 0.01, "RGB color b value wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_opacity() {
        let lines = vec!["{opacity=0.5|A}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!((rendered[0].2.a - 0.5).abs() < 0.01, "Opacity value wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_transform_translate() {
        let lines = vec!["{transform_translate=0.5,0.5|A}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!((rendered[0].1.x - 8.0).abs() < 0.01, "Translate x wrong? {:?}", rendered); // 0.5 * 16.0
        assert!((rendered[0].1.y - 8.0).abs() < 0.01, "Translate y wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_transform_scale() {
        let lines = vec!["{transform_scale=2.0|A}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!((rendered[0].1.scale_x - 2.0).abs() < 0.01, "Scale x wrong? {:?}", rendered);
        assert!((rendered[0].1.scale_y - 2.0).abs() < 0.01, "Scale y wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_transform_scale_xy() {
        let lines = vec!["{transform_scale=2.0,0.5|A}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!((rendered[0].1.scale_x - 2.0).abs() < 0.01, "Scale x wrong? {:?}", rendered);
        assert!((rendered[0].1.scale_y - 0.5).abs() < 0.01, "Scale y wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_transform_rotate() {
        let lines = vec!["{transform_rotate=45|A}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!((rendered[0].1.rotation - 45.0).abs() < 0.01, "Rotate value wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_wave_effect() {
        let lines = vec!["{wave|ABC}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 3, "Wave effect rendered length wrong? {:?}", rendered);
        assert_ne!(rendered[0].1.y, rendered[1].1.y, "Wave effect Y position not different? {:?}", rendered);
    }

    #[test]
    fn test_render_wave_with_params() {
        let lines = vec!["{wave_w=2.0_f=1.0_a=0.5|AB}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 2, "Wave effect rendered length wrong? {:?}", rendered);
        assert!(rendered[0].1.y.abs() <= 8.0, "Wave effect Y position amplitude wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_pulse_effect() {
        let lines = vec!["{pulse|ABC}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 3, "Pulse effect rendered length wrong? {:?}", rendered);
        assert_ne!(rendered[0].1.scale_x, rendered[1].1.scale_x, "Pulse effect scale not different? {:?}", rendered);
    }

    #[test]
    fn test_render_swing_effect() {
        let lines = vec!["{swing|ABC}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 3, "Swing effect rendered length wrong? {:?}", rendered);
        assert_ne!(rendered[0].1.rotation, rendered[1].1.rotation, "Swing effect rotation not different? {:?}", rendered);
    }

    #[test]
    fn test_render_jitter_effect() {
        let lines = vec!["{jitter_radii=0.1,0.1|A}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered_t1 = Vec::new();
        let mut rendered_t2 = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered_t1.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        render_styled_text(&segments[0], 0.5, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered_t2.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_ne!(rendered_t1[0].1.x, rendered_t2[0].1.x, "Jitter effect X position not different? {:?} {:?}", rendered_t1, rendered_t2);
    }

    #[test]
    fn test_render_gradient_effect() {
        let lines = vec!["{gradient_stops=0:#FF0000,3:#0000FF|ABC}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 3, "Gradient effect rendered length wrong? {:?}", rendered);
        assert!(rendered[0].2.r > 0.5, "Gradient effect first char color not correct? {:?}", rendered);
        assert!(rendered[2].2.b > rendered[0].2.b, "Gradient effect color not correct? {:?}", rendered);
    }

    #[test]
    fn test_render_hide_effect() {
        let lines = vec!["{hide|ABC}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 0, "Hide effect rendered length wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_shadow_effect() {
        let lines = vec!["{shadow|A}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        let mut shadows = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |c, tr, col| shadows.push((c.to_string(), tr, col)));
        
        assert_eq!(rendered.len(), 1, "Shadow effect rendered length wrong? {:?}", rendered);
        assert_eq!(shadows.len(), 1, "Shadow effect shadows length wrong? {:?}", shadows);
        assert_eq!(shadows[0].0, "A", "Shadow effect char wrong? {:?}", shadows);
        assert!(shadows[0].2.r < 0.1, "Shadow effect color r value wrong? {:?}", shadows);
    }

    #[test]
    fn test_render_shadow_with_color() {
        let lines = vec!["{shadow_color=red|A}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut shadows = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |_, _, _| {},
            |c, tr, col| shadows.push((c.to_string(), tr, col)));
        
        assert!(shadows[0].2.r > 0.5, "Shadow color r value wrong? {:?}", shadows);
    }

    #[test]
    fn test_render_shadow_offset() {
        let lines = vec!["{shadow_offset=0.5,0.5|A}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        let mut shadows = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |c, tr, col| shadows.push((c.to_string(), tr, col)));
        
        assert!((shadows[0].1.x - 8.0).abs() < 0.01, "Shadow offset x wrong? {:?}", shadows);
        assert!((shadows[0].1.y - 8.0).abs() < 0.01, "Shadow offset y wrong? {:?}", shadows);
    }

    #[test]
    fn test_render_type_animation() {
        let lines = vec!["{type_in_id=t1_cursor=\\||ABC}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 0, "Type animation at time 0 should show nothing");
        
        rendered.clear();
        render_styled_text(&segments[0], 0.1, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!(rendered.len() > 0, "Type animation after time should show chars");

        assert!(rendered[rendered.len()-1].0 == "|", "Type animation cursor should be present? {:?}", rendered);
    }

    #[test]
    fn test_render_fade_animation() {
        let lines = vec!["{fade_in_id=f1|A}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!(rendered.len() > 0, "Fade animation should render something");
        assert!(rendered[0].2.a < 0.1, "Fade animation alpha at time 0 should be low? {:?}", rendered);
        
        rendered.clear();
        render_styled_text(&segments[0], 2.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!(rendered[0].2.a > 0.9, "Fade animation alpha after time should be high? {:?}", rendered);
    }

    #[test]
    fn test_render_scale_animation() {
        let lines = vec!["{scale_in_id=s1|A}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!(rendered[0].1.scale_x < 0.1, "Scale animation scale_x at time 0 should be small? {:?}", rendered);
        
        rendered.clear();
        render_styled_text(&segments[0], 2.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!(rendered[0].1.scale_x > 0.9, "Scale animation scale_x after time should be large? {:?}", rendered);
    }

    #[test]
    fn test_render_nested_wave_pulse() {
        let lines = vec!["{wave|{pulse|ABC}}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 3, "Nested wave/pulse rendered length wrong? {:?}", rendered);
        assert_ne!(rendered[0].1.y, 0.0, "Wave effect y not applied? {:?}", rendered);
        assert_ne!(rendered[0].1.scale_x, 1.0, "Pulse effect scale_x not applied? {:?}", rendered);
    }

    #[test]
    fn test_render_nested_color_wave() {
        let lines = vec!["{color=red|{wave|ABC}}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 3, "Nested color/wave rendered length wrong? {:?}", rendered);
        assert!(rendered[0].2.r > 0.5, "Nested color effect r value wrong? {:?}", rendered);
        assert!(rendered[1].2.r > 0.5, "Nested color effect r value wrong? {:?}", rendered);
        assert_ne!(rendered[0].1.y, rendered[1].1.y, "Nested wave effect y not different? {:?}", rendered);
    }

    #[test]
    fn test_render_multiple_same_effect_nested() {
        let lines = vec!["{wave|A{wave|B}C}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.5, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 3, "Multiple nested wave rendered length wrong? {:?}", rendered);
        let b_offset = rendered[1].1.y;
        let a_offset = rendered[0].1.y;
        assert_ne!(b_offset, a_offset, "Nested wave offsets not different? {:?}", rendered);
    }

    #[test]
    fn test_render_gradient_over_time() {
        let lines = vec!["{gradient_speed=10|AB}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered_t1 = Vec::new();
        let mut rendered_t2 = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered_t1.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        render_styled_text(&segments[0], 0.1, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered_t2.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_ne!(rendered_t1[0].2.r, rendered_t2[0].2.r, "Gradient color r value should change over time");
    }

    #[test]
    fn test_render_all_effects_combined() {
        let lines = vec!["{wave|{pulse|{swing|{color=cyan|ABC}}}}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.5, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 3, "All effects combined rendered length wrong? {:?}", rendered);
        assert_ne!(rendered[0].1.y, 0.0, "Wave effect y not applied? {:?}", rendered);
        assert_ne!(rendered[0].1.scale_x, 1.0, "Pulse effect scale_x not applied? {:?}", rendered);
        assert_ne!(rendered[0].1.rotation, 0.0, "Swing effect rotation not applied? {:?}", rendered);
        assert!(rendered[0].2.g > 0.5 && rendered[0].2.b > 0.5, "Cyan color effect not applied? {:?}", rendered);
    }

    #[test]
    fn test_render_color_overwrite_nested() {
        let lines = vec!["{color=red|A{color=blue|B}C}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 3, "Color overwrite nested rendered length wrong? {:?}", rendered);
        assert!(rendered[0].2.r > 0.5 && rendered[0].2.b < 0.5, "Outer color red not applied to A? {:?}", rendered);
        assert!(rendered[1].2.b > 0.5 && rendered[1].2.r < 0.5, "Inner color blue not applied to B? {:?}", rendered);
        assert!(rendered[2].2.r > 0.5 && rendered[2].2.b < 0.5, "Outer color red not applied to C? {:?}", rendered);
    }

    #[test]
    fn test_render_transform_accumulation() {
        let lines = vec!["{transform_translate=0.5,0|{transform_translate=0,0.5|A}}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!((rendered[0].1.x - 8.0).abs() < 0.01, "Transform accumulation x wrong? {:?}", rendered);
        assert!((rendered[0].1.y - 8.0).abs() < 0.01, "Transform accumulation y wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_opacity_accumulation() {
        let lines = vec!["{opacity=0.5|{opacity=0.5|A}}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!((rendered[0].2.a - 0.25).abs() < 0.01, "Opacity accumulation wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_shadow_with_transform() {
        let lines = vec!["{transform_scale=2|{shadow|A}}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        let mut shadows = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |c, tr, col| shadows.push((c.to_string(), tr, col)));
        
        assert!((rendered[0].1.scale_x - 2.0).abs() < 0.01, "Shadow with transform scale_x wrong? {:?}", rendered);
        assert!((shadows[0].1.scale_x - 2.0).abs() < 0.01, "Shadow with transform scale_x wrong? {:?}", shadows);
    }

    #[test]
    fn test_render_empty_text() {
        let lines = vec!["".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 0, "Empty text rendered length wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_unicode_with_effects() {
        let lines = vec!["{wave|你好🌍}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 3, "Unicode with effects rendered length wrong? {:?}", rendered);
        assert_eq!(rendered[0].0, "你", "First unicode char wrong? {:?}", rendered);
        assert_eq!(rendered[1].0, "好", "Second unicode char wrong? {:?}", rendered);
        assert_eq!(rendered[2].0, "🌍", "Third unicode char wrong? {:?}", rendered);
    }

    #[test]
    fn test_render_type_out_animation() {
        let lines = vec!["{type_out_id=t2|ABC}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 3, "Type out animation at time 0 should show all chars");
        
        rendered.clear();
        render_styled_text(&segments[0], 0.5, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert_eq!(rendered.len(), 0, "Type out animation after time should hide chars");
    }

    #[test]
    fn test_render_fade_out_animation() {
        let lines = vec!["{fade_out_id=f2|A}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!(rendered.len() > 0);
        assert!(rendered[0].2.a > 0.9, "Fade out animation alpha at time 0 should be high? {:?}", rendered);
        
        rendered.clear();
        render_styled_text(&segments[0], 2.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!(rendered[0].2.a < 0.1, "Fade out animation alpha after time should be low? {:?}", rendered);
    }

    #[test]
    fn test_render_scale_out_animation() {
        let lines = vec!["{scale_out_id=s2|A}".to_string()];
        let segments = parse_text_lines(lines).unwrap();
        let mut tracker = HashMap::new();
        let mut rendered = Vec::new();
        
        render_styled_text(&segments[0], 0.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!(rendered[0].1.scale_x > 0.9, "Scale out animation scale_x at time 0 should be large? {:?}", rendered);
        
        rendered.clear();
        render_styled_text(&segments[0], 2.0, 16.0, &mut tracker, &mut 0,
            |c, tr, col| rendered.push((c.to_string(), tr, col)),
            |_, _, _| {});
        
        assert!(rendered[0].1.scale_x < 0.1, "Scale out animation scale_x after time should be small? {:?}", rendered);
    }
}
