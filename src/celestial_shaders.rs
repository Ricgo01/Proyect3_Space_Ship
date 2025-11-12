use nalgebra_glm::Vec3;
use crate::color::Color;
use crate::fragment::Fragment;
use crate::vertex::Vertex;
use crate::Uniforms;

// Función auxiliar para ruido pseudo-aleatorio
fn noise(x: f32, y: f32, z: f32) -> f32 {
    let a = (x * 12.9898 + y * 78.233 + z * 45.164).sin() * 43758.5453;
    a.fract()
}

// Función para ruido fractal (Fractal Brownian Motion)
fn fbm(x: f32, y: f32, z: f32, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    
    for _ in 0..octaves {
        value += noise(x * frequency, y * frequency, z * frequency) * amplitude;
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    
    value
}

// Helper para mezclar colores
fn mix_color(c1: Color, c2: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::from_float(
        c1.to_float().0 * (1.0 - t) + c2.to_float().0 * t,
        c1.to_float().1 * (1.0 - t) + c2.to_float().1 * t,
        c1.to_float().2 * (1.0 - t) + c2.to_float().2 * t,
    )
}

// Función auxiliar para iluminación Phong
fn calculate_phong_lighting(
    fragment_pos: Vec3,
    normal: Vec3,
    light_pos: Vec3,
    camera_pos: Vec3,
    base_color: Color,
    ambient_strength: f32,
    diffuse_strength: f32,
    specular_strength: f32,
    shininess: f32
) -> Color {
    // Ambiente
    let ambient = base_color * ambient_strength;
    
    // Difusa
    let light_dir = (light_pos - fragment_pos).normalize();
    let diff = normal.dot(&light_dir).max(0.0);
    let diffuse = base_color * (diff * diffuse_strength);
    
    // Especular (Phong)
    let view_dir = (camera_pos - fragment_pos).normalize();
    let reflect_dir = reflect(-light_dir, normal);
    let spec = reflect_dir.dot(&view_dir).max(0.0).powf(shininess);
    let specular = Color::from_float(1.0, 1.0, 1.0) * (spec * specular_strength);
    
    ambient + diffuse + specular
}

fn reflect(incident: Vec3, normal: Vec3) -> Vec3 {
    incident - normal * 2.0 * incident.dot(&normal)
}

// ============= SOL (ESTRELLA) =============
// Shader con 4 capas: núcleo, fuego interno, fuego externo, corona
// El Sol emite luz, no necesita iluminación externa
pub fn sun_shader(_fragment: &Fragment, vertex: &Vertex, time: f32) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    
    // Distancia desde el centro (0 en centro, 1 en borde)
    let dist_from_center = (pos.x * pos.x + pos.y * pos.y + pos.z * pos.z).sqrt();
    
    // Capa 1: Núcleo brillante
    let core_intensity = (1.0 - (dist_from_center * 1.5)).max(0.0).powf(3.0);
    let core_color = Color::from_float(1.0, 1.0, 0.9);
    
    // Capa 2: Fuego interno (naranjas)
    let fire_noise = fbm(
        pos.x * 3.0 + time * 0.5,
        pos.y * 3.0,
        pos.z * 3.0 + time * 0.3,
        4
    );
    let fire_internal = Color::from_float(1.0, 0.6, 0.1);
    
    // Capa 3: Fuego externo (rojos)
    let fire_external_noise = fbm(
        pos.x * 5.0 - time * 0.3,
        pos.y * 5.0,
        pos.z * 5.0 + time * 0.4,
        3
    );
    let fire_external = Color::from_float(1.0, 0.3, 0.0);
    
    // Capa 4: Corona/prominencias solares
    let corona_noise = fbm(
        pos.x * 2.0 + time * 0.2,
        pos.y * 2.0,
        pos.z * 2.0,
        2
    );
    let edge_intensity = (dist_from_center - 0.8).max(0.0) * 5.0;
    let corona_color = Color::from_float(1.0, 0.8, 0.3);
    
    // Efecto de limb darkening (el Sol es menos brillante en los bordes)
    let view_angle = normal.z.abs();
    let limb_darkening = 0.6 + 0.4 * view_angle;
    
    // Mezclar capas
    let mut final_color = core_color * core_intensity;
    final_color = mix_color(final_color, fire_internal, fire_noise * 0.7);
    final_color = mix_color(final_color, fire_external, fire_external_noise * 0.5);
    final_color = mix_color(final_color, corona_color, corona_noise * edge_intensity);
    
    // Aplicar limb darkening y aumentar brillo general
    final_color * limb_darkening * 2.0
}

