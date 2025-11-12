use nalgebra_glm::{Vec3, Mat4};
use minifb::{Key, Window, WindowOptions};
use std::time::Duration;
use std::f32::consts::PI;

mod framebuffer;
mod triangle;
mod line;
mod vertex;
mod obj;
mod color;
mod fragment;
mod shaders;
mod celestial_shaders;

use framebuffer::Framebuffer;
use vertex::Vertex;
use obj::Obj;
use triangle::triangle;
use shaders::vertex_shader;
use celestial_shaders::{CelestialBody, get_celestial_shader};


pub struct Uniforms {
    model_matrix: Mat4,
    view_matrix: Mat4,
    projection_matrix: Mat4,
    time: f32,
    current_shader: CelestialBody,
    light_position: Vec3,
    camera_position: Vec3,
}

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
        let direction = (self.target - self.position).normalize();
        self.position += direction * amount;
        self.target += direction * amount;
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
        let direction = (self.target - self.position).normalize();
        self.position += direction * amount;
        
        // No acercarse demasiado
        let distance = (self.position - self.target).magnitude();
        if distance < 50.0 {
            self.position = self.target - direction * 50.0;
        }
    }

    fn zoom_out(&mut self, amount: f32) {
        let direction = (self.target - self.position).normalize();
        self.position -= direction * amount;
        
        // No alejarse demasiado
        let distance = (self.position - self.target).magnitude();
        if distance > 2000.0 {
            self.position = self.target - direction * 2000.0;
        }
    }
}

