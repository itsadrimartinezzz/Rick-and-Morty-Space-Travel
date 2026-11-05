// Ruido de valor reproducible para shaders y para el mapa de normales
use glam::Vec3;

// Valor pseudoaleatorio de 0 a 1 para una celda
fn hash(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(374_761_393)
        .wrapping_add((y as u32).wrapping_mul(668_265_263))
        .wrapping_add((z as u32).wrapping_mul(2_147_483_647));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.
}

// Resultado de 0 a 1 continuo y reproducible
pub fn value(p: Vec3) -> f32 {
    let base = p.floor();
    let (x, y, z) = (base.x as i32, base.y as i32, base.z as i32);
    let f = p - base;
    let u = f * f * (Vec3::splat(3.) - 2. * f);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let edge = |dy, dz| lerp(hash(x, y + dy, z + dz), hash(x + 1, y + dy, z + dz), u.x);
    lerp(
        lerp(edge(0, 0), edge(1, 0), u.y),
        lerp(edge(0, 1), edge(1, 1), u.y),
        u.z,
    )
}

// Ruido de valor 2D que se repite cada `period` unidades para texturas sin costuras
pub fn periodic(x: f32, y: f32, period: i32, seed: i32) -> f32 {
    let (fx, fy) = (x.floor(), y.floor());
    let (ix, iy) = (fx as i32, fy as i32);
    let (tx, ty) = (x - fx, y - fy);
    let (u, v) = (tx * tx * (3. - 2. * tx), ty * ty * (3. - 2. * ty));
    let corner = |dx: i32, dy: i32| {
        hash((ix + dx).rem_euclid(period), (iy + dy).rem_euclid(period), seed)
    };
    let top = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * u;
    let bottom = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * u;
    top + (bottom - top) * v
}

// Suma de octavas de ruido normalizada de 0 a 1
pub fn fbm(mut p: Vec3, octaves: u32) -> f32 {
    let (mut sum, mut amplitude, mut total) = (0., 0.5, 0.);
    for _ in 0..octaves {
        sum += value(p) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        p *= 2.03;
    }
    sum / total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_reproducible_and_bounded() {
        let p = Vec3::new(1.3, -4.2, 7.7);
        assert_eq!(value(p), value(p));
        for i in 0..400 {
            let v = fbm(Vec3::splat(i as f32 * 0.137), 3);
            assert!((0. ..=1.).contains(&v));
        }
    }
}
