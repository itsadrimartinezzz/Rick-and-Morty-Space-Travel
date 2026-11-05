// Mapa de normales propio generado con ruido al iniciar y usado por el shader de terreno
use crate::ruido;
use std::sync::OnceLock;

/// Semilla del mapa de normales
pub const SEED: i32 = 4177;
const SIZE: usize = 256;
// Inclinacion de las normales respecto a las diferencias de altura
const SLOPE: f32 = 15.;

// Normales en espacio tangente una por texel
pub struct NormalMap {
    size: usize,
    texels: Vec<[f32; 3]>,
}

impl NormalMap {
    // Genera alturas con ruido y calcula sus normales con diferencias centrales
    pub fn generate(size: usize, seed: i32) -> Self {
        let height = |x: usize, y: usize| {
            let (mut sum, mut amplitude, mut period) = (0., 0.62, 6);
            for octave in 0..4 {
                let u = x as f32 / size as f32 * period as f32;
                let v = y as f32 / size as f32 * period as f32;
                // Los pliegues del valor absoluto dan aspecto de roca agrietada
                let wave = ruido::periodic(u, v, period, seed + octave);
                sum += (wave * 2. - 1.).abs() * amplitude;
                amplitude *= 0.5;
                period *= 2;
            }
            sum
        };
        let heights: Vec<f32> = (0..size * size)
            .map(|i| height(i % size, i / size))
            .collect();
        let at = |x: usize, y: usize| heights[y % size * size + x % size];
        let mut texels = Vec::with_capacity(size * size);
        for y in 0..size {
            for x in 0..size {
                let dx = (at(x + 1, y) - at(x + size - 1, y)) * SLOPE;
                let dy = (at(x, y + 1) - at(x, y + size - 1)) * SLOPE;
                let length = (dx * dx + dy * dy + 1.).sqrt();
                texels.push([-dx / length, -dy / length, 1. / length]);
            }
        }
        Self { size, texels }
    }

    // Lectura con repeticion: `u` y `v` dan una vuelta completa cada unidad
    pub fn sample(&self, u: f32, v: f32) -> [f32; 3] {
        let wrap = |c: f32| ((c - c.floor()) * self.size as f32) as usize % self.size;
        self.texels[wrap(v) * self.size + wrap(u)]
    }
}

// Mapa compartido por todos los objetos
pub fn shared() -> &'static NormalMap {
    static MAP: OnceLock<NormalMap> = OnceLock::new();
    MAP.get_or_init(|| NormalMap::generate(SIZE, SEED))
}

// Luz por pixel en espacio tangente; `strength` en 0 equivale a la cara plana
pub fn diffuse(normal: [f32; 3], strength: f32, light: [f32; 3]) -> f32 {
    let (x, y, z) = (normal[0] * strength, normal[1] * strength, normal[2]);
    let length = (x * x + y * y + z * z).sqrt();
    (x * light[0] + y * light[1] + z * light[2]) / length
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normals_are_unit_and_face_outward() {
        let map = NormalMap::generate(64, SEED);
        for texel in &map.texels {
            let length = (texel[0].powi(2) + texel[1].powi(2) + texel[2].powi(2)).sqrt();
            assert!((length - 1.).abs() < 1e-3);
            assert!(texel[2] > 0.);
        }
    }

    #[test]
    fn same_seed_gives_the_same_map_and_another_seed_changes_it() {
        let a = NormalMap::generate(64, 3);
        let b = NormalMap::generate(64, 3);
        let c = NormalMap::generate(64, 4);
        assert_eq!(a.texels, b.texels);
        assert_ne!(a.texels, c.texels);
    }

    #[test]
    fn sampling_wraps_around() {
        let map = NormalMap::generate(64, SEED);
        assert_eq!(map.sample(0.25, 0.5), map.sample(1.25, -0.5));
    }

    #[test]
    fn map_has_no_seam_at_the_edge() {
        let map = NormalMap::generate(128, SEED);
        let mut biggest_inside: f32 = 0.;
        let mut biggest_edge: f32 = 0.;
        for y in 0..128 {
            let row = |x: usize| map.texels[y * 128 + x][0];
            biggest_inside = biggest_inside.max((row(64) - row(63)).abs());
            biggest_edge = biggest_edge.max((row(0) - row(127)).abs());
        }
        assert!(biggest_edge < biggest_inside * 2.5);
    }

    #[test]
    fn strength_zero_is_the_flat_normal_and_strength_changes_the_light() {
        let light = [0.6, 0., 0.8];
        let toward_light = [0.5, 0., 0.866];
        let away_from_light = [-0.5, 0., 0.866];
        assert!((diffuse(toward_light, 0., light) - 0.8).abs() < 1e-5);
        assert!(diffuse(toward_light, 1., light) > 0.8);
        assert!(diffuse(away_from_light, 1., light) < 0.8);
    }
}
