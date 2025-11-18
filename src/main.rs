use minifb::{Key, KeyRepeat, Window, WindowOptions};
use nalgebra_glm::{Mat4, Vec3, Vec4};
use raylib::core::audio::{Music, RaylibAudio};
use std::f32::consts::PI;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

mod celestial_shaders;
mod color;
mod fragment;
mod framebuffer;
mod line;
mod obj;
mod shaders;
mod triangle;
mod vertex;

use celestial_shaders::{get_celestial_shader, CelestialBody};
use color::Color;
use framebuffer::Framebuffer;
use line::line;
use obj::Obj;
use shaders::vertex_shader;
use triangle::triangle;
use vertex::Vertex;

const AIRWING_DISTANCE: f32 = 340.0;
const AIRWING_VERTICAL_OFFSET: f32 = 0.0;
const AIRWING_SIDE_OFFSET: f32 = 0.0;
const AIRWING_SCALE: f32 = 30.0;
const AIRWING_COLLISION_RADIUS: f32 = 60.0;
const AIRWING_EXTRA_VERTICAL_RANGE: f32 = 40.0;
const AIRWING_FORWARD_RANGE: f32 = 80.0;
const CAMERA_SHIP_SAFE_GAP: f32 = 110.0;
const CAMERA_SHIP_MIN_OFFSET: f32 = 140.0;
const CAMERA_SHIP_MAX_OFFSET: f32 = 420.0;
const MIN_CAMERA_TARGET_DISTANCE: f32 = 1.0;
const STAR_COUNT: usize = 950;
const STARFIELD_RADIUS: f32 = 5200.0;
const STAR_DEPTH: f32 = 0.999_99;

#[derive(Clone, Copy)]
struct Star {
    position: Vec3,
    color: Color,
}

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        // Numerical Recipes LCG constants
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.state >> 16) as u32
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }
}

pub struct Uniforms {
    model_matrix: Mat4,
    view_matrix: Mat4,
    projection_matrix: Mat4,
    time: f32,
    current_shader: CelestialBody,
    light_position: Vec3,
    camera_position: Vec3,
    detail_level: f32,
    ring_palette: Option<[Color; 3]>,
}
fn locate_asset(base_name: &str) -> Option<PathBuf> {
    let entries = fs::read_dir("assets").ok()?;
    for entry in entries {
        let entry = entry.ok()?;
        if !entry.file_type().ok()?.is_file() {
            continue;
        }
        let path = entry.path();
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if stem.eq_ignore_ascii_case(base_name) {
                return Some(path);
            }
        }
    }
    None
}

#[derive(Clone)]
struct Camera {
    position: Vec3,
    target: Vec3,
    up: Vec3,
    zoom: f32,
}

impl Camera {
    fn new(position: Vec3, target: Vec3) -> Self {
        Camera {
            position,
            target,
            up: Vec3::new(0.0, 1.0, 0.0),
            zoom: 1.0,
        }
    }

    fn get_view_matrix(&self) -> Mat4 {
        nalgebra_glm::look_at(&self.position, &self.target, &self.up)
    }

    fn forward_direction(&self) -> Option<Vec3> {
        let dir = self.target - self.position;
        if dir.magnitude_squared() < 1e-6 {
            None
        } else {
            Some(dir.normalize())
        }
    }

    fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        let radius = (self.position - self.target).magnitude();

        // Calcular ángulos actuales
        let dx = self.position.x - self.target.x;
        let dy = self.position.y - self.target.y;
        let dz = self.position.z - self.target.z;

        let mut theta = dz.atan2(dx); // ángulo horizontal
        let mut phi = (dy / radius).asin(); // ángulo vertical

        // Aplicar deltas
        theta += delta_x;
        phi += delta_y;

        // Limitar phi para evitar gimbal lock
        phi = phi.clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);

        // Calcular nueva posición
        self.position.x = self.target.x + radius * phi.cos() * theta.cos();
        self.position.y = self.target.y + radius * phi.sin();
        self.position.z = self.target.z + radius * phi.cos() * theta.sin();
    }

    fn move_forward(&mut self, amount: f32) {
        if let Some(direction) = self.forward_direction() {
            self.position += direction * amount;
            self.target += direction * amount;
        }
    }

    fn move_right(&mut self, amount: f32) {
        let forward = (self.target - self.position).normalize();
        let right = forward.cross(&self.up).normalize();
        self.position += right * amount;
        self.target += right * amount;
    }

    fn move_up(&mut self, amount: f32) {
        self.position.y += amount;
        self.target.y += amount;
    }

    fn zoom_in(&mut self, amount: f32) {
        let Some(direction) = self.forward_direction() else {
            return;
        };
        let current_distance = (self.position - self.target).magnitude();

        // Zoom más lento cuando está cerca (para mejor control)
        let adjusted_amount = if current_distance < 200.0 {
            amount * 0.5
        } else if current_distance < 500.0 {
            amount * 0.75
        } else {
            amount
        };

        self.position += direction * adjusted_amount;

        let new_distance = (self.position - self.target).magnitude();
        if new_distance < MIN_CAMERA_TARGET_DISTANCE {
            self.position = self.target - direction * MIN_CAMERA_TARGET_DISTANCE;
        }
    }

    fn zoom_out(&mut self, amount: f32) {
        let Some(direction) = self.forward_direction() else {
            return;
        };
        let current_distance = (self.position - self.target).magnitude();

        // Zoom más rápido cuando está lejos
        let adjusted_amount = if current_distance > 2000.0 {
            amount * 1.5
        } else {
            amount
        };

        self.position -= adjusted_amount * direction;
    }
}