fn create_model_matrix(translation: Vec3, scale: f32, rotation: Vec3) -> Mat4 {
    let (sin_x, cos_x) = rotation.x.sin_cos();
    let (sin_y, cos_y) = rotation.y.sin_cos();
    let (sin_z, cos_z) = rotation.z.sin_cos();

    let rotation_matrix_x = Mat4::new(
        1.0,  0.0,    0.0,   0.0,
        0.0,  cos_x, -sin_x, 0.0,
        0.0,  sin_x,  cos_x, 0.0,
        0.0,  0.0,    0.0,   1.0,
    );

    let rotation_matrix_y = Mat4::new(
        cos_y,  0.0,  sin_y, 0.0,
        0.0,    1.0,  0.0,   0.0,
        -sin_y, 0.0,  cos_y, 0.0,
        0.0,    0.0,  0.0,   1.0,
    );

    let rotation_matrix_z = Mat4::new(
        cos_z, -sin_z, 0.0, 0.0,
        sin_z,  cos_z, 0.0, 0.0,
        0.0,    0.0,  1.0, 0.0,
        0.0,    0.0,  0.0, 1.0,
    );

    let rotation_matrix = rotation_matrix_z * rotation_matrix_y * rotation_matrix_x;

    let transform_matrix = Mat4::new(
        scale, 0.0,   0.0,   translation.x,
        0.0,   scale, 0.0,   translation.y,
        0.0,   0.0,   scale, translation.z,
        0.0,   0.0,   0.0,   1.0,
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

fn create_viewport_matrix(width: f32, height: f32) -> Mat4 {
    Mat4::new(
        width / 2.0, 0.0, 0.0, width / 2.0,
        0.0, -height / 2.0, 0.0, height / 2.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    )
}

fn render(framebuffer: &mut Framebuffer, uniforms: &Uniforms, vertex_array: &[Vertex]) {
    // Vertex Shader Stage
    let mut transformed_vertices = Vec::with_capacity(vertex_array.len());
    for vertex in vertex_array {
        let transformed = vertex_shader(vertex, uniforms);
        transformed_vertices.push(transformed);
    }

    // Primitive Assembly Stage
    let mut triangles = Vec::new();
    for i in (0..transformed_vertices.len()).step_by(3) {
        if i + 2 < transformed_vertices.len() {
            triangles.push([
                transformed_vertices[i].clone(),
                transformed_vertices[i + 1].clone(),
                transformed_vertices[i + 2].clone(),
            ]);
        }
    }

    // Rasterization Stage and Fragment Processing
    for tri in &triangles {
        let frags = triangle(&tri[0], &tri[1], &tri[2]);
        for mut frag in frags {
            // Apply celestial shader
            // Use the first vertex of the triangle as reference for position/normal
            let shader_color = get_celestial_shader(uniforms.current_shader, &frag, &tri[0], uniforms);
            frag.color = shader_color;
            
            // Fragment Processing Stage
            let x = frag.position.x as usize;
            let y = frag.position.y as usize;
            if x < framebuffer.width && y < framebuffer.height {
                let color = frag.color.to_hex();
                framebuffer.set_current_color(color);
                framebuffer.point(x, y, frag.depth);
            }
        }
    }
}

struct CelestialObject {
    body_type: CelestialBody,
    translation: Vec3,
    rotation: Vec3,
    scale: f32,
    rotation_speed: Vec3,
    orbit_speed: f32,
    orbit_radius: f32,
    orbit_center: Vec3,
    use_large_sphere: bool,
}

impl CelestialObject {
    fn new(body_type: CelestialBody, translation: Vec3, scale: f32, use_large_sphere: bool) -> Self {
        CelestialObject {
            body_type,
            translation,
            rotation: Vec3::new(0.0, 0.0, 0.0),
            scale,
            rotation_speed: Vec3::new(0.0, 0.01, 0.0),
            orbit_speed: 0.0,
            orbit_radius: 0.0,
            orbit_center: Vec3::new(400.0, 300.0, 0.0),
            use_large_sphere,
        }
    }

    fn with_orbit(mut self, radius: f32, speed: f32) -> Self {
        self.orbit_radius = radius;
        self.orbit_speed = speed;
        self
    }

    fn with_rotation_speed(mut self, speed: Vec3) -> Self {
        self.rotation_speed = speed;
        self
    }

    fn update(&mut self, time: f32) {
        // Rotación propia
        self.rotation = self.rotation + self.rotation_speed;

        // Órbita
        if self.orbit_radius > 0.0 {
            let angle = time * self.orbit_speed;
            self.translation.x = self.orbit_center.x + angle.cos() * self.orbit_radius;
            self.translation.z = angle.sin() * self.orbit_radius;
        }
    }
}

fn main() {
    let window_width = 1200;
    let window_height = 800;
    let framebuffer_width = 1200;
    let framebuffer_height = 800;
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

    // Cargar los modelos de esferas (rutas ajustadas a la carpeta `models/` en la raíz del proyecto)
    let sphere_large = Obj::load("models/esfera_grande.obj").expect("Failed to load esfera_grande.obj");
    let sphere_large_vertices = sphere_large.get_vertex_array();
    
    let sphere_small = Obj::load("models/esfera_chica.obj").expect("Failed to load esfera_chica.obj");
    let sphere_small_vertices = sphere_small.get_vertex_array();

    // Crear los cuerpos celestes con distancias orbitales bien separadas
    let mut celestial_objects = vec![
        // Sol (centro) - esfera grande
        CelestialObject::new(CelestialBody::Sun, Vec3::new(600.0, 400.0, 0.0), 80.0, true)
            .with_rotation_speed(Vec3::new(0.0, 0.005, 0.0)),
        
        // Tierra - esfera chica
        CelestialObject::new(CelestialBody::Earth, Vec3::new(600.0, 400.0, 0.0), 28.0, false)
            .with_orbit(250.0, 0.35)
            .with_rotation_speed(Vec3::new(0.0, 0.02, 0.0)),
        
        // Marte - esfera chica (más separado)
        CelestialObject::new(CelestialBody::Mars, Vec3::new(600.0, 400.0, 0.0), 20.0, false)
            .with_orbit(450.0, 0.24)
            .with_rotation_speed(Vec3::new(0.0, 0.02, 0.0)),
        
        // Júpiter - esfera grande (bien separado)
        CelestialObject::new(CelestialBody::Jupiter, Vec3::new(600.0, 400.0, 0.0), 55.0, true)
            .with_orbit(700.0, 0.13)
            .with_rotation_speed(Vec3::new(0.0, 0.03, 0.0)),
        
        // Saturno - esfera grande (el más lejano, muy separado)
        CelestialObject::new(CelestialBody::Saturn, Vec3::new(600.0, 400.0, 0.0), 50.0, true)
            .with_orbit(1000.0, 0.08)
            .with_rotation_speed(Vec3::new(0.0, 0.025, 0.0)),
    ];

    // Luna de la Tierra - esfera chica (muy cerca de la Tierra)
    let mut earth_moon = CelestialObject::new(CelestialBody::Moon, Vec3::new(600.0, 400.0, 0.0), 8.0, false)
        .with_orbit(45.0, 1.2)  // Órbita más pequeña y más rápida
        .with_rotation_speed(Vec3::new(0.0, 0.01, 0.0));

    let mut time = 0.0f32;
    
    // Inicializar cámara - mucho más alejada para ver todo el sistema expandido
    let mut camera = Camera::new(
        Vec3::new(600.0, 600.0, 1600.0),  // posición de la cámara (mucho más alejada y elevada)
        Vec3::new(600.0, 400.0, 0.0)       // mirando al centro (donde está el sol)
    );

    let projection_matrix = create_projection_matrix(window_width as f32, window_height as f32);

    while window.is_open() {
        if window.is_key_down(Key::Escape) {
            break;
        }

        handle_input(&window, &mut camera);

        framebuffer.clear();

        time += 0.016;
        
        let view_matrix = camera.get_view_matrix();

        // Actualizar posiciones
        for obj in celestial_objects.iter_mut() {
            obj.update(time);
        }

        // Actualizar luna de la Tierra
        earth_moon.orbit_center = celestial_objects[1].translation; // La Tierra es el índice 1
        earth_moon.update(time);

        // La posición del Sol es la fuente de luz
        let light_position = celestial_objects[0].translation;
        
        // Renderizar todos los cuerpos
        for celestial_obj in &celestial_objects {
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
            };
            
            // Usar esfera grande o chica según el planeta
            let vertices = if celestial_obj.use_large_sphere {
                &sphere_large_vertices
            } else {
                &sphere_small_vertices
            };
            render(&mut framebuffer, &uniforms, vertices);
        }

        // Renderizar luna
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
        };
        render(&mut framebuffer, &moon_uniforms, &sphere_small_vertices);

        // Renderizar anillos de Saturno
        render_saturn_rings(&mut framebuffer, &celestial_objects[4], time, view_matrix, projection_matrix, light_position, camera.position, &sphere_large_vertices);

        window
            .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }
}

