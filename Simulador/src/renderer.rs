use crate::{color::SHIP_COLOR, framebuffer::Framebuffer, obj_loader::Model, triangle::triangle};
use glam::{Mat4, Vec3};

// Modelo -> mundo -> vista -> proyeccion -> NDC -> pixeles
pub fn prepare_vertices(model: &Model, width: usize, height: usize) -> Vec<Vec3> {
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(-f32::MAX);
    for &v in &model.vertices {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    let center = (lo + hi) * 0.5;
    let model_matrix = Mat4::from_translation(-center);
    // El OBJ exportado por Blender tiene Y como eje vertical
    let view_matrix = Mat4::look_at_rh(Vec3::new(8.0, 7.0, 11.0), Vec3::ZERO, Vec3::Y);
    let vm = view_matrix * model_matrix;
    let mut view_lo = Vec3::splat(f32::MAX);
    let mut view_hi = Vec3::splat(-f32::MAX);
    for &v in &model.vertices {
        let p = (vm * v.extend(1.0)).truncate();
        view_lo = view_lo.min(p);
        view_hi = view_hi.max(p);
    }
    // Encuadre ortografico automatico: margen proporciones y centro de pantalla
    let aspect = width as f32 / height as f32;
    let half_height = ((view_hi.y - view_lo.y) * 0.5)
        .max((view_hi.x - view_lo.x) * 0.5 / aspect)
        .max(0.001)
        / 0.78;
    let half_width = half_height * aspect;
    let mid = (view_lo + view_hi) * 0.5;
    let projection = Mat4::orthographic_rh_gl(
        mid.x - half_width,
        mid.x + half_width,
        mid.y - half_height,
        mid.y + half_height,
        0.1,
        100.0,
    );

    model
        .vertices
        .iter()
        .map(|&v| {
            let clip = projection * vm * v.extend(1.0);
            let ndc = clip.truncate() / clip.w;
            Vec3::new(
                (ndc.x + 1.0) * 0.5 * width as f32,
                (1.0 - ndc.y) * 0.5 * height as f32,
                (ndc.z + 1.0) * 0.5,
            )
        })
        .collect()
}

// Dibuja un modelo con luz plana y color fijo
pub fn render(model: &Model, screen_vertices: &[Vec3], buffer: &mut Framebuffer) -> usize {
    let light = Vec3::new(-0.4, 0.8, 0.7).normalize();
    // Recorrer MANUALMENTE cada cara y buscar sus tres vertices por indice
    for face in &model.faces {
        let index_a = face[0];
        let index_b = face[1];
        let index_c = face[2];
        let a = screen_vertices[index_a];
        let b = screen_vertices[index_b];
        let c = screen_vertices[index_c];

        // Color base simple e iluminacion plana para distinguir las superficies
        let cross = (model.vertices[index_b] - model.vertices[index_a])
            .cross(model.vertices[index_c] - model.vertices[index_a]);
        let length = cross.length();
        let intensity = if length > 1e-8 {
            0.3 + 0.7 * (cross / length).dot(light).max(0.0)
        } else {
            0.3
        };
        buffer.set_color(SHIP_COLOR.shade(intensity));
        triangle(buffer, a, b, c); // Funcion propia en Rust no una libreria para dibujar triangulos
    }
    model.faces.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{color::BACKGROUND, obj_loader::parse_obj};

    #[test]
    fn full_ship_is_centered_and_inside_screen() {
        let model = parse_obj(include_str!("../assets/Nave_Espacial.obj")).unwrap();
        assert_eq!(model.vertices.len(), 84);
        assert_eq!(model.faces.len(), 164);
        let mut fb = Framebuffer::new(1000, 750);
        let vertices = prepare_vertices(&model, fb.width, fb.height);
        assert_eq!(render(&model, &vertices, &mut fb), 164);
        let mut bounds = (usize::MAX, usize::MAX, 0, 0);
        for (index, pixel) in fb.pixels.chunks_exact(4).enumerate() {
            if pixel != [BACKGROUND.r, BACKGROUND.g, BACKGROUND.b, 255] {
                let (x, y) = (index % fb.width, index / fb.width);
                bounds.0 = bounds.0.min(x);
                bounds.1 = bounds.1.min(y);
                bounds.2 = bounds.2.max(x);
                bounds.3 = bounds.3.max(y);
            }
        }
        assert!(bounds.0 > 0 && bounds.1 > 0 && bounds.2 < 999 && bounds.3 < 749);
        assert!(((bounds.0 + bounds.2) as f32 * 0.5 - 500.0).abs() < 2.0);
        assert!(((bounds.1 + bounds.3) as f32 * 0.5 - 375.0).abs() < 2.0);
        assert!(bounds.2 - bounds.0 > 700);
    }
}