fn create_model_matrix(translation: Vec3, scale: f32, rotation: Vec3) -> Mat4 {
    let (sin_x, cos_x) = rotation.x.sin_cos();
    let (sin_y, cos_y) = rotation.y.sin_cos();
    let (sin_z, cos_z) = rotation.z.sin_cos();

    let rotation_matrix_x = Mat4::new(
        1.0, 0.0, 0.0, 0.0, 0.0, cos_x, -sin_x, 0.0, 0.0, sin_x, cos_x, 0.0, 0.0, 0.0, 0.0, 1.0,
    );

    let rotation_matrix_y = Mat4::new(
        cos_y, 0.0, sin_y, 0.0, 0.0, 1.0, 0.0, 0.0, -sin_y, 0.0, cos_y, 0.0, 0.0, 0.0, 0.0, 1.0,
    );

    let rotation_matrix_z = Mat4::new(
        cos_z, -sin_z, 0.0, 0.0, sin_z, cos_z, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    );

    let rotation_matrix = rotation_matrix_z * rotation_matrix_y * rotation_matrix_x;

    let transform_matrix = Mat4::new(
        scale,
        0.0,
        0.0,
        translation.x,
        0.0,
        scale,
        0.0,
        translation.y,
        0.0,
        0.0,
        scale,
        translation.z,
        0.0,
        0.0,
        0.0,
        1.0,
    );

    transform_matrix * rotation_matrix
}

fn create_projection_matrix(window_width: f32, window_height: f32) -> Mat4 {
    let fov = 45.0 * PI / 180.0;
    let aspect_ratio = window_width / window_height;
    let near = 0.1;
    let far = 1000.0;

    nalgebra_glm::perspective(aspect_ratio, fov, near, far)
}

struct AirwingPose {
    position: Vec3,
    forward: Vec3,
}

fn compute_airwing_pose(
    camera: &Camera,
    forward_adjust: f32,
    vertical_adjust: f32,
    lateral_adjust: f32,
) -> AirwingPose {
    let up_dir = camera.up.normalize();
    let forward_dir = {
        let dir = camera.target - camera.position;
        if dir.magnitude() < f32::EPSILON {
            Vec3::new(0.0, 0.0, -1.0)
        } else {
            dir.normalize()
        }
    };
    let right_dir = {
        let candidate = forward_dir.cross(&up_dir);
        if candidate.magnitude() < f32::EPSILON {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            candidate.normalize()
        }
    };
    // Mantener al airwing estable y claramente visible frente a la cámara
    let fixed_ship_distance = AIRWING_DISTANCE;

    let forward_delta = (forward_adjust * 0.15).clamp(-25.0, 25.0);
    let offset = forward_dir * (fixed_ship_distance + forward_delta)
        + up_dir * (AIRWING_VERTICAL_OFFSET + vertical_adjust)
        + right_dir * (AIRWING_SIDE_OFFSET + lateral_adjust);
    let position = camera.position + offset;

    AirwingPose {
        position,
        forward: forward_dir,
    }
}

fn stylized_airwing_offsets(
    ship_forward_state: f32,
    ship_vertical_state: f32,
    idle_vertical: f32,
    lateral_offset: f32,
    bank_state: f32,
    pitch_state: f32,
) -> (f32, f32, f32) {
    let thrust_ratio = (ship_forward_state / AIRWING_FORWARD_RANGE).clamp(-1.2, 1.2);
    let pitch_boost = pitch_state * 34.0;
    let bank_arc_forward = bank_state * 18.0;
    let surge_forward = thrust_ratio * 16.0;

    let animated_forward = ship_forward_state + pitch_boost + bank_arc_forward + surge_forward;
    let animated_vertical = ship_vertical_state + idle_vertical + bank_state * 7.0;
    let animated_lateral = lateral_offset + bank_state * 8.0;

    (animated_forward, animated_vertical, animated_lateral)
}

fn collision_radius(obj: &CelestialObject) -> f32 {
    if obj.ring.is_some() {
        // Permitir que la nave atraviese el mesh del anillo; sólo colisiona con el planeta.
        obj.scale * 0.85
    } else {
        obj.scale
    }
}

#[derive(Clone, Copy, Debug)]
struct CollisionHit {
    normal: Vec3,
    penetration: f32,
}

fn detect_ship_collision(
    ship_pos: Vec3,
    ship_radius: f32,
    objects: &[CelestialObject],
    moon: &CelestialObject,
) -> Option<CollisionHit> {
    fn test_target(
        target: &CelestialObject,
        ship_pos: Vec3,
        ship_radius: f32,
    ) -> Option<CollisionHit> {
        let combined = ship_radius + collision_radius(target);
        let delta = ship_pos - target.translation;
        let distance_sq = delta.magnitude_squared();
        if distance_sq >= combined * combined {
            return None;
        }

        let distance = distance_sq.sqrt().max(1e-4);
        let normal = if distance_sq < 1e-6 {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            delta / distance
        };
        let penetration = combined - distance;
        Some(CollisionHit {
            normal,
            penetration,
        })
    }

    let mut best_hit: Option<CollisionHit> = None;
    let mut consider = |target: &CelestialObject| {
        if let Some(hit) = test_target(target, ship_pos, ship_radius) {
            match best_hit {
                Some(existing) if existing.penetration >= hit.penetration => {}
                _ => best_hit = Some(hit),
            }
        }
    };

    for obj in objects.iter().skip(1) {
        consider(obj);
    }
    consider(moon);

    best_hit
}

// Asigna un nivel de detalle simple (0), medio (1) o completo (2) según la distancia.
fn check_lod(object_position: Vec3, object_radius: f32, camera: &Camera) -> usize {
    // Distancia del objeto hacia la cámara
    let to_object = object_position - camera.position;
    let distance = to_object.magnitude();

    // Muy cerca: malla mínima para evitar desperdicio de fill-rate
    if distance < object_radius * 4.0 {
        return 0;
    }

    // Distancia media: mantener pocos triángulos pero más detalle visual
    if distance < object_radius * 12.0 {
        return 1;
    }

    // Lejos: toda la malla disponible
    2
}

fn create_viewport_matrix(width: f32, height: f32) -> Mat4 {
    Mat4::new(
        width / 2.0,
        0.0,
        0.0,
        width / 2.0,
        0.0,
        -height / 2.0,
        0.0,
        height / 2.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    )
}

