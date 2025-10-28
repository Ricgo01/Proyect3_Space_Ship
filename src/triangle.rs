// triangle.rs

use crate::framebuffer::Framebuffer;
use crate::line::line;
use crate::shader::{fragment_shader, Fragment, Uniforms};
use raylib::prelude::*;

pub fn triangle(
    framebuffer: &mut Framebuffer,
    v1: Vector3,
    v2: Vector3,
    v3: Vector3,
) {
    let a = Vector2::new(v1.x, v1.y);
    let b = Vector2::new(v2.x, v2.y);
    let c = Vector2::new(v3.x, v3.y);

    line(framebuffer, a, b);
    line(framebuffer, b, c);
    line(framebuffer, c, a);
}

fn edge_function(a: Vector2, b: Vector2, c: Vector2) -> f32 {
    (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)
}

pub fn triangle_filled(
    framebuffer: &mut Framebuffer,
    v1: Vector3,
    v2: Vector3,
    v3: Vector3,
    base_color: Vector3, // 0..1
    uniforms: &Uniforms,
) {
    // Screen-space positions
    let p0 = Vector2::new(v1.x, v1.y);
    let p1 = Vector2::new(v2.x, v2.y);
    let p2 = Vector2::new(v3.x, v3.y);

    // Back-face culling by signed area (screen space)
    let signed_area = edge_function(p0, p1, p2);
    // Disabled culling temporarily to see all faces
    // if signed_area >= 0.0 { 
    //     return;
    // }

    // Bounding box
    let min_x = p0.x.min(p1.x).min(p2.x).floor().max(0.0) as i32;
    let max_x = p0.x.max(p1.x).max(p2.x).ceil().min(framebuffer.width as f32 - 1.0) as i32;
    let min_y = p0.y.min(p1.y).min(p2.y).floor().max(0.0) as i32;
    let max_y = p0.y.max(p1.y).max(p2.y).ceil().min(framebuffer.height as f32 - 1.0) as i32;

    let area = signed_area;
    if area.abs() < 1e-5 { return; }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = Vector2::new(x as f32 + 0.5, y as f32 + 0.5);
            let w0 = edge_function(p1, p2, p);
            let w1 = edge_function(p2, p0, p);
            let w2 = edge_function(p0, p1, p);
            if (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0) { // match culling orientation
                let w0n = w0 / area;
                let w1n = w1 / area;
                let w2n = w2 / area;

                // Interpolate world position for fun patterns
                let world = Vector3::new(
                    v1.x * w0n + v2.x * w1n + v3.x * w2n,
                    v1.y * w0n + v2.y * w1n + v3.y * w2n,
                    v1.z * w0n + v2.z * w1n + v3.z * w2n,
                );

                // Depth: use world.z as a simple depth (smaller is closer after transform)
                let depth = world.z;
                if framebuffer.test_and_set_depth(x as u32, y as u32, depth) {
                    let frag = Fragment { world_position: world, color: base_color };
                    let rgb = fragment_shader(&frag, uniforms);
                    let r = rgb.x.max(0.0).min(1.0);
                    let g = rgb.y.max(0.0).min(1.0);
                    let b = rgb.z.max(0.0).min(1.0);
                    let color = Color::new(
                        (r * 255.0) as u8,
                        (g * 255.0) as u8,
                        (b * 255.0) as u8,
                        255,
                    );
                    framebuffer.set_pixel_color(x as u32, y as u32, color);
                }
            }
        }
    }
}