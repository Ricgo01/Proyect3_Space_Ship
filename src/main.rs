// main.rs

mod framebuffer;
mod triangle;
mod line;
mod obj_loader;
mod shader;

use framebuffer::Framebuffer;
use triangle::triangle;
use obj_loader::Mesh;
use shader::{Uniforms};
use raylib::prelude::*;
use std::thread;
use std::time::Duration;
use std::f32::consts::PI;

fn transform(vertex: Vector3, translation: Vector3, scale: f32, rotation: Vector3) -> Vector3 {
    let (sin_x, cos_x) = (rotation.x * PI / 180.0).sin_cos();
    let (sin_y, cos_y) = (rotation.y * PI / 180.0).sin_cos();
    let (sin_z, cos_z) = (rotation.z * PI / 180.0).sin_cos();

    let mut new_vertex = vertex;

    // Rotate X
    let rotated_y = new_vertex.y * cos_x - new_vertex.z * sin_x;
    let rotated_z = new_vertex.y * sin_x + new_vertex.z * cos_x;
    new_vertex.y = rotated_y;
    new_vertex.z = rotated_z;

    // Rotate Y
    let rotated_x = new_vertex.x * cos_y + new_vertex.z * sin_y;
    let rotated_z = -new_vertex.x * sin_y + new_vertex.z * cos_y;
    new_vertex.x = rotated_x;
    new_vertex.z = rotated_z;

    // Rotate Z
    let rotated_x = new_vertex.x * cos_z - new_vertex.y * sin_z;
    let rotated_y = new_vertex.x * sin_z + new_vertex.y * cos_z;
    new_vertex.x = rotated_x;
    new_vertex.y = rotated_y;

    // Perspective projection (very simple) parameters
    // Move model away from camera
    let camera_distance = 3.0_f32; // units in model space
    let perspective = 1.0 / (1.0 + (new_vertex.z / camera_distance));

    // Scale in model space first
    new_vertex.x *= scale * perspective;
    new_vertex.y *= scale * perspective;

    // Translate to screen space
    new_vertex.x += translation.x;
    new_vertex.y += translation.y;

    new_vertex
}

fn render_cube(
    framebuffer: &mut Framebuffer,
    center: Vector3,
    translation: Vector3,
    scale: f32,
    rotation: Vector3,
) {
    let v1 = Vector3::new(center.x - 0.5, center.y - 0.5, center.z - 0.5); 
    let v2 = Vector3::new(center.x + 0.5, center.y - 0.5, center.z - 0.5);
    let v3 = Vector3::new(center.x + 0.5, center.y + 0.5, center.z - 0.5);
    let v4 = Vector3::new(center.x - 0.5, center.y + 0.5, center.z - 0.5);
    let v5 = Vector3::new(center.x - 0.5, center.y - 0.5, center.z + 0.5);
    let v6 = Vector3::new(center.x + 0.5, center.y - 0.5, center.z + 0.5);
    let v7 = Vector3::new(center.x + 0.5, center.y + 0.5, center.z + 0.5);
    let v8 = Vector3::new(center.x - 0.5, center.y + 0.5, center.z + 0.5);

    let t1 = transform(v1, translation, scale, rotation);
    let t2 = transform(v2, translation, scale, rotation);
    let t3 = transform(v3, translation, scale, rotation);
    let t4 = transform(v4, translation, scale, rotation);
    let t5 = transform(v5, translation, scale, rotation);
    let t6 = transform(v6, translation, scale, rotation);
    let t7 = transform(v7, translation, scale, rotation);
    let t8 = transform(v8, translation, scale, rotation);

    // Front face
    triangle(framebuffer, t1, t2, t4);
    triangle(framebuffer, t2, t3, t4);

    // Back face
    triangle(framebuffer, t5, t6, t8);
    triangle(framebuffer, t6, t7, t8);

    // Right face
    triangle(framebuffer, t2, t6, t3);
    triangle(framebuffer, t6, t7, t3);

    // Left face
    triangle(framebuffer, t1, t5, t4);
    triangle(framebuffer, t5, t8, t4);

    // Top face
    triangle(framebuffer, t3, t7, t4);
    triangle(framebuffer, t7, t8, t4);

    // Bottom face
    triangle(framebuffer, t1, t2, t5);
    triangle(framebuffer, t2, t6, t5);
}