fn render(framebuffer: &mut Framebuffer, uniforms: &Uniforms, vertex_array: &[Vertex]) {
    use rayon::prelude::*;

    // Etapa de vértices: se ejecuta en paralelo para aprovechar todos los núcleos
    let transformed_vertices: Vec<Vertex> = vertex_array
        .par_iter()
        .map(|vertex| vertex_shader(vertex, uniforms))
        .collect();

    // Ensamblar triángulos y descartar los que miran en sentido contrario
    let mut triangles = Vec::new();
    for i in (0..transformed_vertices.len()).step_by(3) {
        if i + 2 < transformed_vertices.len() {
            let v0 = &transformed_vertices[i].transformed_position;
            let v1 = &transformed_vertices[i + 1].transformed_position;
            let v2 = &transformed_vertices[i + 2].transformed_position;

            let edge1_x = v1.x - v0.x;
            let edge1_y = v1.y - v0.y;
            let edge2_x = v2.x - v0.x;
            let edge2_y = v2.y - v0.y;
            let cross = edge1_x * edge2_y - edge1_y * edge2_x;

            if cross > 0.0 {
                triangles.push([
                    transformed_vertices[i].clone(),
                    transformed_vertices[i + 1].clone(),
                    transformed_vertices[i + 2].clone(),
                ]);
            }
        }
    }

    // Rasterizar cada triángulo y ejecutar su shader de fragmentos en paralelo
    let fragments: Vec<_> = triangles
        .par_iter()
        .flat_map(|tri| {
            let frags = triangle(&tri[0], &tri[1], &tri[2]);
            frags
                .into_iter()
                .map(|mut frag| {
                    let shader_color =
                        get_celestial_shader(uniforms.current_shader, &frag, &tri[0], uniforms);
                    frag.color = shader_color;
                    frag
                })
                .collect::<Vec<_>>()
        })
        .collect();

    // Al escribir en el framebuffer se hace de forma secuencial para proteger el z-buffer
    for frag in fragments {
        let x = frag.position.x as usize;
        let y = frag.position.y as usize;
        if x < framebuffer.width && y < framebuffer.height {
            let color = frag.color.to_hex();
            framebuffer.set_current_color(color);
            framebuffer.point(x, y, frag.depth);
        }
    }
}

fn draw_orbit(
    framebuffer: &mut Framebuffer,
    center: Vec3,
    radius_x: f32,
    radius_z: f32,
    segments: u32,
    color: Color,
    view_matrix: &Mat4,
    projection_matrix: &Mat4,
    viewport_matrix: &Mat4,
) {
    if radius_x <= 0.0 || segments < 2 {
        return;
    }

    let mvp = projection_matrix * view_matrix;
    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * 2.0 * PI;
        let angle2 = ((i + 1) as f32 / segments as f32) * 2.0 * PI;

        let p1_world = Vec3::new(
            center.x + radius_x * angle1.cos(),
            center.y,
            center.z + radius_z * angle1.sin(),
        );
        let p2_world = Vec3::new(
            center.x + radius_x * angle2.cos(),
            center.y,
            center.z + radius_z * angle2.sin(),
        );

        let p1_clip = mvp * Vec4::new(p1_world.x, p1_world.y, p1_world.z, 1.0);
        let p2_clip = mvp * Vec4::new(p2_world.x, p2_world.y, p2_world.z, 1.0);

        if p1_clip.w.abs() < 0.001 || p2_clip.w.abs() < 0.001 {
            continue;
        }

        let p1_ndc = Vec3::new(
            p1_clip.x / p1_clip.w,
            p1_clip.y / p1_clip.w,
            p1_clip.z / p1_clip.w,
        );
        let p2_ndc = Vec3::new(
            p2_clip.x / p2_clip.w,
            p2_clip.y / p2_clip.w,
            p2_clip.z / p2_clip.w,
        );

        if p1_ndc.x.abs() > 2.0
            || p1_ndc.y.abs() > 2.0
            || p2_ndc.x.abs() > 2.0
            || p2_ndc.y.abs() > 2.0
            || p1_clip.w < 0.0
            || p2_clip.w < 0.0
        {
            continue;
        }

        let p1_screen = viewport_matrix * Vec4::new(p1_ndc.x, p1_ndc.y, p1_ndc.z, 1.0);
        let p2_screen = viewport_matrix * Vec4::new(p2_ndc.x, p2_ndc.y, p2_ndc.z, 1.0);

        if p1_screen.x.is_nan()
            || p1_screen.y.is_nan()
            || p2_screen.x.is_nan()
            || p2_screen.y.is_nan()
        {
            continue;
        }

        let mut v1 = Vertex::default();
        v1.position = p1_world;
        v1.color = color;
        v1.transformed_position = Vec3::new(p1_screen.x, p1_screen.y, p1_screen.z);

        let mut v2 = Vertex::default();
        v2.position = p2_world;
        v2.color = color;
        v2.transformed_position = Vec3::new(p2_screen.x, p2_screen.y, p2_screen.z);

        let fragments = line(&v1, &v2);
        for fragment in fragments {
            if fragment.position.x < 0.0 || fragment.position.y < 0.0 {
                continue;
            }

            let x = fragment.position.x as usize;
            let y = fragment.position.y as usize;
            if x < framebuffer.width && y < framebuffer.height {
                framebuffer.set_current_color(color.to_hex());
                framebuffer.point(x, y, fragment.depth);
            }
        }
    }
}

fn generate_stars(count: usize, radius: f32, center: Vec3) -> Vec<Star> {
    let mut rng = Lcg::new(0xC0FFEEu64 + (radius as u64));
    let mut stars = Vec::with_capacity(count);
    for _ in 0..count {
        let u = rng.next_f32() * 2.0 - 1.0;
        let theta = rng.next_f32() * 2.0 * PI;
        let r = (1.0 - u * u).sqrt();
        let position = Vec3::new(
            center.x + radius * r * theta.cos(),
            center.y + radius * u,
            center.z + radius * r * theta.sin(),
        );
        let base = 180.0 + rng.next_f32() * 75.0;
        let flicker = (rng.next_f32() * 30.0).sin().abs() * 40.0;
        let intensity = (base + flicker).clamp(160.0, 255.0) as u8;
        stars.push(Star {
            position,
            color: Color::new(intensity, intensity, intensity),
        });
    }
    stars
}

