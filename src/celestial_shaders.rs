use nalgebra_glm::Vec3;
use crate::color::Color;
use crate::fragment::Fragment;
use crate::vertex::Vertex;
use crate::Uniforms;

// ============= FUNCIONES DE NOISE =============

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

// Worley/Cellular noise para efectos de células
fn worley_noise(x: f32, y: f32, z: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let zi = z.floor() as i32;
    
    let mut min_dist: f32 = 100.0; // Fix: especificar tipo explícitamente
    
    for i in -1..=1 {
        for j in -1..=1 {
            for k in -1..=1 {
                let cell_x = (xi + i) as f32;
                let cell_y = (yi + j) as f32;
                let cell_z = (zi + k) as f32;
                
                let point_x = cell_x + noise(cell_x, cell_y, cell_z);
                let point_y = cell_y + noise(cell_x + 1.0, cell_y, cell_z);
                let point_z = cell_z + noise(cell_x, cell_y + 1.0, cell_z);
                
                let dx = point_x - x;
                let dy = point_y - y;
                let dz = point_z - z;
                let dist = (dx*dx + dy*dy + dz*dz).sqrt();
                
                min_dist = min_dist.min(dist);
            }
        }
    }
    
    min_dist
}

// Turbulencia para efectos caóticos
fn turbulence(x: f32, y: f32, z: f32, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    
    for _ in 0..octaves {
        value += (fbm(x * frequency, y * frequency, z * frequency, 2) - 0.5).abs() * amplitude;
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
// Shader con 5+ capas: núcleo, plasma, manchas solares, llamaradas, corona
pub fn sun_shader(_fragment: &Fragment, vertex: &Vertex, time: f32) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    
    let dist_from_center = (pos.x * pos.x + pos.y * pos.y + pos.z * pos.z).sqrt();
    
    // Capa 1: Núcleo ultra brillante con pulsación
    let pulse = (time * 2.0).sin() * 0.15 + 1.0;
    let core_intensity = (1.0 - (dist_from_center * 1.8)).max(0.0).powf(4.0) * pulse;
    let core_color = Color::from_float(1.0, 1.0, 0.95);
    
    // Capa 2: Plasma interno con movimiento caótico
    let plasma_noise = turbulence(
        pos.x * 4.0 + time * 0.4,
        pos.y * 4.0 + (time * 0.3).sin() * 0.2,
        pos.z * 4.0 + time * 0.35,
        5
    );
    let plasma_color = Color::from_float(1.0, 0.7, 0.0);
    
    // Capa 3: Manchas solares (áreas más oscuras)
    let sunspot_noise = worley_noise(pos.x * 3.0, pos.y * 3.0, pos.z * 3.0);
    let sunspot_intensity = (sunspot_noise - 0.3).max(0.0).min(0.5);
    let sunspot_color = Color::from_float(0.6, 0.2, 0.0);
    
    // Capa 4: Llamaradas solares en los bordes
    let flare_noise = fbm(
        pos.x * 6.0 - time * 0.5,
        pos.y * 6.0,
        pos.z * 6.0 + time * 0.4,
        4
    );
    let edge_dist = (dist_from_center - 0.75).max(0.0);
    let flare_intensity = edge_dist * flare_noise * 8.0;
    let flare_color = Color::from_float(1.0, 0.4, 0.0);
    
    // Capa 5: Corona brillante con partículas
    let corona_noise = fbm(
        pos.x * 2.5 + time * 0.15,
        pos.y * 2.5,
        pos.z * 2.5 - time * 0.1,
        3
    );
    let corona_intensity = (dist_from_center - 0.85).max(0.0) * 6.0;
    let corona_color = Color::from_float(1.0, 0.9, 0.5);
    
    // Limb darkening mejorado
    let view_angle = normal.z.abs();
    let limb_darkening = 0.5 + 0.5 * view_angle.powf(0.7);
    
    // Mezclar todas las capas
    let mut final_color = core_color * core_intensity;
    final_color = mix_color(final_color, plasma_color, plasma_noise * 0.8);
    final_color = mix_color(final_color, sunspot_color, sunspot_intensity);
    final_color = mix_color(final_color, flare_color, flare_intensity.min(1.0));
    final_color = mix_color(final_color, corona_color, corona_noise * corona_intensity);
    
    final_color * limb_darkening * 2.5
}

// ============= PLANETA ROCOSO (TIPO TIERRA) =============
// Shader con 6 capas: océanos, continentes, biomas, casquetes polares, nubes, atmósfera
pub fn earth_like_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Capa 1: Océanos con profundidad variable
    let ocean_depth = fbm(pos.x * 4.0, pos.y * 4.0, pos.z * 4.0, 3);
    let deep_ocean = Color::from_float(0.02, 0.08, 0.25);
    let shallow_ocean = Color::from_float(0.1, 0.3, 0.5);
    
    // Capa 2: Continentes con Worley noise para forma realista
    let land_noise = worley_noise(pos.x * 1.5, pos.y * 1.5, pos.z * 1.5);
    let land_detail = fbm(pos.x * 3.0, pos.y * 3.0, pos.z * 3.0, 4);
    let is_land = land_noise > 0.45 || land_detail > 0.6;
    
    // Capa 3: Biomas terrestres
    let biome_noise = fbm(pos.x * 2.5, pos.y * 2.5, pos.z * 2.5, 3);
    let altitude = fbm(pos.x * 5.0, pos.y * 5.0, pos.z * 5.0, 2);
    
    let forest = Color::from_float(0.15, 0.4, 0.15);
    let plains = Color::from_float(0.4, 0.5, 0.2);
    let desert = Color::from_float(0.75, 0.65, 0.35);
    let mountain = Color::from_float(0.5, 0.5, 0.5);
    let snow = Color::from_float(0.9, 0.9, 0.95);
    
    let mut base_color = if is_land {
        if altitude > 0.75 {
            if pos.y.abs() > 0.6 || altitude > 0.85 {
                snow
            } else {
                mountain
            }
        } else if biome_noise > 0.6 {
            desert
        } else if biome_noise > 0.4 {
            plains
        } else {
            forest
        }
    } else {
        mix_color(deep_ocean, shallow_ocean, ocean_depth)
    };
    
    // Capa 4: Casquetes polares
    let pole_intensity = (pos.y.abs() - 0.65).max(0.0) * 8.0;
    let ice_noise = fbm(pos.x * 8.0, pos.y * 8.0, pos.z * 8.0, 2);
    base_color = mix_color(base_color, snow, (pole_intensity * ice_noise).min(1.0));
    
    // Aplicar iluminación Phong
    let specular = if !is_land { 0.8 } else { 0.05 };
    let shininess = if !is_land { 64.0 } else { 4.0 };
    
    base_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        base_color,
        0.25,
        0.8,
        specular,
        shininess
    );
    
    // Capa 5: Sistema de nubes mejorado
    let cloud_layer1 = fbm(
        pos.x * 5.0 + uniforms.time * 0.08,
        pos.y * 5.0,
        pos.z * 5.0 - uniforms.time * 0.05,
        4
    );
    let cloud_layer2 = fbm(
        pos.x * 8.0 - uniforms.time * 0.06,
        pos.y * 8.0,
        pos.z * 8.0,
        3
    );
    let cloud_intensity = ((cloud_layer1 * 0.6 + cloud_layer2 * 0.4) - 0.5).max(0.0) * 2.5;
    
    let light_dir = (uniforms.light_position - fragment_pos).normalize();
    let cloud_lighting = normal.dot(&light_dir).max(0.0) * 0.7 + 0.3; // Fix: remover paréntesis innecesarios
    let cloud_color = Color::from_float(1.0, 1.0, 1.0) * cloud_lighting;
    
    base_color = mix_color(base_color, cloud_color, cloud_intensity.min(0.75));
    
    // Capa 6: Atmósfera con dispersión Rayleigh
    let view_dir = (uniforms.camera_position - fragment_pos).normalize();
    let fresnel = (1.0 - normal.dot(&view_dir).abs()).powf(3.5);
    let atmosphere_color = Color::from_float(0.4, 0.6, 1.0);
    
    mix_color(base_color, atmosphere_color, fresnel * 0.5)
}

