use nalgebra_glm::Vec3;
use crate::color::Color;
use crate::fragment::Fragment;
use crate::vertex::Vertex;
use crate::Uniforms;

// ============= FUNCIONES DE NOISE MEJORADAS =============

// Función auxiliar para ruido pseudo-aleatorio
fn noise(x: f32, y: f32, z: f32) -> f32 {
    let a = (x * 12.9898 + y * 78.233 + z * 45.164).sin() * 43758.5453;
    a.fract()
}

// Interpolación suave (smoothstep) para transiciones más naturales
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// Ruido interpolado linealmente para reducir pixelación
fn noise_interpolated(x: f32, y: f32, z: f32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let zi = z.floor();
    
    let xf = x - xi;
    let yf = y - yi;
    let zf = z - zi;
    
    // Interpolar con smoothstep para transiciones más suaves
    let u = smoothstep(xf);
    let v = smoothstep(yf);
    let w = smoothstep(zf);
    
    // 8 esquinas del cubo
    let n000 = noise(xi, yi, zi);
    let n100 = noise(xi + 1.0, yi, zi);
    let n010 = noise(xi, yi + 1.0, zi);
    let n110 = noise(xi + 1.0, yi + 1.0, zi);
    let n001 = noise(xi, yi, zi + 1.0);
    let n101 = noise(xi + 1.0, yi, zi + 1.0);
    let n011 = noise(xi, yi + 1.0, zi + 1.0);
    let n111 = noise(xi + 1.0, yi + 1.0, zi + 1.0);
    
    // Interpolación trilinear
    let x00 = n000 * (1.0 - u) + n100 * u;
    let x10 = n010 * (1.0 - u) + n110 * u;
    let x01 = n001 * (1.0 - u) + n101 * u;
    let x11 = n011 * (1.0 - u) + n111 * u;
    
    let y0 = x00 * (1.0 - v) + x10 * v;
    let y1 = x01 * (1.0 - v) + x11 * v;
    
    y0 * (1.0 - w) + y1 * w
}

// Función para ruido fractal (Fractal Brownian Motion) con interpolación
fn fbm(x: f32, y: f32, z: f32, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    let mut max_value = 0.0;
    
    for _ in 0..octaves {
        value += noise_interpolated(x * frequency, y * frequency, z * frequency) * amplitude;
        max_value += amplitude;
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    
    // Normalizar para mantener el rango [0, 1]
    if max_value > 0.0 {
        value / max_value
    } else {
        value
    }
}

// Worley/Cellular noise mejorado para efectos de células más suaves
fn worley_noise(x: f32, y: f32, z: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let zi = z.floor() as i32;
    
    let mut min_dist: f32 = 100.0;
    let mut second_min_dist: f32 = 100.0;
    
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
                
                if dist < min_dist {
                    second_min_dist = min_dist;
                    min_dist = dist;
                } else if dist < second_min_dist {
                    second_min_dist = dist;
                }
            }
        }
    }
    
    // Usar la diferencia entre las dos distancias más cercanas para bordes más suaves
    (second_min_dist - min_dist).clamp(0.0, 1.0)
}