fn draw_stars(
    framebuffer: &mut Framebuffer,
    stars: &[Star],
    view_matrix: &Mat4,
    projection_matrix: &Mat4,
    viewport_matrix: &Mat4,
) {
    use rayon::prelude::*;

    let vp = projection_matrix * view_matrix;
    let viewport = *viewport_matrix;

    let star_points: Vec<(usize, usize, u32)> = stars
        .par_iter()
        .filter_map(|star| {
            let clip = vp * Vec4::new(star.position.x, star.position.y, star.position.z, 1.0);
            if clip.w.abs() < 0.001 || clip.z < -clip.w {
                return None;
            }
            let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);
            if ndc.x.abs() > 1.1 || ndc.y.abs() > 1.1 || ndc.z > 1.1 {
                return None;
            }
            let screen = viewport * Vec4::new(ndc.x, ndc.y, ndc.z, 1.0);
            if !screen.x.is_finite() || !screen.y.is_finite() {
                return None;
            }
            let x = screen.x.round() as isize;
            let y = screen.y.round() as isize;
            if x < 0 || y < 0 {
                return None;
            }
            let (x, y) = (x as usize, y as usize);
            if x >= framebuffer.width || y >= framebuffer.height {
                return None;
            }
            Some((x, y, star.color.to_hex()))
        })
        .collect();

    for (x, y, color) in star_points {
        framebuffer.set_current_color(color);
        framebuffer.point(x, y, STAR_DEPTH);
    }
}

fn sphere_visible(position: Vec3, radius: f32, view_projection: &Mat4) -> bool {
    let clip = view_projection * Vec4::new(position.x, position.y, position.z, 1.0);
    if clip.w.abs() < 0.001 {
        return false;
    }

    let inv_w = 1.0 / clip.w;
    let ndc = Vec3::new(clip.x * inv_w, clip.y * inv_w, clip.z * inv_w);
    let margin = (radius / 400.0).clamp(0.05, 0.35);

    ndc.x >= -1.2 - margin
        && ndc.x <= 1.2 + margin
        && ndc.y >= -1.2 - margin
        && ndc.y <= 1.2 + margin
        && ndc.z >= -1.2 - margin
        && ndc.z <= 1.2 + margin
}

fn orbit_segment_count(distance_to_camera: f32, radius: f32) -> u32 {
    let base = if distance_to_camera > 2200.0 {
        120
    } else if distance_to_camera > 1200.0 {
        100
    } else if distance_to_camera > 600.0 {
        80
    } else {
        60
    };

    let radius_factor = (radius / 2000.0).clamp(0.5, 1.4);
    let segments = (base as f32 * radius_factor).round() as u32;
    segments.clamp(32, 140)
}

fn orbit_color(body: CelestialBody) -> Color {
    let _ = body; // Orbit lines all share the same white tint
    Color::new(255, 255, 255)
}

struct CelestialObject {
    body_type: CelestialBody,
    translation: Vec3,
    rotation: Vec3,
    scale: f32,
    rotation_speed: Vec3,
    orbit_speed: f32,
    orbit_radius: f32,
    orbit_minor_ratio: f32,
    orbit_center: Vec3,
    orbit_phase: f32,
    current_orbit_angle: f32,
    use_large_sphere: bool,
    ring: Option<RingConfig>,
}

impl CelestialObject {
    fn new(
        body_type: CelestialBody,
        translation: Vec3,
        scale: f32,
        use_large_sphere: bool,
    ) -> Self {
        CelestialObject {
            body_type,
            translation,
            rotation: Vec3::new(0.0, 0.0, 0.0),
            scale,
            rotation_speed: Vec3::new(0.0, 0.01, 0.0),
            orbit_speed: 0.0,
            orbit_radius: 0.0,
            orbit_minor_ratio: 1.0,
            orbit_center: Vec3::new(600.0, 400.0, 0.0),
            orbit_phase: 0.0,
            current_orbit_angle: 0.0,
            use_large_sphere,
            ring: None,
        }
    }

    fn with_orbit(mut self, radius: f32, speed: f32) -> Self {
        self.orbit_radius = radius;
        self.orbit_speed = speed;
        self
    }

    fn with_orbit_shape(mut self, minor_ratio: f32) -> Self {
        self.orbit_minor_ratio = minor_ratio;
        self
    }

    fn with_orbit_phase(mut self, phase: f32) -> Self {
        self.orbit_phase = phase;
        self.current_orbit_angle = phase;
        self
    }

    fn with_orbit_center(mut self, center: Vec3) -> Self {
        self.orbit_center = center;
        self
    }

    fn with_rotation_speed(mut self, speed: Vec3) -> Self {
        self.rotation_speed = speed;
        self
    }

    fn with_ring(mut self, config: RingConfig) -> Self {
        self.ring = Some(config);
        self
    }

    fn update(&mut self, delta_time: f32) {
        let clamped_dt = delta_time.clamp(0.0, 0.05);

        // Rotación propia (mantener ajustes existentes con base en 60 FPS)
        self.rotation = self.rotation + self.rotation_speed * clamped_dt * 60.0;

        // Órbita
        if self.orbit_radius > 0.0 {
            self.current_orbit_angle += self.orbit_speed * clamped_dt;
            if self.current_orbit_angle > 2.0 * PI {
                self.current_orbit_angle -= 2.0 * PI;
            }
            if self.current_orbit_angle < 0.0 {
                self.current_orbit_angle += 2.0 * PI;
            }

            let angle = self.current_orbit_angle;
            self.translation.x = self.orbit_center.x + angle.cos() * self.orbit_radius;
            self.translation.z =
                self.orbit_center.z + angle.sin() * self.orbit_radius * self.orbit_minor_ratio;
        }

        if let Some(ring) = &mut self.ring {
            ring.rotation = ring.rotation + ring.rotation_speed * clamped_dt * 60.0;
        }
    }
}