// ============= GIGANTE GASEOSO (TIPO JÚPITER) =============
// Shader con 5 capas: bandas, turbulencias, gran mancha roja, tormentas secundarias, brillo
pub fn gas_giant_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Capa 1: Bandas atmosféricas principales con más detalle
    let band_freq = 10.0;
    let band_distortion = turbulence(pos.x * 3.0, pos.y * 2.0, pos.z * 3.0, 3) * 0.8;
    let band_pattern = ((pos.y + band_distortion) * band_freq).sin();
    
    let band_color1 = Color::from_float(0.9, 0.7, 0.5);
    let band_color2 = Color::from_float(0.7, 0.5, 0.3);
    let band_color3 = Color::from_float(0.85, 0.65, 0.45);
    let band_mix = (band_pattern + 1.0) / 2.0;
    
    let mut base_color = if band_mix > 0.66 {
        band_color1
    } else if band_mix > 0.33 {
        mix_color(band_color2, band_color3, (band_mix - 0.33) * 3.0)
    } else {
        band_color2
    };
    
    // Capa 2: Turbulencias y vórtices
    let vortex_noise = turbulence(
        pos.x * 8.0 + uniforms.time * 0.03,
        pos.y * 4.0,
        pos.z * 8.0 - uniforms.time * 0.02,
        5
    );
    let vortex_color = Color::from_float(0.75, 0.55, 0.35);
    base_color = mix_color(base_color, vortex_color, vortex_noise * 0.4);
    
    // Capa 3: Gran Mancha Roja (más grande y dinámica)
    let storm_center = Vec3::new(0.25, -0.15, 0.6);
    let dx = pos.x - storm_center.x;
    let dy = (pos.y - storm_center.y) * 2.0;
    let dz = pos.z - storm_center.z;
    let dist_to_storm = (dx*dx + dy*dy + dz*dz).sqrt();
    
    let storm_radius = 0.35;
    let storm_intensity = (1.0 - (dist_to_storm / storm_radius)).max(0.0).powf(1.5);
    let storm_swirl = turbulence(
        pos.x * 12.0 + uniforms.time * 0.15,
        pos.y * 12.0,
        pos.z * 12.0 - uniforms.time * 0.12,
        4
    );
    let storm_color = Color::from_float(0.8, 0.2, 0.1);
    base_color = mix_color(base_color, storm_color, storm_intensity * storm_swirl);
    
    // Capa 4: Tormentas secundarias
    let small_storm1 = Vec3::new(-0.3, 0.3, 0.5);
    let dist1 = ((pos - small_storm1).magnitude() * 6.0 - 1.0).max(0.0).min(1.0);
    let storm_color2 = Color::from_float(0.9, 0.6, 0.4);
    base_color = mix_color(base_color, storm_color2, (1.0 - dist1) * 0.3);
    
    // Aplicar iluminación Phong
    base_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        base_color,
        0.3,
        0.75,
        0.2,
        8.0
    );
    
    // Capa 5: Brillo atmosférico mejorado
    let view_dir = (uniforms.camera_position - fragment_pos).normalize();
    let glow = (1.0 - normal.dot(&view_dir).abs()).powf(2.0);
    let glow_color = Color::from_float(0.95, 0.75, 0.55);
    
    mix_color(base_color, glow_color, glow * 0.3)
}