// Turbulencia para efectos caóticos con interpolación suave
fn turbulence(x: f32, y: f32, z: f32, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_value = 0.0;
    
    for _ in 0..octaves {
        let n = noise_interpolated(x * frequency, y * frequency, z * frequency);
        value += (n - 0.5).abs() * amplitude;
        max_value += amplitude * 0.5;
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    
    // Normalizar
    if max_value > 0.0 {
        value / max_value
    } else {
        value
    }
}

// Helper para mezclar colores con interpolación suave
fn mix_color(c1: Color, c2: Color, t: f32) -> Color {
    let t = smoothstep(t.clamp(0.0, 1.0)); // Usar smoothstep para transiciones más naturales
    Color::from_float(
        c1.to_float().0 * (1.0 - t) + c2.to_float().0 * t,
        c1.to_float().1 * (1.0 - t) + c2.to_float().1 * t,
        c1.to_float().2 * (1.0 - t) + c2.to_float().2 * t,
    )
}

// Mezclar múltiples colores con pesos
fn mix_colors_multi(colors: &[Color], weights: &[f32]) -> Color {
    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;
    let mut total_weight = 0.0;
    
    for (color, &weight) in colors.iter().zip(weights.iter()) {
        let (cr, cg, cb) = color.to_float();
        r += cr * weight;
        g += cg * weight;
        b += cb * weight;
        total_weight += weight;
    }
    
    if total_weight > 0.0 {
        Color::from_float(r / total_weight, g / total_weight, b / total_weight)
    } else {
        colors[0]
    }
}

fn scale_octaves(base: u32, detail_level: f32) -> u32 {
    let detail = detail_level.clamp(0.4, 1.0);
    let scaled = (base as f32 * detail).floor() as u32;
    scaled.max(1).min(base)
}

fn fbm_adaptive(x: f32, y: f32, z: f32, base_octaves: u32, detail_level: f32) -> f32 {
    fbm(x, y, z, scale_octaves(base_octaves, detail_level))
}

fn turbulence_adaptive(x: f32, y: f32, z: f32, base_octaves: u32, detail_level: f32) -> f32 {
    turbulence(x, y, z, scale_octaves(base_octaves, detail_level))
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
    
    // OCÉANOS REALISTAS - Colores tipo Tierra real
    let ocean_depth = fbm(pos.x * 3.5, pos.y * 3.5, pos.z * 3.5, 4);
    let ocean_waves = fbm(pos.x * 18.0, pos.y * 18.0, pos.z * 18.0, 2) * 0.1;
    
    // Océanos profundos azul oscuro, océanos poco profundos más turquesa
    let deep_ocean = Color::from_float(0.01, 0.05, 0.15);      // Azul muy oscuro
    let shallow_ocean = Color::from_float(0.05, 0.25, 0.45);   // Azul medio
    
    // CONTINENTES REALISTAS - Usar múltiples capas de noise para formas irregulares
    // Combinar Worley + FBM para crear continentes más naturales
    let continent_base = worley_noise(pos.x * 1.2, pos.y * 1.2, pos.z * 1.2);
    let continent_detail = fbm(pos.x * 2.5, pos.y * 2.5, pos.z * 2.5, 5);
    let continent_variation = fbm(pos.x * 1.8, pos.y * 1.8, pos.z * 1.8, 4);
    
    // Ajustar umbral para tener ~30% de tierra (como la Tierra real)
    let land_threshold = 0.48 + continent_variation * 0.08;
    let is_land = (continent_base > land_threshold) || (continent_detail > 0.62 && continent_base > 0.42);
    
    // BIOMAS TERRESTRES REALISTAS - Colores tipo Tierra
    let biome_noise = fbm(pos.x * 2.8, pos.y * 2.8, pos.z * 2.8, 4);
    let altitude = fbm(pos.x * 4.5, pos.y * 4.5, pos.z * 4.5, 3);
    let coastal_distance = fbm(pos.x * 6.0, pos.y * 6.0, pos.z * 6.0, 3);
    
    // Colores más realistas de la Tierra
    let forest = Color::from_float(0.13, 0.38, 0.13);        // Verde bosque oscuro
    let plains = Color::from_float(0.42, 0.48, 0.22);        // Verde/amarillo praderas
    let desert = Color::from_float(0.76, 0.60, 0.35);        // Arena/desierto cálido
    let mountain = Color::from_float(0.45, 0.40, 0.35);      // Marrón/gris montaña
    let snow = Color::from_float(0.95, 0.95, 0.98);          // Nieve brillante
    let tundra = Color::from_float(0.55, 0.50, 0.45);        // Tundra ártica
    let beach_sand = Color::from_float(0.88, 0.82, 0.65);    // Arena de playa
    
    let mut base_color = if is_land {
        // BIOMAS REALISTAS con transiciones suaves
        let latitude_factor = pos.y.abs(); // 0 = ecuador, 1 = polos
        
        if altitude > 0.78 {
            // MONTAÑAS ALTAS con nieve
            if altitude > 0.88 || latitude_factor > 0.65 {
                snow // Nieve permanente
            } else {
                mix_color(mountain, snow, (altitude - 0.78) * 4.0) // Transición montaña-nieve
            }
        } else if latitude_factor > 0.55 {
            // ZONAS ÁRTICAS/ANTÁRTICAS (norte/sur lejanos)
            if altitude > 0.65 {
                mix_color(tundra, snow, (latitude_factor - 0.55) * 3.0) // Tundra nevada
            } else {
                mix_color(plains, tundra, (latitude_factor - 0.55) * 4.0) // Pradera fría
            }
        } else if biome_noise > 0.65 {
            // DESIERTOS (África, Arabia, Australia)
            let desert_intensity = (biome_noise - 0.65) * 2.8;
            mix_color(plains, desert, desert_intensity.clamp(0.0, 1.0))
        } else if biome_noise > 0.45 {
            // PRADERAS Y SABANAS (transición)
            let plains_blend = (biome_noise - 0.45) * 5.0;
            mix_color(forest, plains, plains_blend.clamp(0.0, 1.0))
        } else {
            // BOSQUES Y SELVAS (zonas húmedas y ecuatoriales)
            let forest_variation = biome_noise * 2.2;
            forest * (0.7 + forest_variation * 0.3)
        }
    } else {
        // OCÉANO con profundidad y costas
        let ocean_color = mix_color(deep_ocean, shallow_ocean, ocean_depth + ocean_waves);
        
        // Zonas costeras con arena (transición océano-tierra)
        if coastal_distance > 0.56 && continent_base > 0.40 && continent_base < land_threshold {
            let coast_blend = (coastal_distance - 0.56) * 6.0;
            mix_color(ocean_color, beach_sand, coast_blend.clamp(0.0, 0.7))
        } else {
            ocean_color
        }
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
    
    // NUBES REALISTAS - Sistema de 3 capas que se mueven
    // Nubes grandes (sistemas climáticos)
    let cloud_large = fbm(
        pos.x * 3.5 + uniforms.time * 0.05,
        pos.y * 3.5,
        pos.z * 3.5 - uniforms.time * 0.03,
        4
    );
    // Nubes medianas (formaciones)
    let cloud_medium = fbm(
        pos.x * 7.0 - uniforms.time * 0.07,
        pos.y * 7.0,
        pos.z * 7.0 + uniforms.time * 0.04,
        3
    );
    // Detalles finos (cirrus, etc)
    let cloud_fine = fbm(
        pos.x * 12.0,
        pos.y * 12.0,
        pos.z * 12.0,
        2
    ) * 0.25;
    
    // Combinar capas (más nubes en zonas ecuatoriales)
    let latitude_cloud_factor = 1.0 - (pos.y.abs() * 0.5); // Más nubes cerca del ecuador
    let cloud_combined = (cloud_large * 0.5 + cloud_medium * 0.3 + cloud_fine) * latitude_cloud_factor;
    let cloud_intensity = (cloud_combined - 0.45).max(0.0) * 2.0;
    
    // Iluminación de nubes (sombras realistas)
    let light_dir = (uniforms.light_position - fragment_pos).normalize();
    let cloud_lighting = (normal.dot(&light_dir).max(0.0) * 0.75 + 0.25).min(1.0);
    let cloud_color = Color::from_float(0.98, 0.98, 1.0) * cloud_lighting;
    
    // Aplicar nubes con transparencia variable
    base_color = mix_color(base_color, cloud_color, (cloud_intensity * 0.7).min(0.75));
    
    // ATMÓSFERA AZUL REALISTA - Efecto Rayleigh scattering
    let view_dir = (uniforms.camera_position - fragment_pos).normalize();
    let fresnel = (1.0 - normal.dot(&view_dir).abs()).powf(2.8); // Borde atmosférico
    
    // Color de atmósfera terrestre (azul cielo)
    let atmosphere_color = Color::from_float(0.35, 0.55, 0.95);
    
    // Agregar brillo atmosférico más intenso en el borde
    let atmosphere_glow = fresnel * 0.45;
    
    mix_color(base_color, atmosphere_color, atmosphere_glow)
}

// ============= GIGANTE GASEOSO (TIPO JÚPITER) =============
// Shader con 7+ capas: atmósfera profunda, bandas en múltiples alturas, turbulencias,
// gran mancha roja, tormentas secundarias, scattering, brillo volumétrico
pub fn gas_giant_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    let view_dir = (uniforms.camera_position - fragment_pos).normalize();
    let detail = uniforms.detail_level;

    // Calcular profundidad atmosférica (más denso en el centro, menos en los bordes)
    let edge_factor = normal.dot(&view_dir).abs();
    let atmospheric_depth = (1.0 - edge_factor).powf(0.5);

    // ===== CAPA 1: Atmósfera profunda base (colores más precisos de Júpiter) =====
    // Júpiter tiene tonos naranjas, cremas y marrones
    let deep_atm_noise = fbm_adaptive(pos.x * 2.5, pos.y * 2.5, pos.z * 2.5, 4, detail);
    let deep_color1 = Color::from_float(0.82, 0.58, 0.35); // Naranja cálido
    let deep_color2 = Color::from_float(0.68, 0.45, 0.28); // Marrón dorado
    let deep_layer = mix_color(deep_color1, deep_color2, deep_atm_noise);

    // ===== CAPA 2: Bandas atmosféricas horizontales (como en la referencia de Three.js) =====
    // Júpiter tiene bandas muy pronunciadas con mucha turbulencia
    let band_freq = 14.0; // Más bandas para mayor realismo
    
    // Banda lenta (ecuatorial)
    let slow_distortion = turbulence_adaptive(
        pos.x * 3.0 + uniforms.time * 0.015,
        pos.y * 2.0,
        pos.z * 3.0 - uniforms.time * 0.012,
        5, // Más octavas para bandas suaves
        detail,
    ) * 1.5;
    let slow_band = ((pos.y + slow_distortion) * band_freq * 0.7).sin();

    // Banda media (zonas templadas)
    let mid_distortion = turbulence_adaptive(
        pos.x * 4.0 + uniforms.time * 0.028,
        pos.y * 3.0,
        pos.z * 4.0 - uniforms.time * 0.022,
        5,
        detail,
    ) * 1.1;
    let mid_band = ((pos.y + mid_distortion) * band_freq).sin();

    // Banda rápida (zonas polares)
    let fast_distortion = turbulence_adaptive(
        pos.x * 5.5 + uniforms.time * 0.045,
        pos.y * 3.8,
        pos.z * 5.5 - uniforms.time * 0.038,
        4,
        detail,
    ) * 0.8;
    let fast_band = ((pos.y + fast_distortion) * band_freq * 1.3).sin();

    // Colores más precisos de Júpiter (inspirados en imágenes reales)
    let band_color1 = Color::from_float(0.98, 0.88, 0.72); // Zona clara (crema brillante)
    let band_color2 = Color::from_float(0.75, 0.52, 0.32); // Cinturón oscuro (marrón rojizo)
    let band_color3 = Color::from_float(0.92, 0.78, 0.58); // Zona intermedia (naranja suave)
    let band_color4 = Color::from_float(0.68, 0.45, 0.28); // Cinturón profundo (marrón oscuro)

    let combined_band = slow_band * 0.4 + mid_band * 0.35 + fast_band * 0.25;
    let band_value = (combined_band + 1.0) / 2.0;

    let band_color = if band_value > 0.75 {
        band_color1
    } else if band_value > 0.5 {
        mix_color(band_color3, band_color1, (band_value - 0.5) * 4.0)
    } else if band_value > 0.25 {
        mix_color(band_color2, band_color3, (band_value - 0.25) * 4.0)
    } else {
        mix_color(band_color4, band_color2, band_value * 4.0)
    };

    let mut base_color = mix_color(deep_layer, band_color, 0.4 + atmospheric_depth * 0.6);

    // ===== CAPA 3: Turbulencias y vórtices (tormentas joviales) =====
    // Júpiter tiene miles de tormentas, vamos a simular múltiples escalas
    let large_vortex = turbulence_adaptive(
        pos.x * 7.0 + uniforms.time * 0.035,
        pos.y * 5.0,
        pos.z * 7.0 - uniforms.time * 0.03,
        6, // Más octavas para tormentas complejas
        detail,
    );
    let medium_vortex = turbulence_adaptive(
        pos.x * 12.0 + uniforms.time * 0.06,
        pos.y * 8.0,
        pos.z * 12.0 - uniforms.time * 0.05,
        5,
        detail,
    );
    let small_vortex = turbulence_adaptive(
        pos.x * 18.0 + uniforms.time * 0.09,
        pos.y * 12.0,
        pos.z * 18.0 - uniforms.time * 0.08,
        4,
        detail,
    );
    let vortex_combined = large_vortex * 0.5 + medium_vortex * 0.3 + small_vortex * 0.2;
    let vortex_color = Color::from_float(0.85, 0.65, 0.45); // Naranja turbulento
    base_color = mix_color(base_color, vortex_color, vortex_combined * 0.4);

    // ===== CAPA 4: Gran Mancha Roja (Great Red Spot) =====
    // La tormenta más famosa del sistema solar - tiene que verse BIEN
    let storm_center = Vec3::new(0.3, -0.12, 0.65);
    let dx = pos.x - storm_center.x;
    let dy = (pos.y - storm_center.y) * 1.8; // Elíptica (más ancha que alta)
    let dz = pos.z - storm_center.z;
    let dist_to_storm = (dx * dx + dy * dy + dz * dz).sqrt();

    let storm_radius = 0.38; // Más grande
    let storm_intensity = (1.0 - (dist_to_storm / storm_radius)).max(0.0).powf(1.3);
    
    // Rotación de la tormenta (anti-ciclónica)
    let angle = pos.x.atan2(pos.z) + uniforms.time * 0.08;
    let storm_swirl = turbulence_adaptive(
        pos.x * 16.0 + angle.cos() * 3.0,
        pos.y * 16.0,
        pos.z * 16.0 + angle.sin() * 3.0,
        6, // Más detalle en la mancha
        detail,
    );

    // Colores de la Gran Mancha Roja (rojo ladrillo con bordes naranjas)
    let storm_center_color = Color::from_float(0.92, 0.22, 0.12); // Rojo intenso
    let storm_mid_color = Color::from_float(0.88, 0.35, 0.18);    // Rojo anaranjado
    let storm_edge_color = Color::from_float(0.82, 0.48, 0.28);   // Naranja
    
    let storm_color = if storm_intensity > 0.6 {
        mix_color(storm_mid_color, storm_center_color, (storm_intensity - 0.6) * 2.5)
    } else {
        mix_color(storm_edge_color, storm_mid_color, storm_intensity * 1.67)
    };
    
    base_color = mix_color(base_color, storm_color, storm_intensity * (0.7 + storm_swirl * 0.3));

    // ===== CAPA 5: Tormentas secundarias =====
    let white_spot_center = Vec3::new(-0.35, 0.35, 0.5);
    let dist_white = ((pos - white_spot_center).magnitude() * 7.0 - 1.0).max(0.0);
    let white_spot_intensity = (1.0 - dist_white).max(0.0).powf(2.0);
    let white_storm_color = Color::from_float(0.95, 0.85, 0.70);
    base_color = mix_color(base_color, white_storm_color, white_spot_intensity * 0.5);

    let brown_spot_center = Vec3::new(0.4, 0.25, -0.4);
    let dist_brown = ((pos - brown_spot_center).magnitude() * 9.0 - 1.0).max(0.0);
    let brown_spot_intensity = (1.0 - dist_brown).max(0.0).powf(2.5);
    let brown_storm_color = Color::from_float(0.65, 0.45, 0.30);
    base_color = mix_color(base_color, brown_storm_color, brown_spot_intensity * 0.4);

    // ===== CAPA 6: Nubes de alta altitud =====
    let high_clouds = fbm_adaptive(
        pos.x * 8.0 + uniforms.time * 0.12,
        pos.y * 8.0,
        pos.z * 8.0 - uniforms.time * 0.1,
        3,
        detail,
    );
    let cloud_intensity = ((high_clouds - 0.55).max(0.0) * 3.0).min(1.0);
    let high_cloud_color = Color::from_float(0.98, 0.90, 0.75);
    base_color = mix_color(base_color, high_cloud_color, cloud_intensity * 0.25);

    // ===== CAPA 7: Iluminación atmosférica realista (inspirada en Three.js) =====
    let light_dir = (uniforms.light_position - fragment_pos).normalize();
    
    // Diffuse con wrap lighting para atmósfera densa
    let diffuse_factor = (normal.dot(&light_dir) * 0.6 + 0.4).max(0.0);
    
    // Subsurface scattering (la luz atraviesa las nubes)
    let subsurface = (-normal.dot(&light_dir)).max(0.0).powf(1.8) * (0.25 * detail + 0.08);
    
    // Ambient más cálido (luz reflejada de otras partes del planeta)
    let ambient = 0.28;
    
    // Specular suave (nubes brillantes)
    let reflect_dir = reflect(-light_dir, normal);
    let spec = reflect_dir.dot(&view_dir).max(0.0).powf(6.0) * 0.12;
    
    // Fresnel para bordes más brillantes
    let fresnel = (1.0 - edge_factor).powf(2.5) * 0.18;

    let lighting = ambient + diffuse_factor * 0.85 + subsurface + spec + fresnel;
    base_color = base_color * lighting.clamp(0.3, 1.8);

    // ===== CAPA 8: Scattering atmosférico (rayos de luz dispersándose) =====
    let scatter_intensity = (1.0 - edge_factor).powf(2.8);
    let scatter_color = Color::from_float(0.92, 0.78, 0.62); // Naranja dorado cálido
    base_color = mix_color(base_color, scatter_color, scatter_intensity * 0.25);

    // ===== CAPA 9: Rim Light volumétrico (brillo atmosférico en los bordes) =====
    let rim_light = (1.0 - edge_factor).powf(2.2);
    let rim_color = Color::from_float(0.98, 0.82, 0.62);
    base_color = mix_color(base_color, rim_color, rim_light * 0.35);

    // ===== CAPA 10: Variación de densidad =====
    let density_variation = fbm_adaptive(
        pos.x * 12.0 + uniforms.time * 0.06,
        pos.y * 12.0,
        pos.z * 12.0,
        2,
        detail,
    );
    let density_factor = 0.7 + density_variation * 0.3;
    base_color = base_color * density_factor;

    base_color
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
// Shader con 7+ capas: atmósfera profunda, bandas en múltiples altitudes, turbulencias sutiles,
// hexágono polar, corrientes de viento, scattering, brillo volumétrico
pub fn saturn_like_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    let view_dir = (uniforms.camera_position - fragment_pos).normalize();
    let detail = uniforms.detail_level;
    
    // Calcular profundidad atmosférica
    let edge_factor = normal.dot(&view_dir).abs();
    let atmospheric_depth = (1.0 - edge_factor).powf(0.5);
    
    // ===== CAPA 1: Atmósfera profunda base (tonos crema/beige) =====
    let deep_atm_noise = fbm_adaptive(pos.x * 1.8, pos.y * 1.8, pos.z * 1.8, 3, detail);
    let deep_color1 = Color::from_float(0.90, 0.85, 0.68);
    let deep_color2 = Color::from_float(0.85, 0.80, 0.63);
    let deep_layer = mix_color(deep_color1, deep_color2, deep_atm_noise);
    
    // ===== CAPA 2: Bandas atmosféricas en múltiples altitudes =====
    let band_freq = 9.0;
    
    // Banda lenta (capa profunda) - movimiento lento hacia el este
    let slow_distortion = fbm_adaptive(
        pos.x * 2.0 + uniforms.time * 0.015,
        pos.y * 1.5,
        pos.z * 2.0 - uniforms.time * 0.01,
        3,
        detail,
    ) * 0.6;
    let slow_band = ((pos.y + slow_distortion) * band_freq * 0.9).sin();
    
    // Banda media
    let mid_distortion = fbm_adaptive(
        pos.x * 3.0 + uniforms.time * 0.025,
        pos.y * 2.0,
        pos.z * 3.0 - uniforms.time * 0.018,
        3,
        detail,
    ) * 0.4;
    let mid_band = ((pos.y + mid_distortion) * band_freq).sin();
    
    // Banda rápida (capa superior) - nubes rápidas
    let fast_distortion = fbm_adaptive(
        pos.x * 4.0 + uniforms.time * 0.04,
        pos.y * 2.5,
        pos.z * 4.0 - uniforms.time * 0.035,
        2,
        detail,
    ) * 0.3;
    let fast_band = ((pos.y + fast_distortion) * band_freq * 1.1).sin();
    
    // Colores de bandas (tonos pastel suaves)
    let band_color1 = Color::from_float(0.98, 0.94, 0.78);  // Crema muy claro
    let band_color2 = Color::from_float(0.88, 0.84, 0.68);  // Beige
    let band_color3 = Color::from_float(0.93, 0.89, 0.73);  // Intermedio
    let band_color4 = Color::from_float(0.84, 0.80, 0.65);  // Beige oscuro
    
    // Combinar bandas
    let combined_band = slow_band * 0.4 + mid_band * 0.4 + fast_band * 0.2;
    let band_value = (combined_band + 1.0) / 2.0;
    
    let band_color = if band_value > 0.75 {
        band_color1
    } else if band_value > 0.5 {
        mix_color(band_color3, band_color1, (band_value - 0.5) * 4.0)
    } else if band_value > 0.25 {
        mix_color(band_color2, band_color3, (band_value - 0.25) * 4.0)
    } else {
        mix_color(band_color4, band_color2, band_value * 4.0)
    };
    
    // Mezclar capa profunda con bandas
    let mut base_color = mix_color(deep_layer, band_color, 0.3 + atmospheric_depth * 0.7);
    
    // ===== CAPA 3: Turbulencias sutiles (más suaves que Júpiter) =====
    let gentle_turbulence = fbm_adaptive(
        pos.x * 5.0 + uniforms.time * 0.028,
        pos.y * 3.5,
        pos.z * 5.0 - uniforms.time * 0.022,
        4,
        detail,
    );
    let turb_color = Color::from_float(0.91, 0.87, 0.71);
    base_color = mix_color(base_color, turb_color, gentle_turbulence * 0.25);
    
    // ===== CAPA 4: Corrientes de viento (jet streams) =====
    // Saturno tiene vientos muy rápidos en ciertas latitudes
    let wind_latitude = pos.y;
    let wind_strength = if wind_latitude.abs() > 0.4 && wind_latitude.abs() < 0.6 {
        1.0
    } else {
        0.0
    };
    
    let wind_pattern = fbm_adaptive(
        pos.x * 15.0 + uniforms.time * 0.15,
        pos.y * 10.0,
        pos.z * 15.0,
        2,
        detail,
    );
    let wind_color = Color::from_float(0.96, 0.92, 0.76);
    base_color = mix_color(base_color, wind_color, wind_pattern * wind_strength * 0.3);
    
    // ===== CAPA 5: Hexágono en polo norte (característica real única de Saturno) =====
    if pos.y > 0.68 {
        let angle = pos.x.atan2(pos.z);
        
        // Crear patrón hexagonal (6 lados)
        let hex_sides = 6.0;
        let hex_angle = angle * hex_sides / 2.0;
        let hex_pattern = hex_angle.cos();
        
        // Intensidad basada en latitud y patrón hexagonal
        let lat_factor = ((pos.y - 0.68) * 8.0).min(1.0);
        let hex_intensity = hex_pattern * lat_factor;
        
        // Color del hexágono (más oscuro)
        let hex_color = Color::from_float(0.78, 0.74, 0.60);
        base_color = mix_color(base_color, hex_color, hex_intensity.abs() * 0.4);
        
        // Agregar turbulencia dentro del hexágono
        let hex_turb = turbulence_adaptive(
            pos.x * 12.0 + uniforms.time * 0.08,
            pos.y * 12.0,
            pos.z * 12.0 - uniforms.time * 0.06,
            3,
            detail,
        );
        let hex_turb_color = Color::from_float(0.82, 0.78, 0.64);
        base_color = mix_color(base_color, hex_turb_color, hex_turb * lat_factor * 0.3);
    }
    
    // ===== CAPA 6: Nubes de alta altitud (wispy clouds) =====
    let high_clouds = fbm_adaptive(
        pos.x * 7.0 + uniforms.time * 0.08,
        pos.y * 7.0,
        pos.z * 7.0 - uniforms.time * 0.06,
        3,
        detail,
    );
    let cloud_intensity = ((high_clouds - 0.6).max(0.0) * 3.5).min(1.0);
    let wispy_color = Color::from_float(0.99, 0.96, 0.82);
    base_color = mix_color(base_color, wispy_color, cloud_intensity * 0.2);
    
    // ===== CAPA 7: Iluminación atmosférica (gas dispersa luz suavemente) =====
    let light_dir = (uniforms.light_position - fragment_pos).normalize();
    let diffuse_factor = (normal.dot(&light_dir) * 0.5 + 0.5).max(0.0); // Wrap lighting
    
    // Subsurface scattering
    let subsurface = (-normal.dot(&light_dir)).max(0.0).powf(2.5) * (0.18 + 0.12 * detail);
    
    let ambient = 0.4;
    let spec = reflect(-light_dir, normal).dot(&view_dir).max(0.0).powf(3.0) * 0.12;
    
    let lighting = ambient + diffuse_factor * 0.65 + subsurface + spec;
    base_color = base_color * lighting.min(1.4);
    
    // ===== CAPA 8: Scattering atmosférico (tonos dorados) =====
    let scatter_intensity = (1.0 - edge_factor).powf(3.5);
    let scatter_color = Color::from_float(0.95, 0.91, 0.75);
    base_color = mix_color(base_color, scatter_color, scatter_intensity * 0.18);
    
    // ===== CAPA 9: Brillo volumétrico suave en los bordes =====
    let rim_light = (1.0 - edge_factor).powf(2.2);
    let rim_color = Color::from_float(0.99, 0.95, 0.80);
    base_color = mix_color(base_color, rim_color, rim_light * 0.25);
    
    // ===== CAPA 10: Variación de densidad (atmósfera menos densa en los bordes) =====
    let density_variation = fbm_adaptive(
        pos.x * 10.0 + uniforms.time * 0.04,
        pos.y * 10.0,
        pos.z * 10.0,
        2,
        detail,
    );
    let density_factor = 0.75 + density_variation * 0.25;
    base_color = base_color * density_factor;
    
    base_color
}

// ============= ANILLOS MEJORADOS =============
// Shader con 4 capas: bandas principales, gaps, partículas, sombras
pub fn ring_shader(_fragment: &Fragment, vertex: &Vertex, uniforms: &Uniforms) -> Color {
    let pos = vertex.position;
    let normal = vertex.transformed_normal.normalize();
    let fragment_pos = vertex.transformed_position;
    
    // Distancia radial desde el centro (en el plano XZ)
    let radial_dist = (pos.x * pos.x + pos.z * pos.z).sqrt();
    
    // IMPORTANTE: Solo renderizar anillos entre ciertos radios (crear el "agujero" en el centro)
    // Los anillos están entre 0.6 y 1.0 del radio normalizado
    if radial_dist < 0.6 || radial_dist > 1.0 || pos.y.abs() > 0.05 {
        // Fuera del rango de anillos o demasiado lejos del plano ecuatorial = transparente/negro
        return Color::new(0, 0, 0);
    }
    
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
