use raylib::prelude::*;
use std::f32::consts::PI;

#[derive(Copy, Clone, Debug, Default)]
pub struct Fragment {
    pub world_position: Vector3,
    pub color: Vector3, // base color (0..1)
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Uniforms {
    pub time: f32,
    pub pattern: u32, // 0..N patterns selector
}

fn clamp01(x: f32) -> f32 { x.max(0.0).min(1.0) }

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vector3 {
    let h = h - h.floor();
    let i = (h * 6.0).floor() as i32;
    let f = h * 6.0 - (i as f32);
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i.rem_euclid(6) {
        0 => Vector3::new(v, t, p),
        1 => Vector3::new(q, v, p),
        2 => Vector3::new(p, v, t),
        3 => Vector3::new(p, q, v),
        4 => Vector3::new(t, p, v),
        _ => Vector3::new(v, p, q),
    }
}

fn pattern_rainbow_rings(pos: Vector3, time: f32, base: Vector3) -> Vector3 {
    let angle = pos.x.atan2(pos.z) + time * 0.6;
    let radius = (pos.x * pos.x + pos.y * pos.y + pos.z * pos.z).sqrt();
    let stripes = (radius * 12.0 + time * 2.0).sin().abs();
    let hue = ((angle / (2.0 * PI)) % 1.0 + 1.0) % 1.0;
    let rgb = hsv_to_rgb(hue, 1.0, 1.0);
    base * (1.0 - 0.7 * stripes) + rgb * (0.7 * stripes)
}

fn pattern_checker3d(pos: Vector3, time: f32, _base: Vector3) -> Vector3 {
    let scale = 4.0;
    let xi = (pos.x * scale + time * 0.2).floor() as i32;
    let yi = (pos.y * scale).floor() as i32;
    let zi = (pos.z * scale).floor() as i32;
    let parity = ((xi & 1) ^ (yi & 1) ^ (zi & 1)) as f32;
    // two-tone but tint it with a hue over time
    let hue = ((time * 0.1) % 1.0 + 1.0) % 1.0;
    let tint = hsv_to_rgb(hue, 0.8, 1.0);
    let a = Vector3::new(0.15, 0.15, 0.15);
    let b = Vector3::new(0.95, 0.95, 0.95);
    let base = if parity > 0.0 { a } else { b };
    base * 0.6 + tint * 0.4
}

fn pattern_grid_lines(pos: Vector3, time: f32, base: Vector3) -> Vector3 {
    let scale = 6.0;
    let fx = (pos.x * scale + time * 0.5).fract().abs();
    let fy = (pos.y * scale).fract().abs();
    let fz = (pos.z * scale).fract().abs();
    let line = (fx.min(1.0 - fx).min(fy.min(1.0 - fy)).min(fz.min(1.0 - fz)) < 0.04) as i32 as f32;
    let hue = ((pos.y * 0.2 + time * 0.2) % 1.0 + 1.0) % 1.0;
    let line_color = hsv_to_rgb(hue, 1.0, 1.0);
    base * (1.0 - line) + line_color * line
}

fn pattern_stripes(pos: Vector3, time: f32, base: Vector3) -> Vector3 {
    let v = ((pos.x * 8.0 + pos.y * 3.0 + time * 2.0).sin().abs()).powf(0.8);
    let hue = ((pos.z * 0.3 + time * 0.3) % 1.0 + 1.0) % 1.0;
    let color = hsv_to_rgb(hue, 0.9, v);
    base * 0.3 + color * 0.7
}

fn pattern_plasma(pos: Vector3, time: f32, base: Vector3) -> Vector3 {
    let s = (pos.x * 3.0 + time).sin() + (pos.y * 3.0 - time * 1.2).sin() + (pos.z * 3.0 + time * 0.6).sin();
    let v = 0.5 + 0.5 * (s * 0.5).sin();
    let hue = ((s * 0.1 + time * 0.1) % 1.0 + 1.0) % 1.0;
    let color = hsv_to_rgb(hue, 0.9, v);
    base * 0.2 + color * 0.8
}

// Red gradient pattern: emphasize red channel with slight time wave
fn pattern_red_gradient(pos: Vector3, time: f32, _base: Vector3) -> Vector3 {
    let d = (pos.x * pos.x + pos.y * pos.y + pos.z * pos.z).sqrt();
    let v = (1.0 / (1.0 + d)).max(0.0).min(1.0); // more red towards center
    let pulse = (time * 2.0 + d * 2.0).sin() * 0.1 + 0.9; // subtle pulse
    Vector3::new((v * pulse).max(0.0).min(1.0), 0.1 * v, 0.1 * v)
}

// Material colors only - no shader effects
fn pattern_material_only(_pos: Vector3, _time: f32, base: Vector3) -> Vector3 {
    base // just return the material color as-is
}

// returns RGB (0..1)
pub fn fragment_shader(fragment: &Fragment, uniforms: &Uniforms) -> Vector3 {
    let pos = fragment.world_position;
    let base = fragment.color;
    let time = uniforms.time;

    let col = match uniforms.pattern % 7 {
        0 => pattern_rainbow_rings(pos, time, base),
        1 => pattern_checker3d(pos, time, base),
        2 => pattern_grid_lines(pos, time, base),
        3 => pattern_stripes(pos, time, base),
        4 => pattern_plasma(pos, time, base),
        5 => pattern_red_gradient(pos, time, base),
        _ => pattern_material_only(pos, time, base),
    };

    Vector3::new(clamp01(col.x), clamp01(col.y), clamp01(col.z))
}