// ============= PLANETA ROCOSO (TIPO MARTE) =============
// Shader con 4 capas: superficie oxidada, cráteres, polos de hielo, atmósfera
pub fn mars_like_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Capa 1: Superficie oxidada con variación
    let base_noise = fbm(pos.x * 3.0, pos.y * 3.0, pos.z * 3.0, 4);
    let rust_color1 = Color::from_float(0.8, 0.3, 0.1);
    let rust_color2 = Color::from_float(0.6, 0.25, 0.15);
    let rust_color3 = Color::from_float(0.7, 0.35, 0.2);
    
    let mut base_color = if base_noise > 0.6 {
        rust_color1
    } else if base_noise > 0.3 {
        rust_color3
    } else {
        rust_color2
    };
    
    // Capa 2: Cráteres con profundidad
    let crater_noise = worley_noise(pos.x * 5.0, pos.y * 5.0, pos.z * 5.0);
    let crater_depth = fbm(pos.x * 12.0, pos.y * 12.0, pos.z * 12.0, 2);
    let crater_intensity = ((crater_noise - 0.4).max(0.0) * crater_depth).min(1.0);
    let crater_color = Color::from_float(0.3, 0.15, 0.1);
    base_color = mix_color(base_color, crater_color, crater_intensity * 0.6);
    
    // Capa 3: Polos de hielo (CO2)
    let pole_intensity = (pos.y.abs() - 0.65).max(0.0) * 6.0;
    let ice_noise = fbm(pos.x * 10.0, pos.y * 10.0, pos.z * 10.0, 3);
    let ice_color = Color::from_float(0.9, 0.95, 1.0);
    base_color = mix_color(base_color, ice_color, (pole_intensity * ice_noise).min(1.0));
    
    // Aplicar iluminación Phong
    base_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        base_color,
        0.3,
        0.75,
        0.08,
        4.0
    );
    
    // Capa 4: Atmósfera tenue con tormentas de polvo
    let view_dir = (uniforms.camera_position - fragment_pos).normalize();
    let atmosphere = (1.0 - normal.dot(&view_dir).abs()).powf(4.0);
    let dust_storm = fbm(pos.x * 4.0 + uniforms.time * 0.1, pos.y * 4.0, pos.z * 4.0, 2);
    let atm_color = mix_color(
        Color::from_float(0.9, 0.6, 0.4),
        Color::from_float(0.8, 0.5, 0.3),
        dust_storm
    );
    
    mix_color(base_color, atm_color, atmosphere * 0.2)
}