// ============= PLANETA ROCOSO (TIPO TIERRA) =============
// Shader con 5 capas: océanos, continentes, vegetación, nubes, atmósfera
pub fn earth_like_shader(fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Capa 1: Océanos (azul profundo)
    let ocean_base = Color::from_float(0.05, 0.15, 0.4);
    
    // Capa 2: Continentes
    let land_noise = fbm(pos.x * 2.0, pos.y * 2.0, pos.z * 2.0, 4);
    let is_land = land_noise > 0.4;
    
    let continent_color = Color::from_float(0.3, 0.5, 0.2); // Verde
    
    // Capa 3: Características del terreno (desiertos, montañas)
    let terrain_noise = fbm(pos.x * 5.0, pos.y * 5.0, pos.z * 5.0, 3);
    let mountain_color = Color::from_float(0.4, 0.4, 0.35);
    let desert_color = Color::from_float(0.7, 0.6, 0.3);
    
    // Determinar el color base según el terreno
    let mut base_color = if is_land {
        if terrain_noise > 0.7 {
            mountain_color
        } else if terrain_noise < 0.3 {
            desert_color
        } else {
            continent_color
        }
    } else {
        ocean_base
    };
    
    // Aplicar iluminación Phong realista
    let specular = if !is_land { 0.6 } else { 0.1 }; // El agua refleja más
    let shininess = if !is_land { 32.0 } else { 8.0 };
    
    base_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        base_color,
        0.35,  // ambiente (aumentado para ver todo mejor)
        0.7,   // difusa
        specular,
        shininess
    );
    
    // Capa 4: Nubes con iluminación
    let cloud_noise = fbm(
        pos.x * 4.0 + uniforms.time * 0.05,
        pos.y * 4.0,
        pos.z * 4.0 - uniforms.time * 0.03,
        3
    );
    let cloud_intensity = (cloud_noise - 0.5).max(0.0) * 2.0;
    
    // Iluminar las nubes
    let light_dir = (uniforms.light_position - fragment_pos).normalize();
    let cloud_lighting = normal.dot(&light_dir).max(0.0);
    let cloud_color = Color::from_float(1.0, 1.0, 1.0) * (0.4 + cloud_lighting * 0.6);
    
    base_color = mix_color(base_color, cloud_color, cloud_intensity * 0.7);
    
    // Capa 5: Atmósfera (brillo en los bordes)
    let view_dir = (uniforms.camera_position - fragment_pos).normalize();
    let fresnel = (1.0 - normal.dot(&view_dir).abs()).powf(3.0);
    let atmosphere_color = Color::from_float(0.3, 0.5, 0.9);
    
    mix_color(base_color, atmosphere_color, fresnel * 0.4)
}

