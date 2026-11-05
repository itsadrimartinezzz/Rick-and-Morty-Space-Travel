// Niveles de detalle generados al iniciar uniendo los vertices de cada celda de una rejilla
use glam::Vec3;
use std::collections::{HashMap, HashSet};

// Malla resultante de simplificar
pub struct Simplified<C> {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<[usize; 3]>,
    pub colors: Vec<C>,
}

// Une los vertices de cada celda y descarta los triangulos sin area
pub fn cluster<C: Copy>(
    vertices: &[Vec3],
    faces: &[[usize; 3]],
    colors: &[C],
    cell: f32,
) -> Simplified<C> {
    let mut cells: HashMap<(i32, i32, i32), usize> = HashMap::new();
    let mut sums: Vec<(Vec3, f32)> = Vec::new();
    let mut remap = Vec::with_capacity(vertices.len());
    for &v in vertices {
        let key = (
            (v.x / cell).floor() as i32,
            (v.y / cell).floor() as i32,
            (v.z / cell).floor() as i32,
        );
        let index = *cells.entry(key).or_insert_with(|| {
            sums.push((Vec3::ZERO, 0.));
            sums.len() - 1
        });
        sums[index].0 += v;
        sums[index].1 += 1.;
        remap.push(index);
    }
    let merged: Vec<Vec3> = sums.iter().map(|(sum, count)| *sum / *count).collect();
    let mut seen = HashSet::new();
    let mut new_faces = Vec::new();
    let mut new_colors = Vec::new();
    for (face, color) in faces.iter().zip(colors) {
        let [a, b, c] = face.map(|i| remap[i]);
        if a == b || b == c || a == c {
            continue;
        }
        // Rotar conserva el sentido: las caras opuestas no cuentan como iguales
        let rotated = if a < b && a < c {
            [a, b, c]
        } else if b < c {
            [b, c, a]
        } else {
            [c, a, b]
        };
        if seen.insert(rotated) {
            new_faces.push([a, b, c]);
            new_colors.push(*color);
        }
    }
    Simplified {
        vertices: merged,
        faces: new_faces,
        colors: new_colors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(n: usize, step: f32) -> (Vec<Vec3>, Vec<[usize; 3]>) {
        let mut vertices = vec![];
        let mut faces = vec![];
        for y in 0..=n {
            for x in 0..=n {
                vertices.push(Vec3::new(x as f32 * step, y as f32 * step, 0.));
            }
        }
        for y in 0..n {
            for x in 0..n {
                let a = y * (n + 1) + x;
                faces.push([a, a + 1, a + n + 2]);
                faces.push([a, a + n + 2, a + n + 1]);
            }
        }
        (vertices, faces)
    }

    #[test]
    fn larger_cells_mean_fewer_triangles() {
        let (vertices, faces) = grid(40, 0.1);
        let colors = vec![0u8; faces.len()];
        let fine = cluster(&vertices, &faces, &colors, 0.2);
        let coarse = cluster(&vertices, &faces, &colors, 0.8);
        assert!(fine.faces.len() < faces.len());
        assert!(coarse.faces.len() < fine.faces.len());
        assert_eq!(coarse.faces.len(), coarse.colors.len());
    }

    #[test]
    fn a_shape_bigger_than_the_cell_survives_and_keeps_its_size() {
        let (vertices, faces) = grid(40, 0.1);
        let colors = vec![0u8; faces.len()];
        let simple = cluster(&vertices, &faces, &colors, 0.5);
        let max_x = simple.vertices.iter().map(|v| v.x).fold(0., f32::max);
        assert!(max_x > 3.5, "se encogio: {max_x}");
        assert!(!simple.faces.is_empty());
    }

    #[test]
    fn both_sides_of_a_thin_surface_are_kept() {
        let vertices = [Vec3::ZERO, Vec3::X * 4., Vec3::Y * 4.];
        let faces = [[0, 1, 2], [0, 2, 1]];
        let simple = cluster(&vertices, &faces, &[0u8, 1u8], 0.5);
        assert_eq!(simple.faces.len(), 2);
    }
}