// ============= GIGANTE GASEOSO CON ANILLOS (TIPO SATURNO) =============
// Shader con 4 capas: bandas suaves, turbulencias sutiles, hexágono polar, brillo
pub fn saturn_like_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Capa 1: Bandas suaves (colores pastel)
    let band_pattern = (pos.y * 8.0 + fbm(pos.x * 2.0, pos.y * 2.0, pos.z * 2.0, 3) * 0.4).sin();
    let band_color1 = Color::from_float(0.95, 0.9, 0.7);
    let band_color2 = Color::from_float(0.88, 0.83, 0.65);
    let band_color3 = Color::from_float(0.92, 0.87, 0.68);
    let band_mix = (band_pattern + 1.0) / 2.0;
    
    let mut base_color = if band_mix > 0.66 {
        band_color1
    } else if band_mix > 0.33 {
        band_color3
    } else {
        band_color2
    };
    
    // Capa 2: Turbulencias sutiles
    let turb = fbm(
        pos.x * 5.0 + uniforms.time * 0.02,
        pos.y * 3.0,
        pos.z * 5.0,
        3
    );
    let turb_color = Color::from_float(0.9, 0.85, 0.67);
    base_color = mix_color(base_color, turb_color, turb * 0.2);
    
    // Capa 3: Hexágono en polo norte (característica real de Saturno)
    if pos.y > 0.7 {
        let angle = pos.x.atan2(pos.z);
        let hex_pattern = (angle * 3.0).cos();
        let hex_intensity = (pos.y - 0.7) * 5.0;
        let hex_color = Color::from_float(0.85, 0.8, 0.6);
        base_color = mix_color(base_color, hex_color, hex_pattern * hex_intensity * 0.3);
    }
    
    // Aplicar iluminación Phong
    base_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        base_color,
        0.35,
        0.7,
        0.15,
        6.0
    );
    
    // Capa 4: Brillo atmosférico suave
    let view_dir = (uniforms.camera_position - fragment_pos).normalize();
    let glow = (1.0 - normal.dot(&view_dir).abs()).powf(2.5);
    let glow_color = Color::from_float(0.98, 0.93, 0.75);
    
    mix_color(base_color, glow_color, glow * 0.2)
}

