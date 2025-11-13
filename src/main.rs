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
    detail_level: f32,
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
        
        // No acercarse demasiado
        let distance = (self.position - self.target).magnitude();
        if distance < 80.0 {
            self.position = self.target - direction * 80.0;
        }
    }

    fn zoom_out(&mut self, amount: f32) {
        let direction = (self.target - self.position).normalize();
        let current_distance = (self.position - self.target).magnitude();
        
        // Zoom más rápido cuando está lejos
        let adjusted_amount = if current_distance > 2000.0 {
            amount * 1.5
        } else {
            amount
        };
        
        self.position -= adjusted_amount * direction;
        
        // No alejarse demasiado (aumentado para ver todo el sistema)
        let distance = (self.position - self.target).magnitude();
        if distance > 4000.0 {
            self.position = self.target - direction * 4000.0;
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

// Sistema LOD de 3 niveles para máximo rendimiento
// Retorna: (0=ultra_low, 1=low, 2=high)
fn check_lod(object_position: Vec3, object_radius: f32, camera: &Camera) -> usize {
    // Calcular distancia del objeto a la cámara
    let to_object = object_position - camera.position;
    let distance = to_object.magnitude();
    
    // ULTRA LOW POLY: MUY cerca (12 vértices, 20 triángulos) - MÁXIMO RENDIMIENTO
    if distance < object_radius * 4.0 {
        return 0; // Ultra low poly
    }
    
    // LOW POLY: Cerca-medio (482 vértices, 512 triángulos) - Buen rendimiento
    if distance < object_radius * 12.0 {
        return 1; // Low poly
    }
    
    // HIGH POLY: Lejos (482 vértices, 960 triángulos) - Mejor calidad
    2 // High poly
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
    use rayon::prelude::*;
    
    // Vertex Shader Stage (PARALELO - 2-4x más rápido en multi-core)
    let transformed_vertices: Vec<Vertex> = vertex_array
        .par_iter()
        .map(|vertex| vertex_shader(vertex, uniforms))
        .collect();

    // Primitive Assembly Stage (secuencial - es muy rápido)
    let mut triangles = Vec::new();
    for i in (0..transformed_vertices.len()).step_by(3) {
        if i + 2 < transformed_vertices.len() {
            // Backface culling TEMPRANO (antes de rasterizar)
            let v0 = &transformed_vertices[i].transformed_position;
            let v1 = &transformed_vertices[i + 1].transformed_position;
            let v2 = &transformed_vertices[i + 2].transformed_position;
            
            // Producto cruz en 2D (determina orientación)
            let edge1_x = v1.x - v0.x;
            let edge1_y = v1.y - v0.y;
            let edge2_x = v2.x - v0.x;
            let edge2_y = v2.y - v0.y;
            let cross = edge1_x * edge2_y - edge1_y * edge2_x;
            
            // Si cross <= 0, el triángulo está de espaldas - SALTAR
            if cross > 0.0 {
                triangles.push([
                    transformed_vertices[i].clone(),
                    transformed_vertices[i + 1].clone(),
                    transformed_vertices[i + 2].clone(),
                ]);
            }
        }
    }

    // Rasterización y Fragment Shader (PARALELO con chunks)
    // Procesar triángulos en paralelo y luego escribir al framebuffer
    let fragments: Vec<_> = triangles
        .par_iter()
        .flat_map(|tri| {
            let frags = triangle(&tri[0], &tri[1], &tri[2]);
            frags.into_iter().map(|mut frag| {
                // Aplicar shader
                let shader_color = get_celestial_shader(uniforms.current_shader, &frag, &tri[0], uniforms);
                frag.color = shader_color;
                frag
            }).collect::<Vec<_>>()
        })
        .collect();
    
    // Escribir fragmentos al framebuffer (secuencial para evitar race conditions en z-buffer)
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
    // Supersampling dinámico: factor cambia según la distancia de la cámara
    let mut supersample_factor = 2usize;
    let mut framebuffer_width = window_width * supersample_factor;
    let mut framebuffer_height = window_height * supersample_factor;
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
    // Cargar modelo LOW POLY optimizado (178 vértices, 192 caras)
    let sphere_low = Obj::load("models/Esfera_Low.obj").expect("Failed to load Esfera_Low.obj");
    let sphere_low_vertices = sphere_low.get_vertex_array();    // Crear los cuerpos celestes con distancias orbitales bien separadas
    // TODOS usan esfera_chica (LOW POLY) para MEJOR RENDIMIENTO
    let mut celestial_objects = vec![
        // Sol (centro) - esfera LOW
        CelestialObject::new(CelestialBody::Sun, Vec3::new(600.0, 400.0, 0.0), 80.0, false)
            .with_rotation_speed(Vec3::new(0.0, 0.005, 0.0)),
        
        // Mercurio (Lava Planet) - esfera LOW, muy cerca del sol
        CelestialObject::new(CelestialBody::LavaPlanet, Vec3::new(600.0, 400.0, 0.0), 15.0, false)
            .with_orbit(150.0, 0.47)
            .with_rotation_speed(Vec3::new(0.0, 0.01, 0.0)),
        
        // Tierra - esfera LOW
        CelestialObject::new(CelestialBody::Earth, Vec3::new(600.0, 400.0, 0.0), 28.0, false)
            .with_orbit(250.0, 0.35)
            .with_rotation_speed(Vec3::new(0.0, 0.02, 0.0)),
        
        // Marte - esfera LOW (más separado)
        CelestialObject::new(CelestialBody::Mars, Vec3::new(600.0, 400.0, 0.0), 20.0, false)
            .with_orbit(450.0, 0.24)
            .with_rotation_speed(Vec3::new(0.0, 0.02, 0.0)),
        
        // Júpiter - esfera LOW (bien separado)
        CelestialObject::new(CelestialBody::Jupiter, Vec3::new(600.0, 400.0, 0.0), 55.0, false)
            .with_orbit(700.0, 0.13)
            .with_rotation_speed(Vec3::new(0.0, 0.03, 0.0)),
        
        // Saturno - esfera LOW (el más lejano, muy separado)
        CelestialObject::new(CelestialBody::Saturn, Vec3::new(600.0, 400.0, 0.0), 50.0, false)
            .with_orbit(1000.0, 0.08)
            .with_rotation_speed(Vec3::new(0.0, 0.025, 0.0)),
        
        // Urano (Ice Planet) - esfera LOW, muy lejano
        CelestialObject::new(CelestialBody::IcePlanet, Vec3::new(600.0, 400.0, 0.0), 42.0, false)
            .with_orbit(1300.0, 0.06)
            .with_rotation_speed(Vec3::new(0.0, 0.022, 0.0)),
        
        // Neptuno (Alien Planet) - esfera LOW, el más lejano
        CelestialObject::new(CelestialBody::AlienPlanet, Vec3::new(600.0, 400.0, 0.0), 40.0, false)
            .with_orbit(1600.0, 0.04)
            .with_rotation_speed(Vec3::new(0.0, 0.02, 0.0)),
    ];

    // Luna de la Tierra - esfera chica (SUPER CERCA de la Tierra)
    let mut earth_moon = CelestialObject::new(CelestialBody::Moon, Vec3::new(600.0, 400.0, 0.0), 8.0, false)
        .with_orbit(15.0, 1.2)  // Órbita SUPER cercana (15 unidades) - la luna está bastante cerca
        .with_rotation_speed(Vec3::new(0.0, 0.01, 0.0));

    let mut time = 0.0f32;
    
    // Inicializar cámara - MUCHO más alejada para ver todo el sistema expandido con los planetas exteriores
    let mut camera = Camera::new(
        Vec3::new(600.0, 800.0, 2200.0),  // posición de la cámara (muy alejada y elevada)
        Vec3::new(600.0, 400.0, 0.0)       // mirando al centro (donde está el sol)
    );

    let projection_matrix = create_projection_matrix(window_width as f32, window_height as f32);

    while window.is_open() {
        if window.is_key_down(Key::Escape) {
            break;
        }

        handle_input(&window, &mut camera);

        // Calcular distancia de la cámara al objetivo
        let distance_to_target = (camera.position - camera.target).magnitude();
        
        // Decidir factor de supersampling basado en distancia (con histéresis para evitar parpadeo)
        let desired_supersample = if distance_to_target > 1500.0 {
            2usize  // Lejos: alta calidad
        } else if distance_to_target > 600.0 {
            1usize  // Media distancia: calidad normal
        } else {
            1usize  // Cerca: sin supersampling (rendimiento)
        };

        // Solo cambiar el framebuffer si el factor cambia (para evitar saltos)
        if desired_supersample != supersample_factor {
            supersample_factor = desired_supersample;
            framebuffer_width = window_width * supersample_factor;
            framebuffer_height = window_height * supersample_factor;
            framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);
            framebuffer.set_background_color(0x000011);
        }

        framebuffer.clear();

        time += 0.016;
        
        let view_matrix = camera.get_view_matrix();

        // Actualizar posiciones
        for obj in celestial_objects.iter_mut() {
            obj.update(time);
        }

        // Actualizar luna de la Tierra
    earth_moon.orbit_center = celestial_objects[2].translation; // La Tierra es el índice 2 (después de Sol y Mercurio/Lava)
        earth_moon.update(time);

        // La posición del Sol es la fuente de luz
        let light_position = celestial_objects[0].translation;

        // Nivel de detalle ULTRA AGRESIVO basado en distancia (más cerca = menos detalle para MÁXIMO rendimiento)
        let detail_level = if distance_to_target > 1500.0 {
            1.0  // Lejos: máximo detalle
        } else if distance_to_target > 800.0 {
            0.65 // Media: buen detalle
        } else if distance_to_target > 400.0 {
            0.45 // Cerca: detalle reducido
        } else if distance_to_target > 200.0 {
            0.3  // Muy cerca: bajo detalle
        } else {
            0.15 // ULTRA CERCA: mínimo detalle absoluto para MÁXIMO rendimiento
        };        // Renderizar todos los cuerpos usando Esfera_Low.obj (178 vértices, 192 caras - MÁXIMO rendimiento)
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
                detail_level,
            };
            
            // TODOS usan Esfera_Low.obj (178 vértices, 192 caras) para MÁXIMO rendimiento
            render(&mut framebuffer, &uniforms, &sphere_low_vertices);
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
        };
        // La luna usa Esfera_Low.obj (máximo rendimiento)
        render(&mut framebuffer, &moon_uniforms, &sphere_low_vertices);

        // Renderizar anillos de Saturno (SIEMPRE - sin frustum culling)
        render_saturn_rings(
            &mut framebuffer,
            &celestial_objects[5],
            time,
            view_matrix,
            projection_matrix,
            light_position,
            camera.position,
            detail_level,
            &sphere_low_vertices,
        );

        // Renderizar anillos del planeta Alien (índice 7)
        render_alien_rings(
            &mut framebuffer,
            &celestial_objects[7],
            time,
            view_matrix,
            projection_matrix,
            light_position,
            camera.position,
            detail_level,
            &sphere_low_vertices,
        );

        if supersample_factor > 1 {
            // Aplicar downsampling para anti-aliasing
            let downsampled = downsample_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height, window_width, window_height);
            window
                .update_with_buffer(&downsampled, window_width, window_height)
                .unwrap();
        } else {
            window
                .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
                .unwrap();
        }

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
    detail_level: f32,
    vertex_arrays: &[Vertex],
) {
    // Renderizar anillos grandes y prominentes de Saturno
    let ring_scale = saturn.scale * 2.5; // Anillos más grandes y visibles
    let ring_translation = Vec3::new(saturn.translation.x, saturn.translation.y, saturn.translation.z);
    let ring_rotation = Vec3::new(PI / 4.5, saturn.rotation.y, 0.0); // Inclinación más suave para verse mejor

    let model_matrix = create_model_matrix(ring_translation, ring_scale, ring_rotation);
    let uniforms = Uniforms {
        model_matrix,
        view_matrix,
        projection_matrix,
        time,
        current_shader: CelestialBody::Ring,
        light_position,
        camera_position,
        detail_level,
    };

    // Renderizar con el shader de anillos
    render(framebuffer, &uniforms, vertex_arrays);
}