fn main() {
    let window_width = 800;
    let window_height = 600;

    let (mut window, raylib_thread) = raylib::init()
        .size(window_width, window_height)
        .title("Window Example")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();

    let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);

    // Load Air_Wing model
    let models_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("models");
    let airwing_path = models_dir.join("Air_Wing.obj");
    
    println!("Loading OBJ from: {}", airwing_path.display());
    let mesh = match Mesh::load_obj(&airwing_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load OBJ: {e}");
            return;
        }
    };
    println!("Loaded mesh: {} vertices, {} triangles", mesh.vertices.len(), mesh.faces.len());

    framebuffer.set_background_color(Color::new(20, 20, 30, 255)); // Darker background to see gray material better

    let mut translation = Vector3::new(400.0, 300.0, 0.0); // screen center
    let mut rotation_user = Vector3::new(0.0, 0.0, 0.0);
    // Apply model orientation correction
    let model_rotation_offset = Vector3::new(180.0, 0.0, 0.0); // flip to correct orientation

    // Auto-fit scale based on mesh bounding box (already centered in loader)
    let mut max_len = 0.0f32;
    for v in &mesh.vertices {
        let len = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
        if len > max_len { max_len = len; }
    }
    // Fit so model roughly fills 70% of min(window_width, window_height)
    let target_pixels = (window_width.min(window_height) as f32) * 0.7;
    let mut scale = if max_len > 0.0 { target_pixels / max_len } else { 100.0 };
    // Allow user fine scaling with previous A/S keys

    let mut show_cube = false; // toggle between cube and mesh
    let mut filled = true;     // start with shader-filled by default
    let mut pattern: u32 = 6;  // default to pattern 6 (material colors only)
    while !window.window_should_close() {
        if window.is_key_pressed(KeyboardKey::KEY_C) {
            show_cube = !show_cube;
        }
        if window.is_key_pressed(KeyboardKey::KEY_F) {
            filled = !filled;
        }
    if window.is_key_pressed(KeyboardKey::KEY_ONE) { pattern = 0; }
    if window.is_key_pressed(KeyboardKey::KEY_TWO) { pattern = 1; }
    if window.is_key_pressed(KeyboardKey::KEY_THREE) { pattern = 2; }
    if window.is_key_pressed(KeyboardKey::KEY_FOUR) { pattern = 3; }
    if window.is_key_pressed(KeyboardKey::KEY_FIVE) { pattern = 4; }
    if window.is_key_pressed(KeyboardKey::KEY_SIX) { pattern = 5; }
    if window.is_key_pressed(KeyboardKey::KEY_SEVEN) { pattern = 6; }
        // New controls mapping:
        // W = zoom in, S = zoom out
        if window.is_key_down(KeyboardKey::KEY_W) { scale *= 1.02; }
        if window.is_key_down(KeyboardKey::KEY_S) { scale *= 0.98; }
        // Q = move right, E = move left
        if window.is_key_down(KeyboardKey::KEY_Q) { translation.x += 2.0; }
        if window.is_key_down(KeyboardKey::KEY_E) { translation.x -= 2.0; }
        // A / D = rotate around Y
    if window.is_key_down(KeyboardKey::KEY_A) { rotation_user.y -= 1.0; }
    if window.is_key_down(KeyboardKey::KEY_D) { rotation_user.y += 1.0; }

    framebuffer.clear();
    framebuffer.set_current_color(Color::WHITE);

    // uniforms (time in seconds and pattern)
    let uniforms = Uniforms { time: window.get_time() as f32, pattern };

        if show_cube {
            let vertex = Vector3::new(0.0, 0.0, 0.0);
            if filled {
                // render cube faces filled
                // Recompute transformed cube vertices
                let hs = 0.5;
                let raw = [
                    Vector3::new(-hs, -hs, -hs), Vector3::new(hs, -hs, -hs), Vector3::new(hs, hs, -hs), Vector3::new(-hs, hs, -hs),
                    Vector3::new(-hs, -hs, hs),  Vector3::new(hs, -hs, hs),  Vector3::new(hs, hs, hs),  Vector3::new(-hs, hs, hs),
                ];
                let tv: Vec<Vector3> = raw.iter().map(|&v| transform(v + vertex, translation, scale * 0.002, rotation_user)).collect();
                let faces = [
                    [0,1,3],[1,2,3], // front
                    [4,5,7],[5,6,7], // back
                    [1,5,2],[5,6,2], // right
                    [0,4,3],[4,7,3], // left
                    [2,6,3],[6,7,3], // top
                    [0,1,4],[1,5,4], // bottom
                ];
                let base = Vector3::new(1.0, 1.0, 1.0);
                for f in faces {
                    crate::triangle::triangle_filled(&mut framebuffer, tv[f[0]], tv[f[1]], tv[f[2]], base, &uniforms);
                }
            } else {
                render_cube(&mut framebuffer, vertex, translation, scale * 0.002, rotation_user);
            }
        } else {
            if filled {
                // Combine user rotation with model-specific offset for the mesh
                let rot_mesh = Vector3::new(
                    rotation_user.x + model_rotation_offset.x,
                    rotation_user.y + model_rotation_offset.y,
                    rotation_user.z + model_rotation_offset.z,
                );
                for (i, face) in mesh.faces.iter().enumerate() {
                    let v1 = transform(mesh.vertices[face[0]], translation, scale, rot_mesh);
                    let v2 = transform(mesh.vertices[face[1]], translation, scale, rot_mesh);
                    let v3 = transform(mesh.vertices[face[2]], translation, scale, rot_mesh);
                    
                    // Get material color for this face
                    let mat_idx = mesh.face_materials.get(i).copied().unwrap_or(0);
                    let material_color = mesh.materials.get(mat_idx)
                        .map(|m| m.diffuse_color)
                        .unwrap_or(Vector3::new(1.0, 1.0, 1.0));
                    
                    crate::triangle::triangle_filled(&mut framebuffer, v1, v2, v3, material_color, &uniforms);
                }
            } else {
                // wireframe
                let rot_mesh = Vector3::new(
                    rotation_user.x + model_rotation_offset.x,
                    rotation_user.y + model_rotation_offset.y,
                    rotation_user.z + model_rotation_offset.z,
                );
                for face in &mesh.faces {
                    let v1 = transform(mesh.vertices[face[0]], translation, scale, rot_mesh);
                    let v2 = transform(mesh.vertices[face[1]], translation, scale, rot_mesh);
                    let v3 = transform(mesh.vertices[face[2]], translation, scale, rot_mesh);
                    triangle(&mut framebuffer, v1, v2, v3);
                }
            }
        }

        framebuffer.swap_buffers(&mut window, &raylib_thread);

        thread::sleep(Duration::from_millis(16));
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    // h in [0,1), s,v in [0,1]
    let h6 = (h * 6.0).fract();
    let i = (h * 6.0).floor() as i32;
    let f = h6;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i.rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}