// ============= ANILLOS MEJORADOS =============
// Shader con 4 capas: bandas principales, gaps, partículas, sombras
pub fn ring_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Distancia radial desde el centro (en el plano XZ)
    let radial_dist = (pos.x * pos.x + pos.z * pos.z).sqrt();
    
    // Capa 1: Bandas principales con divisiones (Cassini Division)
    let band_pattern = (radial_dist * 40.0).sin();
    let gap_pattern = ((radial_dist - 2.5).abs() * 50.0).cos(); // Gap de Cassini
    
    // Colores de los anillos
    let ring_color1 = Color::from_float(0.95, 0.9, 0.75);
    let ring_color2 = Color::from_float(0.85, 0.8, 0.65);
    let ring_color3 = Color::from_float(0.75, 0.7, 0.6);
    let gap_color = Color::from_float(0.3, 0.28, 0.25);
    
    let band_value = (band_pattern + 1.0) / 2.0;
    
    let mut base_color = if band_value > 0.7 {
        ring_color1
    } else if band_value > 0.4 {
        ring_color2
    } else {
        ring_color3
    };
    
    // Aplicar gaps (divisiones oscuras)
    if gap_pattern > 0.5 {
        base_color = mix_color(base_color, gap_color, 0.7);
    }
    
    // Capa 2: Partículas y textura granular
    let particle_noise = fbm(
        pos.x * 40.0 + uniforms.time * 0.05,
        pos.y * 40.0,
        pos.z * 40.0 - uniforms.time * 0.03,
        4
    );
    let particle_color = Color::from_float(0.9, 0.85, 0.7);
    base_color = mix_color(base_color, particle_color, particle_noise * 0.25);
    
    // Capa 3: Variación radial de densidad
    let density = (radial_dist * 15.0).sin() * 0.5 + 0.5;
    base_color = base_color * (0.7 + density * 0.3);
    
    // Aplicar iluminación Phong
    base_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        base_color,
        0.3,
        0.7,
        0.25,
        8.0
    );
    
    // Capa 4: Efecto de translucidez cuando el sol está detrás
    let light_dir = (uniforms.light_position - fragment_pos).normalize();
    let backlight = (-normal.dot(&light_dir)).max(0.0);
    let glow_color = Color::from_float(1.0, 0.95, 0.85);
    
    mix_color(base_color, glow_color, backlight * 0.3)
}

// ============= LUNA =============
// Shader con 4 capas: superficie, cráteres, mares, rayos de eyección
pub fn moon_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Capa 1: Superficie lunar (gris con variación)
    let surface_noise = fbm(pos.x * 5.0, pos.y * 5.0, pos.z * 5.0, 3);
    let base_gray = Color::from_float(0.6, 0.6, 0.65);
    let light_gray = Color::from_float(0.7, 0.7, 0.72);
    let mut base_color = mix_color(base_gray, light_gray, surface_noise);
    
    // Capa 2: Cráteres con Worley noise
    let crater_noise = worley_noise(pos.x * 6.0, pos.y * 6.0, pos.z * 6.0);
    let crater_detail = fbm(pos.x * 15.0, pos.y * 15.0, pos.z * 15.0, 2);
    let crater_intensity = ((crater_noise - 0.3).max(0.0) * crater_detail).min(1.0);
    let crater_color = Color::from_float(0.3, 0.3, 0.32);
    base_color = mix_color(base_color, crater_color, crater_intensity * 0.8);
    
    // Capa 3: Mares lunares (zonas basálticas más oscuras)
    let maria_noise = fbm(pos.x * 2.0, pos.y * 2.0, pos.z * 2.0, 3);
    let is_maria = maria_noise > 0.6;
    let maria_color = Color::from_float(0.35, 0.35, 0.38);
    if is_maria {
        base_color = mix_color(base_color, maria_color, 0.7);
    }
    
    // Capa 4: Rayos de eyección (líneas brillantes desde cráteres)
    let ray_pattern = fbm(
        pos.x * 20.0 + pos.y * 5.0,
        pos.y * 20.0,
        pos.z * 20.0 + pos.x * 5.0,
        2
    );
    if crater_intensity > 0.6 && ray_pattern > 0.7 {
        let ray_color = Color::from_float(0.8, 0.8, 0.82);
        base_color = mix_color(base_color, ray_color, 0.4);
    }
    
    // Aplicar iluminación Phong
    base_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        base_color,
        0.2,
        0.85,
        0.03,
        2.0
    );
    
    base_color
}

// ============= PLANETAS EXTRAS PARA BONIFICACIÓN =============