// ============= GIGANTE GASEOSO (TIPO JÚPITER) =============
// Shader con 4 capas: bandas base, bandas secundarias, tormentas, brillo
pub fn gas_giant_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Capa 1: Bandas atmosféricas principales
    let band_pattern = (pos.y * 8.0 + fbm(pos.x * 2.0, pos.y * 2.0, pos.z * 2.0, 2) * 0.5).sin();
    let band_color1 = Color::from_float(0.85, 0.65, 0.45);
    let band_color2 = Color::from_float(0.65, 0.45, 0.3);
    let band_mix = (band_pattern + 1.0) / 2.0;
    
    // Capa 2: Turbulencias en las bandas
    let turbulence = fbm(
        pos.x * 6.0 + uniforms.time * 0.02,
        pos.y * 3.0,
        pos.z * 6.0,
        4
    );
    let turb_color = Color::from_float(0.8, 0.6, 0.4);
    
    // Capa 3: Gran Mancha Roja (tormenta)
    let storm_center = Vec3::new(0.3, 0.2, 0.5);
    let dist_to_storm = ((pos.x - storm_center.x).powi(2) + 
                         (pos.y - storm_center.y).powi(2) + 
                         (pos.z - storm_center.z).powi(2)).sqrt();
    let storm_intensity = (1.0 - dist_to_storm * 3.0).max(0.0).powf(2.0);
    let storm_noise = fbm(pos.x * 8.0 + uniforms.time * 0.1, pos.y * 8.0, pos.z * 8.0, 2);
    let storm_color = Color::from_float(0.75, 0.25, 0.15);
    
    // Combinar capas base
    let mut base_color = mix_color(band_color1, band_color2, band_mix);
    base_color = mix_color(base_color, turb_color, turbulence * 0.3);
    base_color = mix_color(base_color, storm_color, storm_intensity * storm_noise);
    
    // Aplicar iluminación Phong
    base_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        base_color,
        0.35,  // ambiente (aumentado)
        0.7,   // difusa
        0.15,  // especular
        4.0    // shininess
    );
    
    // Capa 4: Brillo atmosférico
    let view_dir = (uniforms.camera_position - fragment_pos).normalize();
    let glow = (1.0 - normal.dot(&view_dir).abs()).powf(2.5);
    let glow_color = Color::from_float(0.9, 0.7, 0.5);
    
    mix_color(base_color, glow_color, glow * 0.25)
}

// ============= PLANETA ROCOSO (TIPO MARTE) =============
// Shader con 4 capas: superficie base, cráteres, polos de hielo, polvo atmosférico
pub fn mars_like_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Capa 1: Superficie oxidada (rojo/naranja)
    let base_noise = fbm(pos.x * 3.0, pos.y * 3.0, pos.z * 3.0, 3);
    let rust_color1 = Color::from_float(0.8, 0.3, 0.1);
    let rust_color2 = Color::from_float(0.6, 0.25, 0.15);
    
    // Capa 2: Cráteres
    let crater_noise = fbm(pos.x * 8.0, pos.y * 8.0, pos.z * 8.0, 2);
    let crater_intensity = (crater_noise - 0.6).max(0.0);
    let crater_color = Color::from_float(0.3, 0.15, 0.1);
    
    // Capa 3: Polos de hielo (CO2)
    let pole_intensity = (pos.y.abs() - 0.7).max(0.0) * 5.0;
    let ice_noise = fbm(pos.x * 10.0, pos.y * 10.0, pos.z * 10.0, 2);
    let ice_color = Color::from_float(0.9, 0.95, 1.0);
    
    // Combinar capas
    let mut base_color = mix_color(rust_color1, rust_color2, base_noise);
    base_color = mix_color(base_color, crater_color, crater_intensity * 0.5);
    base_color = mix_color(base_color, ice_color, pole_intensity * ice_noise);
    
    // Aplicar iluminación Phong
    base_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        base_color,
        0.35,  // ambiente (aumentado)
        0.7,   // difusa
        0.1,   // especular (superficie mate)
        8.0    // shininess
    );
    
    // Capa 4: Atmósfera tenue (rosada)
    let view_dir = (uniforms.camera_position - fragment_pos).normalize();
    let atmosphere = (1.0 - normal.dot(&view_dir).abs()).powf(4.0);
    let atm_color = Color::from_float(0.9, 0.6, 0.4);
    
    mix_color(base_color, atm_color, atmosphere * 0.15)
}

// ============= GIGANTE GASEOSO CON ANILLOS (TIPO SATURNO) =============
pub fn saturn_like_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Bandas suaves (colores pastel)
    let band_pattern = (pos.y * 6.0 + fbm(pos.x * 1.5, pos.y * 1.5, pos.z * 1.5, 2) * 0.3).sin();
    let band_color1 = Color::from_float(0.95, 0.9, 0.7);
    let band_color2 = Color::from_float(0.85, 0.8, 0.6);
    let band_mix = (band_pattern + 1.0) / 2.0;
    
    let mut base_color = mix_color(band_color1, band_color2, band_mix);
    
    // Aplicar iluminación Phong
    base_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        base_color,
        0.35,  // ambiente (aumentado)
        0.7,   // difusa
        0.12,  // especular (gas suave)
        4.0    // shininess
    );
    
    base_color
}

