use glam::Vec3;
use std::path::Path;

// Vertices y caras trianguladas de un modelo
pub struct Model {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<[usize; 3]>,
}

// Lee un archivo OBJ
pub fn load_obj(path: &Path) -> Result<Model, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("No se pudo leer {}: {e}", path.display()))?;
    parse_obj(&text)
}

// Interpreta el texto de un OBJ y valida sus indices
pub fn parse_obj(text: &str) -> Result<Model, String> {
    let mut model = Model {
        vertices: Vec::new(),
        faces: Vec::new(),
    };
    for (number, line) in text.lines().enumerate() {
        let mut fields = line.split('#').next().unwrap_or("").split_whitespace();
        let error = |message: &str| format!("OBJ linea {}: {message}", number + 1);
        match fields.next() {
            Some("v") => {
                let mut xyz = [0.0_f32; 3];
                for value in &mut xyz {
                    *value = fields
                        .next()
                        .ok_or_else(|| error("Vertice incompleto"))?
                        .parse::<f32>()
                        .map_err(|_| error("Coordenada invalida"))?;
                    if !value.is_finite() {
                        return Err(error("Coordenada no finita"));
                    }
                }
                model.vertices.push(Vec3::from_array(xyz));
            }
            Some("f") => {
                let tokens: Vec<_> = fields.collect();
                if tokens.len() != 3 {
                    return Err(error(
                        "El OBJ debe estar triangulado: tres indices por cara",
                    ));
                }
                let mut face = [0; 3];
                for (i, token) in tokens.iter().enumerate() {
                    // Acepta v v/vt v//vn y v/vt/vn: se usa el indice de posicion
                    let raw: isize = token
                        .split('/')
                        .next()
                        .unwrap_or("")
                        .parse()
                        .map_err(|_| error("Indice invalido"))?;
                    // OBJ empieza en 1: los negativos cuentan desde el ultimo vertice
                    let index = if raw > 0 {
                        raw - 1
                    } else {
                        model.vertices.len() as isize + raw
                    };
                    if raw == 0 || index < 0 || index as usize >= model.vertices.len() {
                        return Err(error("Indice fuera del arreglo de vertices"));
                    }
                    face[i] = index as usize;
                }
                model.faces.push(face);
            }
            _ => {} // No se necesitan texturas normales ni materiales externos
        }
    }
    if model.vertices.is_empty() || model.faces.is_empty() {
        return Err("El OBJ no contiene triangulos".into());
    }
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;
    const VERTICES: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\n";

    #[test]
    fn positive_negative_and_slash_indices_agree() {
        let model = parse_obj(&format!("{VERTICES}f 1/1/1 2//1 3/1/1\nf -3 -2 -1\n")).unwrap();
        assert_eq!(model.vertices.len(), 3);
        assert_eq!(model.faces, vec![[0, 1, 2], [0, 1, 2]]);
    }

    #[test]
    fn rejects_bad_indices_and_non_triangular_faces() {
        for bad in [
            "f 0 2 3",
            "f 1 2 4",
            "f -4 2 3",
            "f 1 2 3 1",
            "f 1 2",
            "f x 2 3",
        ] {
            assert!(parse_obj(&format!("{VERTICES}{bad}\n")).is_err(), "{bad}");
        }
        assert!(parse_obj("v NaN 0 0\nf 1 1 1").is_err());
        assert!(parse_obj("").is_err());
    }
}