// PLANETA DE LAVA VOLCÁNICO - 4 capas
pub fn lava_planet_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Capa 1: Corteza oscura (roca volcánica)
    let crust_noise = fbm(pos.x * 4.0, pos.y * 4.0, pos.z * 4.0, 3);
    let dark_crust = Color::from_float(0.15, 0.1, 0.08);
    let light_crust = Color::from_float(0.25, 0.2, 0.15);
    
    // Capa 2: Grietas de lava (patrón de Worley para grietas)
    let crack_pattern = worley_noise(pos.x * 8.0, pos.y * 8.0, pos.z * 8.0);
    let is_crack = crack_pattern < 0.35;
    
    // Capa 3: Lava brillante animada
    let lava_flow = fbm(
        pos.x * 6.0 + uniforms.time * 0.3,
        pos.y * 6.0,
        pos.z * 6.0 - uniforms.time * 0.25,
        4
    );
    let lava_intensity = (lava_flow * 1.5).min(1.0);
    
    // Colores de lava (de oscuro a brillante)
    let lava_dark = Color::from_float(0.8, 0.2, 0.0);
    let lava_bright = Color::from_float(1.0, 0.6, 0.1);
    let lava_white = Color::from_float(1.0, 0.9, 0.5);
    
    let mut base_color = if is_crack {
        if lava_intensity > 0.8 {
            lava_white
        } else if lava_intensity > 0.5 {
            lava_bright
        } else {
            lava_dark
        }
    } else {
        mix_color(dark_crust, light_crust, crust_noise)
    };
    
    // Aplicar iluminación (la lava emite luz)
    if is_crack {
        base_color = base_color * (1.5 + lava_intensity * 0.5);
    } else {
        base_color = calculate_phong_lighting(
            fragment_pos,
            normal,
            uniforms.light_position,
            uniforms.camera_position,
            base_color,
            0.2,
            0.6,
            0.1,
            4.0
        );
    }
    
    // Capa 4: Atmósfera volcánica (ceniza y gases)
    let view_dir = (uniforms.camera_position - fragment_pos).normalize();
    let atmosphere = (1.0 - normal.dot(&view_dir).abs()).powf(3.0);
    let smoke_color = Color::from_float(0.4, 0.25, 0.15);
    
    mix_color(base_color, smoke_color, atmosphere * 0.4)
}

// PLANETA DE HIELO/CRISTAL - 5 capas
pub fn ice_planet_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Capa 1: Hielo base (azul cristalino)
    let ice_noise = fbm(pos.x * 3.0, pos.y * 3.0, pos.z * 3.0, 4);
    let ice_base = Color::from_float(0.7, 0.85, 0.95);
    let ice_deep = Color::from_float(0.5, 0.7, 0.9);
    let mut base_color = mix_color(ice_deep, ice_base, ice_noise);
    
    // Capa 2: Fracturas cristalinas
    let fracture_pattern = worley_noise(pos.x * 5.0, pos.y * 5.0, pos.z * 5.0);
    let is_fracture = fracture_pattern < 0.25;
    let fracture_color = Color::from_float(0.3, 0.5, 0.7);
    if is_fracture {
        base_color = mix_color(base_color, fracture_color, 0.6);
    }
    
    // Capa 3: Cristales de hielo (brillo prismático)
    let crystal_noise = fbm(pos.x * 12.0, pos.y * 12.0, pos.z * 12.0, 2);
    let crystal_sparkle = (crystal_noise - 0.7).max(0.0) * 5.0;
    let sparkle_color = Color::from_float(0.9, 0.95, 1.0);
    base_color = mix_color(base_color, sparkle_color, crystal_sparkle.min(1.0) * 0.5);
    
    // Capa 4: Auroras congeladas (bandas de color)
    let aurora_pattern = ((pos.y * 8.0 + pos.x * 2.0) + 
                          fbm(pos.x * 4.0, pos.y * 4.0, pos.z * 4.0, 2) * 2.0).sin();
    let aurora_intensity = (aurora_pattern + 1.0) / 2.0;
    let aurora_color = Color::from_float(0.3, 0.8, 0.9);
    base_color = mix_color(base_color, aurora_color, aurora_intensity * 0.3);
    
    // Aplicar iluminación Phong (hielo es muy reflectante)
    base_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        base_color,
        0.4,
        0.6,
        0.9,
        128.0
    );
    
    // Capa 5: Atmósfera cristalina
    let view_dir = (uniforms.camera_position - fragment_pos).normalize();
    let fresnel = (1.0 - normal.dot(&view_dir).abs()).powf(2.0);
    let atm_color = Color::from_float(0.6, 0.85, 1.0);
    
    mix_color(base_color, atm_color, fresnel * 0.6)
}