#[derive(Clone)]
struct RingConfig {
    scale_multiplier: f32,
    vertical_offset: f32,
    tilt: Vec3,
    rotation: Vec3,
    rotation_speed: Vec3,
    palette: [Color; 3],
}

impl RingConfig {
    fn new(scale_multiplier: f32) -> Self {
        RingConfig {
            scale_multiplier,
            vertical_offset: 0.0,
            tilt: Vec3::new(0.0, 0.0, 0.0),
            rotation: Vec3::new(0.0, 0.0, 0.0),
            rotation_speed: Vec3::new(0.0, 0.0, 0.0),
            palette: [
                Color::from_float(0.96, 0.82, 0.44), // Dorado claro
                Color::from_float(0.78, 0.58, 0.30), // Ámbar
                Color::from_float(0.58, 0.40, 0.20), // Café rojizo
            ],
        }
    }

    fn with_vertical_offset(mut self, offset: f32) -> Self {
        self.vertical_offset = offset;
        self
    }

    fn with_tilt(mut self, tilt: Vec3) -> Self {
        self.tilt = tilt;
        self
    }

    fn with_rotation_speed(mut self, speed: Vec3) -> Self {
        self.rotation_speed = speed;
        self
    }

    fn with_palette(mut self, colors: [Color; 3]) -> Self {
        self.palette = colors;
        self
    }
}