fn render_alien_rings(
    framebuffer: &mut Framebuffer,
    alien_planet: &CelestialObject,
    time: f32,
    view_matrix: Mat4,
    projection_matrix: Mat4,
    light_position: Vec3,
    camera_position: Vec3,
    detail_level: f32,
    vertex_arrays: &[Vertex],
) {
    // Renderizar anillos ENORMES del planeta alien - MUY visibles y dramáticos
    let ring_scale = alien_planet.scale * 4.0; // Anillos ENORMES (4x el tamaño del planeta!)
    let ring_translation = Vec3::new(alien_planet.translation.x, alien_planet.translation.y, alien_planet.translation.z);
    // Rotación similar a Saturno pero con más inclinación para verse mejor desde cualquier ángulo
    let ring_rotation = Vec3::new(PI / 3.5, alien_planet.rotation.y + time * 0.001, PI / 8.0);

    let model_matrix = create_model_matrix(ring_translation, ring_scale, ring_rotation);
    let uniforms = Uniforms {
        model_matrix,
        view_matrix,
        projection_matrix,
        time,
        current_shader: CelestialBody::Ring, // Usar el shader de anillos (tiene transparencia)
        light_position,
        camera_position,
        detail_level,
    };

    // Renderizar con el shader de anillos
    render(framebuffer, &uniforms, vertex_arrays);
}