// PLANETA ALIENÍGENA (Púrpura/Magenta con bioluminiscencia) - 5 capas
pub fn alien_planet_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Capa 1: Superficie base alienígena (púrpura/magenta)
    let surface_noise = fbm(pos.x * 3.0, pos.y * 3.0, pos.z * 3.0, 4);
    let alien_base1 = Color::from_float(0.6, 0.2, 0.8);
    let alien_base2 = Color::from_float(0.8, 0.3, 0.7);
    let mut base_color = mix_color(alien_base1, alien_base2, surface_noise);
    
    // Capa 2: Formaciones cristalinas alienígenas
    let crystal_pattern = worley_noise(pos.x * 6.0, pos.y * 6.0, pos.z * 6.0);
    let crystal_color = Color::from_float(0.4, 0.8, 0.9);
    base_color = mix_color(base_color, crystal_color, (crystal_pattern - 0.6).max(0.0) * 3.0);
    
    // Capa 3: Bioluminiscencia pulsante
    let pulse = (uniforms.time * 3.0).sin() * 0.3 + 0.7;
    let bio_pattern = fbm(
        pos.x * 8.0 + uniforms.time * 0.1,
        pos.y * 8.0,
        pos.z * 8.0 - uniforms.time * 0.08,
        3
    );
    let bio_spots = (bio_pattern - 0.6).max(0.0) * 4.0;
    let bio_color = Color::from_float(0.0, 1.0, 0.8);
    base_color = mix_color(base_color, bio_color * pulse, bio_spots.min(1.0));
    
    // Capa 4: Venas energéticas (líneas brillantes)
    let vein_pattern = turbulence(pos.x * 10.0, pos.y * 10.0, pos.z * 10.0, 3);
    let vein_intensity = (vein_pattern - 0.7).max(0.0) * 5.0;
    let vein_color = Color::from_float(1.0, 0.4, 0.9);
    base_color = mix_color(base_color, vein_color, vein_intensity.min(1.0) * 0.6);
    
    // Aplicar iluminación
    base_color = calculate_phong_lighting(
        fragment_pos,
        normal,
        uniforms.light_position,
        uniforms.camera_position,
        base_color,
        0.35,
        0.7,
        0.4,
        16.0
    );
    
    // Capa 5: Atmósfera extraña (gradiente multicolor)
    let view_dir = (uniforms.camera_position - fragment_pos).normalize();
    let atmosphere = (1.0 - normal.dot(&view_dir).abs()).powf(2.5);
    let atm_color = mix_color(
        Color::from_float(0.8, 0.2, 1.0),
        Color::from_float(0.2, 1.0, 0.8),
        (uniforms.time * 0.5).sin() * 0.5 + 0.5
    );
    
    mix_color(base_color, atm_color, atmosphere * 0.5)
}

// ============= ENUM Y FUNCIÓN DE SELECCIÓN =============

#[derive(Clone, Copy, PartialEq)]
pub enum CelestialBody {
    Sun,
    Earth,
    Jupiter,
    Mars,
    Saturn,
    Ring,
    Moon,
    LavaPlanet,
    IcePlanet,
    AlienPlanet,
}

pub fn get_celestial_shader(
    body: CelestialBody,
    fragment: &Fragment,
    vertex: &Vertex,
    uniforms: &Uniforms
) -> Color {
    match body {
        CelestialBody::Sun => sun_shader(fragment, vertex, uniforms.time),
        CelestialBody::Earth => earth_like_shader(fragment, vertex, uniforms),
        CelestialBody::Jupiter => gas_giant_shader(fragment, vertex, uniforms),
        CelestialBody::Mars => mars_like_shader(fragment, vertex, uniforms),
        CelestialBody::Saturn => saturn_like_shader(fragment, vertex, uniforms),
        CelestialBody::Ring => ring_shader(fragment, vertex, uniforms),
        CelestialBody::Moon => moon_shader(fragment, vertex, uniforms),
        CelestialBody::LavaPlanet => lava_planet_shader(fragment, vertex, uniforms),
        CelestialBody::IcePlanet => ice_planet_shader(fragment, vertex, uniforms),
        CelestialBody::AlienPlanet => alien_planet_shader(fragment, vertex, uniforms),
    }
}