fn main() {
    let window_width = 1200;
    let window_height = 800;
    let render_scale = 1.0f32;
    let framebuffer_width = ((window_width as f32 * render_scale).round() as usize).max(1);
    let framebuffer_height = ((window_height as f32 * render_scale).round() as usize).max(1);
    let frame_delay = Duration::from_millis(16);

    let mut framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);
    let mut window = Window::new(
        "Solar System - Celestial Bodies Renderer",
        window_width,
        window_height,
        WindowOptions::default(),
    )
    .unwrap();

    window.set_position(200, 100);
    window.update();

    framebuffer.set_background_color(0x000011);

    let audio_device = RaylibAudio::init_audio_device().ok();
    let background_music: Option<Music> = audio_device.as_ref().and_then(|audio| {
        let asset_path = locate_asset("sonido1")?;
        let path_string = asset_path.to_string_lossy().into_owned();
        match audio.new_music(&path_string) {
            Ok(music) => Some(music),
            Err(err) => {
                eprintln!("Aviso: no se pudo cargar el audio 'sonido1': {err}");
                None
            }
        }
    });

    if background_music.is_none() {
        if audio_device.is_none() {
            eprintln!("Aviso: no se pudo inicializar el dispositivo de audio de raylib.");
        } else {
            eprintln!("Aviso: no se encontró archivo 'sonido1' en assets/.");
        }
    } else if let Some(music) = background_music.as_ref() {
        music.set_volume(0.35);
        music.play_stream();
    }

    let sun_position = Vec3::new(600.0, 400.0, 0.0);
    let stars = generate_stars(STAR_COUNT, STARFIELD_RADIUS, sun_position);

    // Todos los planetas comparten la malla Esfera_Low para simplificar el render
    let sphere_low = Obj::load("models/Esfera_Low.obj").expect("Failed to load Esfera_Low.obj");
    let sphere_low_vertices = sphere_low.get_vertex_array();
    let ring_mesh = Obj::load("models/anillo.obj").expect("Failed to load anillo.obj");
    let ring_vertices = ring_mesh.get_vertex_array();
    let airwing_mesh = Obj::load("models/airwing.obj").expect("Failed to load airwing.obj");
    let airwing_vertices = airwing_mesh.get_vertex_array();
    // Definición de los cuerpos celestes con órbitas separadas y la misma malla base
    let mut celestial_objects = vec![
        // Sol (centro)
        CelestialObject::new(CelestialBody::Sun, sun_position, 100.0, false)
            .with_rotation_speed(Vec3::new(0.0, 0.005, 0.0)),
        // Tierra
        CelestialObject::new(CelestialBody::Earth, sun_position, 40.0, false)
            .with_orbit_center(sun_position)
            .with_orbit(900.0, 0.22)
            .with_orbit_shape(0.55)
            .with_orbit_phase(PI * 0.12)
            .with_rotation_speed(Vec3::new(0.0, 0.02, 0.0)),
        // Planeta café (usamos el shader de Marte)
        CelestialObject::new(CelestialBody::Mars, sun_position, 34.0, false)
            .with_orbit_center(sun_position)
            .with_orbit(1300.0, 0.16)
            .with_orbit_shape(0.6)
            .with_orbit_phase(PI * 0.33)
            .with_rotation_speed(Vec3::new(0.0, 0.018, 0.0)),
        // Saturno (único con anillos)
        CelestialObject::new(CelestialBody::Saturn, sun_position, 68.0, false)
            .with_orbit_center(sun_position)
            .with_orbit(1900.0, 0.11)
            .with_orbit_shape(0.68)
            .with_orbit_phase(PI * 0.68)
            .with_rotation_speed(Vec3::new(0.0, 0.025, 0.0))
            .with_ring(
                RingConfig::new(1.4)
                    .with_tilt(Vec3::new(0.32, 0.0, 0.05))
                    .with_rotation_speed(Vec3::new(0.0, 0.01, 0.0))
                    .with_palette([
                        Color::from_float(0.98, 0.86, 0.52),
                        Color::from_float(0.83, 0.64, 0.33),
                        Color::from_float(0.63, 0.44, 0.22),
                    ]),
            ),
        // IcePlanet (órbita amplia y lenta)
        CelestialObject::new(CelestialBody::IcePlanet, sun_position, 48.0, false)
            .with_orbit_center(sun_position)
            .with_orbit(2400.0, 0.085)
            .with_orbit_shape(0.6)
            .with_orbit_phase(PI * 0.04)
            .with_rotation_speed(Vec3::new(0.0, 0.022, 0.0)),
        // AlienPlanet (el más lejano)
        CelestialObject::new(CelestialBody::AlienPlanet, sun_position, 52.0, false)
            .with_orbit_center(sun_position)
            .with_orbit(3000.0, 0.07)
            .with_orbit_shape(0.58)
            .with_orbit_phase(PI * 0.5)
            .with_rotation_speed(Vec3::new(0.0, 0.019, 0.0)),
    ];

    // La luna reusa la esfera pequeña y permanece cerca de la Tierra
    let mut earth_moon = CelestialObject::new(CelestialBody::Moon, sun_position, 10.0, false)
        .with_orbit(20.0, 1.2) // Orbita a poca distancia para que siempre acompañe
        .with_orbit_shape(0.5)
        .with_rotation_speed(Vec3::new(0.0, 0.01, 0.0));

    let farthest_orbit_radius = celestial_objects
        .iter()
        .map(|obj| {
            if obj.orbit_radius > 0.0 {
                obj.orbit_radius + obj.scale
            } else {
                let dx = obj.translation.x - sun_position.x;
                let dy = obj.translation.y - sun_position.y;
                let dz = obj.translation.z - sun_position.z;
                (dx * dx + dy * dy + dz * dz).sqrt() + obj.scale
            }
        })
        .fold(0.0_f32, f32::max);
    let close_camera_render_distance =
        farthest_orbit_radius + AIRWING_FORWARD_RANGE + AIRWING_COLLISION_RADIUS + 200.0;

    let mut time = 0.0f32;
    let mut last_frame = Instant::now();
    let mut show_orbits = true;
    let mut ship_vertical_state = 0.0f32;
    let mut ship_forward_state = 0.0f32;
    let mut ship_idle_phase = 0.0f32;

    // La cámara arranca en una posición elevada para abarcar el sistema completo
    let mut camera = Camera::new(
        Vec3::new(
            sun_position.x,
            sun_position.y + 420.0,
            sun_position.z + 3300.0,
        ),
        sun_position,
    );

    let projection_matrix = create_projection_matrix(window_width as f32, window_height as f32);
    let viewport_matrix =
        create_viewport_matrix(framebuffer_width as f32, framebuffer_height as f32);
    let mut airwing_bank_state = 0.0f32;
    let mut airwing_pitch_state = 0.0f32;

    while window.is_open() {
        let frame_start = Instant::now();
        let mut delta_time = frame_start.duration_since(last_frame).as_secs_f32();
        last_frame = frame_start;
        if !delta_time.is_finite() {
            delta_time = 0.0;
        }
        delta_time = delta_time.clamp(0.0, 0.05);

        if window.is_key_down(Key::Escape) {
            break;
        }

        if window.is_key_pressed(Key::O, KeyRepeat::No) {
            show_orbits = !show_orbits;
        }

        if let Some(music) = background_music.as_ref() {
            music.update_stream();
            if !music.is_stream_playing() {
                music.play_stream();
            }
        }

        let previous_camera = camera.clone();
        handle_input(&window, &mut camera);

        let camera_changed = (camera.position - previous_camera.position).magnitude_squared()
            > 0.001
            || (camera.target - previous_camera.target).magnitude_squared() > 0.001;
        if camera_changed {
            airwing_bank_state = 0.0;
            airwing_pitch_state = 0.0;
            ship_vertical_state = 0.0;
            ship_forward_state = 0.0;
        }

        let bank_target = if window.is_key_down(Key::Left) {
            0.85
        } else if window.is_key_down(Key::Right) {
            -0.85
        } else {
            0.0
        };

        let pitch_target = if window.is_key_down(Key::W) {
            0.65
        } else if window.is_key_down(Key::S) {
            -0.65
        } else {
            0.0
        };

        let ship_vertical_target = if window.is_key_down(Key::W) {
            AIRWING_EXTRA_VERTICAL_RANGE
        } else if window.is_key_down(Key::S) {
            -AIRWING_EXTRA_VERTICAL_RANGE
        } else {
            0.0
        };

        let smoothing = 0.12;
        airwing_bank_state += (bank_target - airwing_bank_state) * smoothing;
        airwing_pitch_state += (pitch_target - airwing_pitch_state) * smoothing;
        ship_vertical_state += (ship_vertical_target - ship_vertical_state) * 0.10;
        ship_vertical_state =
            ship_vertical_state.clamp(-AIRWING_EXTRA_VERTICAL_RANGE, AIRWING_EXTRA_VERTICAL_RANGE);

        let ship_thrust_target = if window.is_key_down(Key::W) {
            AIRWING_FORWARD_RANGE
        } else if window.is_key_down(Key::S) {
            -AIRWING_FORWARD_RANGE * 0.7
        } else {
            -10.0
        };
        ship_forward_state += (ship_thrust_target - ship_forward_state) * 0.08;
        ship_forward_state =
            ship_forward_state.clamp(-AIRWING_FORWARD_RANGE, AIRWING_FORWARD_RANGE);

        ship_idle_phase += delta_time * 1.8;
        if ship_idle_phase > 2.0 * PI {
            ship_idle_phase -= 2.0 * PI;
        }

        let lateral_offset = airwing_bank_state * 12.0;
        let idle_vertical = ship_idle_phase.sin() * 4.0;
        let (mut animated_forward, mut animated_vertical, mut animated_lateral) =
            stylized_airwing_offsets(
                ship_forward_state,
                ship_vertical_state,
                idle_vertical,
                lateral_offset,
                airwing_bank_state,
                airwing_pitch_state,
            );
        let mut airwing_pose = compute_airwing_pose(
            &camera,
            animated_forward,
            animated_vertical,
            animated_lateral,
        );

        let mut collision_iterations = 0;
        let mut collision_unresolved = false;
        while let Some(hit) = detect_ship_collision(
            airwing_pose.position,
            AIRWING_COLLISION_RADIUS,
            &celestial_objects,
            &earth_moon,
        ) {
            collision_iterations += 1;
            if collision_iterations > 4 {
                camera = previous_camera;
                collision_unresolved = true;
                break;
            }

            let correction = hit.normal * (hit.penetration + 1.5);
            camera.position += correction;
            camera.target += correction;

            if ship_forward_state > 0.0 {
                ship_forward_state *= 0.35;
            }

            (animated_forward, animated_vertical, animated_lateral) = stylized_airwing_offsets(
                ship_forward_state,
                ship_vertical_state,
                idle_vertical,
                lateral_offset,
                airwing_bank_state,
                airwing_pitch_state,
            );
            airwing_pose = compute_airwing_pose(
                &camera,
                animated_forward,
                animated_vertical,
                animated_lateral,
            );
        }

        if collision_unresolved {
            (animated_forward, animated_vertical, animated_lateral) = stylized_airwing_offsets(
                ship_forward_state,
                ship_vertical_state,
                idle_vertical,
                lateral_offset,
                airwing_bank_state,
                airwing_pitch_state,
            );
            airwing_pose = compute_airwing_pose(
                &camera,
                animated_forward,
                animated_vertical,
                animated_lateral,
            );
        }

        let distance_to_target = (camera.position - camera.target).magnitude();

        framebuffer.clear();

        time += delta_time;

        let view_matrix = camera.get_view_matrix();
        let view_projection_matrix = projection_matrix * view_matrix;
        draw_stars(
            &mut framebuffer,
            &stars,
            &view_matrix,
            &projection_matrix,
            &viewport_matrix,
        );

        // Actualizar posiciones
        for obj in celestial_objects.iter_mut() {
            obj.update(delta_time);
        }

        // Actualizar luna de la Tierra
        earth_moon.orbit_center = celestial_objects[1].translation; // La Tierra ahora es el índice 1
        earth_moon.update(delta_time);

        let render_orbits = show_orbits && distance_to_target > 220.0;
        if render_orbits {
            for celestial_obj in &celestial_objects {
                if celestial_obj.orbit_radius <= 0.0 {
                    continue;
                }
                let segments = orbit_segment_count(distance_to_target, celestial_obj.orbit_radius);
                draw_orbit(
                    &mut framebuffer,
                    celestial_obj.orbit_center,
                    celestial_obj.orbit_radius,
                    celestial_obj.orbit_radius * celestial_obj.orbit_minor_ratio,
                    segments,
                    orbit_color(celestial_obj.body_type),
                    &view_matrix,
                    &projection_matrix,
                    &viewport_matrix,
                );
            }

            let moon_segments = orbit_segment_count(distance_to_target, earth_moon.orbit_radius);
            draw_orbit(
                &mut framebuffer,
                earth_moon.orbit_center,
                earth_moon.orbit_radius,
                earth_moon.orbit_radius * earth_moon.orbit_minor_ratio,
                moon_segments,
                orbit_color(CelestialBody::Moon),
                &view_matrix,
                &projection_matrix,
                &viewport_matrix,
            );
        }

        // La posición del Sol es la fuente de luz
        let light_position = celestial_objects[0].translation;

        // Posición del airwing, siempre frente a la cámara
        let airwing_position = airwing_pose.position;
        let forward_dir = airwing_pose.forward;
        let yaw = forward_dir.z.atan2(forward_dir.x);
        let pitch = (-forward_dir.y).asin();
        let pitch_tilt = (pitch * 1.1 + airwing_pitch_state).clamp(-1.2, 1.2);
        let bank_tilt = (airwing_bank_state).clamp(-0.9, 0.9);
        let thrust_ratio = (ship_forward_state / AIRWING_FORWARD_RANGE).clamp(-1.0, 1.0);
        let yaw_sway = airwing_bank_state * 0.25;
        let roll_shake = thrust_ratio * 0.2 + (ship_idle_phase * 1.7).sin() * 0.05;
        let pitch_sweep = airwing_bank_state.abs() * 0.12
            + (ship_vertical_state / AIRWING_EXTRA_VERTICAL_RANGE).clamp(-1.0, 1.0) * 0.1;
        let airwing_rotation = Vec3::new(
            pitch_tilt + PI + pitch_sweep,
            -yaw + PI / 2.0 + yaw_sway,
            bank_tilt + roll_shake,
        );

        // Nivel de detalle basado en distancia (más cerca = MÁS detalle, como debe ser)
        let detail_level = if distance_to_target < 200.0 {
            1.0 // Muy cerca: máximo detalle
        } else if distance_to_target < 400.0 {
            0.85 // Cerca: muy buen detalle
        } else if distance_to_target < 800.0 {
            0.65 // Media: buen detalle
        } else if distance_to_target < 1500.0 {
            0.45 // Lejos: detalle reducido
        } else {
            0.3 // Muy lejos: mínimo detalle para optimizar rendimiento
        };

        let max_object_render_distance = if distance_to_target > 700.0 {
            f32::INFINITY
        } else {
            close_camera_render_distance
        };

        // Renderizar todos los cuerpos usando Esfera_Low.obj (178 vértices, 192 caras - MÁXIMO rendimiento)
        for (index, celestial_obj) in celestial_objects.iter().enumerate() {
            if index != 0 {
                let distance_to_camera = (celestial_obj.translation - camera.position).magnitude();
                if distance_to_camera > max_object_render_distance {
                    continue;
                }

                if !sphere_visible(
                    celestial_obj.translation,
                    celestial_obj.scale * 1.6,
                    &view_projection_matrix,
                ) {
                    continue;
                }
            }

            let model_matrix = create_model_matrix(
                celestial_obj.translation,
                celestial_obj.scale,
                celestial_obj.rotation,
            );
            let uniforms = Uniforms {
                model_matrix,
                view_matrix,
                projection_matrix,
                time,
                current_shader: celestial_obj.body_type,
                light_position,
                camera_position: camera.position,
                detail_level,
                ring_palette: None,
            };

            // TODOS usan Esfera_Low.obj (178 vértices, 192 caras) para MÁXIMO rendimiento
            render(&mut framebuffer, &uniforms, &sphere_low_vertices);

            if let Some(ring) = &celestial_obj.ring {
                let mut ring_translation = celestial_obj.translation;
                ring_translation.y += ring.vertical_offset;
                let ring_model_matrix = create_model_matrix(
                    ring_translation,
                    celestial_obj.scale * ring.scale_multiplier,
                    celestial_obj.rotation + ring.tilt + ring.rotation,
                );
                let ring_uniforms = Uniforms {
                    model_matrix: ring_model_matrix,
                    current_shader: CelestialBody::Ring,
                    ring_palette: Some(ring.palette),
                    ..uniforms
                };
                render(&mut framebuffer, &ring_uniforms, &ring_vertices);
            }
        }

        // Renderizar luna (SIEMPRE - sin frustum culling)
        let moon_matrix = create_model_matrix(
            earth_moon.translation,
            earth_moon.scale,
            earth_moon.rotation,
        );
        let moon_uniforms = Uniforms {
            model_matrix: moon_matrix,
            view_matrix,
            projection_matrix,
            time,
            current_shader: CelestialBody::Moon,
            light_position,
            camera_position: camera.position,
            detail_level,
            ring_palette: None,
        };
        // La luna usa Esfera_Low.obj (máximo rendimiento)
        render(&mut framebuffer, &moon_uniforms, &sphere_low_vertices);

        // Renderizar el airwing siguiendo a la cámara
        let airwing_model_matrix =
            create_model_matrix(airwing_position, AIRWING_SCALE, airwing_rotation);
        let airwing_uniforms = Uniforms {
            model_matrix: airwing_model_matrix,
            view_matrix,
            projection_matrix,
            time,
            current_shader: CelestialBody::Airwing,
            light_position,
            camera_position: camera.position,
            detail_level: 1.0,
            ring_palette: None,
        };
        render(&mut framebuffer, &airwing_uniforms, &airwing_vertices);

        if framebuffer_width != window_width || framebuffer_height != window_height {
            let resampled = resample_buffer(
                &framebuffer.buffer,
                framebuffer_width,
                framebuffer_height,
                window_width,
                window_height,
            );
            window
                .update_with_buffer(&resampled, window_width, window_height)
                .unwrap();
        } else {
            window
                .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
                .unwrap();
        }

        let frame_elapsed = frame_start.elapsed();
        if frame_elapsed < frame_delay {
            std::thread::sleep(frame_delay - frame_elapsed);
        }
    }
}