// ============= ANILLOS =============
pub fn ring_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Distancia radial desde el centro (en el plano XZ)
    let radial_dist = (pos.x * pos.x + pos.z * pos.z).sqrt();
    
    // Crear bandas en los anillos
    let band_pattern = (radial_dist * 30.0 + fbm(pos.x * 10.0, 0.0, pos.z * 10.0, 2) * 2.0).sin();
    
    // Colores de los anillos
    let ring_color1 = Color::from_float(0.9, 0.85, 0.7);
    let ring_color2 = Color::from_float(0.7, 0.65, 0.5);
    let ring_color3 = Color::from_float(0.5, 0.45, 0.35);
    
    let band_value = (band_pattern + 1.0) / 2.0;
    
    let final_color = if band_value > 0.66 {
        ring_color1
    } else if band_value > 0.33 {
        ring_color2
    } else {
        ring_color3
    };
    
    // Agregar variación y transparencia
    let noise_var = fbm(pos.x * 15.0, pos.y * 15.0, pos.z * 15.0, 3);
    let mut base_color = mix_color(final_color, Color::from_float(0.8, 0.75, 0.6), noise_var * 0.3);
    
    // Aplicar iluminación Phong
    base_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        base_color,
        0.35,  // ambiente (aumentado)
        0.65,  // difusa
        0.15,  // especular
        6.0    // shininess
    );
    
    base_color
}

// ============= LUNA =============
// Shader con 4 capas: superficie base, cráteres, mares, brillo
pub fn moon_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Capa 1: Superficie lunar (gris)
    let base_color = Color::from_float(0.6, 0.6, 0.65);
    
    // Capa 2: Cráteres
    let crater_noise = fbm(pos.x * 10.0, pos.y * 10.0, pos.z * 10.0, 4);
    let crater_intensity = (crater_noise - 0.5).max(0.0);
    let crater_color = Color::from_float(0.3, 0.3, 0.32);
    
    // Capa 3: Mares lunares (zonas más oscuras)
    let maria_noise = fbm(pos.x * 2.0, pos.y * 2.0, pos.z * 2.0, 2);
    let is_maria = maria_noise > 0.55;
    let maria_color = Color::from_float(0.35, 0.35, 0.38);
    
    // Capa 4: Variación de textura
    let texture_noise = fbm(pos.x * 20.0, pos.y * 20.0, pos.z * 20.0, 2);
    
    // Combinar capas
    let mut final_color = if is_maria { maria_color } else { base_color };
    final_color = mix_color(final_color, crater_color, crater_intensity * 0.7);
    
    // Aplicar iluminación Phong
    final_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        final_color,
        0.35,  // ambiente (aumentado)
        0.8,   // difusa
        0.05,  // especular (superficie rocosa)
        2.0    // shininess
    );
    
    // Añadir variación sutil
    final_color * (0.95 + texture_noise * 0.1)
}

// Enum para identificar el tipo de shader
#[derive(Clone, Copy, PartialEq)]
pub enum CelestialBody {
    Sun,
    Earth,
    Jupiter,
    Mars,
    Saturn,
    Ring,
    Moon,
}

pub fn get_celestial_shader(body: CelestialBody, fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    match body {
        CelestialBody::Sun => sun_shader(fragment, vertex, uniforms.time),
        CelestialBody::Earth => earth_like_shader(fragment, vertex, uniforms),
        CelestialBody::Jupiter => gas_giant_shader(fragment, vertex, uniforms),
        CelestialBody::Mars => mars_like_shader(fragment, vertex, uniforms),
        CelestialBody::Saturn => saturn_like_shader(fragment, vertex, uniforms),
        CelestialBody::Ring => ring_shader(fragment, vertex, uniforms),
        CelestialBody::Moon => moon_shader(fragment, vertex, uniforms),
    }
}