// Función para downsample el framebuffer (anti-aliasing)
fn downsample_buffer(high_res_buffer: &[u32], high_width: usize, high_height: usize, 
                     low_width: usize, low_height: usize) -> Vec<u32> {
    let mut low_res_buffer = vec![0u32; low_width * low_height];
    let scale_x = high_width / low_width;
    let scale_y = high_height / low_height;
    
    for y in 0..low_height {
        for x in 0..low_width {
            let mut r_sum = 0u32;
            let mut g_sum = 0u32;
            let mut b_sum = 0u32;
            let mut count = 0u32;
            
            // Promediar los píxeles del área correspondiente
            for dy in 0..scale_y {
                for dx in 0..scale_x {
                    let hx = x * scale_x + dx;
                    let hy = y * scale_y + dy;
                    
                    if hx < high_width && hy < high_height {
                        let pixel = high_res_buffer[hy * high_width + hx];
                        r_sum += (pixel >> 16) & 0xFF;
                        g_sum += (pixel >> 8) & 0xFF;
                        b_sum += pixel & 0xFF;
                        count += 1;
                    }
                }
            }
            
            // Calcular promedio
            if count > 0 {
                let r = (r_sum / count) & 0xFF;
                let g = (g_sum / count) & 0xFF;
                let b = (b_sum / count) & 0xFF;
                low_res_buffer[y * low_width + x] = (r << 16) | (g << 8) | b;
            }
        }
    }
    
    low_res_buffer
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