// Reescalar el framebuffer para adaptarlo al tamaño de la ventana
fn resample_buffer(
    src: &[u32],
    src_width: usize,
    src_height: usize,
    dst_width: usize,
    dst_height: usize,
) -> Vec<u32> {
    if src_width == dst_width && src_height == dst_height {
        return src.to_vec();
    }

    let mut dst = vec![0u32; dst_width * dst_height];
    let scale_x = src_width as f32 / dst_width as f32;
    let scale_y = src_height as f32 / dst_height as f32;

    for y in 0..dst_height {
        let src_y = ((y as f32 + 0.5) * scale_y - 0.5).clamp(0.0, src_height as f32 - 1.0);
        let y0 = src_y.floor();
        let y1 = (y0 + 1.0).min(src_height as f32 - 1.0);
        let ty = src_y - y0;
        let y0i = y0 as usize;
        let y1i = y1 as usize;

        for x in 0..dst_width {
            let src_x = ((x as f32 + 0.5) * scale_x - 0.5).clamp(0.0, src_width as f32 - 1.0);
            let x0 = src_x.floor();
            let x1 = (x0 + 1.0).min(src_width as f32 - 1.0);
            let tx = src_x - x0;
            let x0i = x0 as usize;
            let x1i = x1 as usize;

            let c00 = unpack_color(src[y0i * src_width + x0i]);
            let c10 = unpack_color(src[y0i * src_width + x1i]);
            let c01 = unpack_color(src[y1i * src_width + x0i]);
            let c11 = unpack_color(src[y1i * src_width + x1i]);

            let top = lerp_color(c00, c10, tx);
            let bottom = lerp_color(c01, c11, tx);
            let blended = lerp_color(top, bottom, ty);

            dst[y * dst_width + x] = pack_color(blended);
        }
    }

    dst
}