fn render_saturn_rings(
    framebuffer: &mut Framebuffer, 
    saturn: &CelestialObject, 
    time: f32, 
    view_matrix: Mat4,
    projection_matrix: Mat4,
    light_position: Vec3,
    camera_position: Vec3,
    vertex_arrays: &[Vertex]
) {
    // Renderizar anillos como un disco plano
    let ring_scale = saturn.scale * 1.8;
    let ring_translation = Vec3::new(saturn.translation.x, saturn.translation.y, saturn.translation.z);
    let ring_rotation = Vec3::new(PI / 4.0, saturn.rotation.y, 0.0); // Inclinado

    let model_matrix = create_model_matrix(ring_translation, ring_scale, ring_rotation);
    let uniforms = Uniforms {
        model_matrix,
        view_matrix,
        projection_matrix,
        time,
        current_shader: CelestialBody::Ring,
        light_position,
        camera_position,
    };

    // Renderizar con el shader de anillos
    render(framebuffer, &uniforms, vertex_arrays);
}

fn handle_input(window: &Window, camera: &mut Camera) {
    let move_speed = 10.0;
    let rotate_speed = 0.02;
    let zoom_speed = 20.0;
    
    // WASD: mover cámara
    if window.is_key_down(Key::W) {
        camera.move_forward(move_speed);
    }
    if window.is_key_down(Key::S) {
        camera.move_forward(-move_speed);
    }
    if window.is_key_down(Key::A) {
        camera.move_right(-move_speed);
    }
    if window.is_key_down(Key::D) {
        camera.move_right(move_speed);
    }
    
    // Q/E: mover arriba/abajo
    if window.is_key_down(Key::Q) {
        camera.move_up(move_speed);
    }
    if window.is_key_down(Key::E) {
        camera.move_up(-move_speed);
    }
    
    // Flechas: orbitar alrededor del objetivo
    if window.is_key_down(Key::Left) {
        camera.orbit(-rotate_speed, 0.0);
    }
    if window.is_key_down(Key::Right) {
        camera.orbit(rotate_speed, 0.0);
    }
    if window.is_key_down(Key::Up) {
        camera.orbit(0.0, rotate_speed);
    }
    if window.is_key_down(Key::Down) {
        camera.orbit(0.0, -rotate_speed);
    }
    
    // Z/X: zoom
    if window.is_key_down(Key::Z) {
        camera.zoom_in(zoom_speed);
    }
    if window.is_key_down(Key::X) {
        camera.zoom_out(zoom_speed);
    }
}