fn unpack_color(color: u32) -> (f32, f32, f32) {
    (
        ((color >> 16) & 0xFF) as f32,
        ((color >> 8) & 0xFF) as f32,
        (color & 0xFF) as f32,
    )
}

fn pack_color((r, g, b): (f32, f32, f32)) -> u32 {
    let r = r.clamp(0.0, 255.0) as u32;
    let g = g.clamp(0.0, 255.0) as u32;
    let b = b.clamp(0.0, 255.0) as u32;
    (r << 16) | (g << 8) | b
}

fn lerp_color(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

fn handle_input(window: &Window, camera: &mut Camera) {
    let move_speed = 10.0;
    let rotate_speed = 0.02;
    let zoom_speed = 20.0;

    // WASD desplaza la cámara en los ejes básicos
    if window.is_key_down(Key::A) {
        camera.move_right(-move_speed);
    }
    if window.is_key_down(Key::D) {
        camera.move_right(move_speed);
    }
    if window.is_key_down(Key::W) {
        camera.move_up(move_speed);
    }
    if window.is_key_down(Key::S) {
        camera.move_up(-move_speed);
    }

    // Las flechas izquierda/derecha orbitan alrededor del punto de interés
    if window.is_key_down(Key::Left) {
        camera.orbit(-rotate_speed, 0.0);
    }
    if window.is_key_down(Key::Right) {
        camera.orbit(rotate_speed, 0.0);
    }

    // Flechas arriba/abajo avanzan o retroceden sin perder orientación
    if window.is_key_down(Key::Up) {
        camera.move_forward(zoom_speed);
    }
    if window.is_key_down(Key::Down) {
        camera.move_forward(-zoom_speed);
    }

    // Z y X acercan o alejan el punto observado
    if window.is_key_down(Key::Z) {
        camera.zoom_in(zoom_speed);
    }
    if window.is_key_down(Key::X) {
        camera.zoom_out(zoom_speed);
    }
}
