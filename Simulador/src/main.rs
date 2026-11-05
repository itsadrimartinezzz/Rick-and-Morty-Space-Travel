// El juego se abre sin ventana de consola
#![cfg_attr(not(test), windows_subsystem = "windows")]
#![allow(unused_doc_comments)]
use carga_modelos::{
    color::Color,
    framebuffer::Framebuffer,
    lod,
    normal_map,
    obj_loader::{parse_obj, Model},
    ruido, sonido,
    triangle::{triangle_glass, triangle_shaded},
};
use glam::{Mat4, Quat, Vec3};
mod display;
mod meteoritos;
mod vuelo;
use display::{Button, Key, Window};
use std::{f32::consts::TAU, io::Write, path::PathBuf, time::Instant};
use vuelo::{Flight, Input};

const NAMES: [&str; 5] = [
    "Resort World",
    "Bird World",
    "Venzenulon 7",
    "Alphabetrium",
    "Unity's Planet",
];
const FILES: [&str; 5] = [
    "Resort_World",
    "Bird_World",
    "Venzenulon_7",
    "Alphabetrium",
    "Unitys_Planet",
];
// --- Parametros ---

/// Radio de cada mundo
pub const PLANET_SCALE: [f32; 5] = [2.4, 2.9, 2.2, 3.2, 2.6];
/// Tamano del sol
pub const SUN_SCALE: f32 = 2.2;
/// Distancia de cada mundo al sol
pub const ORBIT_RADII: [f32; 5] = [31., 39., 33., 43., 36.];
/// Angulo inicial de cada mundo
const PHASE: [f32; 5] = [0.30, 1.557, 2.813, 4.07, 5.327];
/// Velocidad orbital en radianes por segundo
pub const ORBIT_SPEED: f32 = 0.12;
/// Intensidad del mapeo de normales y 0 lo apaga
const NORMAL_STRENGTH: f32 = 1.;
// Veces que se repite el mapa de normales por unidad del modelo
const NORMAL_TILING: f32 = 0.85;
// El relieve fino se desvanece hasta esta distancia donde ya no se distingue
const NORMAL_FADE: f32 = 70.;
/// Fuerza de la gravedad
const SURFACE_GRAVITY: f32 = 8.;
/// Posicion del portal
pub const PORTAL_CENTER: Vec3 = Vec3::new(55., 165., 0.);
/// Tamano del portal
pub const PORTAL_RADIUS: f32 = 16.;
// Espacio que deja la colision entre la camara y la superficie; incluye la nave
const SURFACE_MARGIN: f32 = 4.4;
// Tamano de la celda de cada nivel de detalle en unidades del modelo
const LOD_CELLS: [f32; 3] = [0.03, 0.08, 0.18];
// Se cambia de nivel cuando una celda mide menos de estos pixeles en pantalla
const LOD_PIXEL_ERROR: f32 = 0.5;
// Cancion del sistema solar relativa a la carpeta Simulador
const MUSIC_FILE: &str = "assets/audio/Rick and Morty Theme.mp3";
// Cancion de adentro del Portal Verde
const PORTAL_MUSIC_FILE: &str =
    "assets/audio/Rick's Backstory Music - Rick & Morty Unreleased Music.mp3";
// Volumen de la musica de 0 a 1
const MUSIC_VOLUME: f32 = 0.7;
const NEAR: f32 = 1.;
const FAR: f32 = 6000.;
const FOV: f32 = 1.05;
const W: usize = 960;
const H: usize = 540;

// Malla lista para dibujar con color y normal por cara
struct Mesh {
    model: Model,
    colors: Vec<Color>,
    normals: Vec<Vec3>,
    radius: f32,
}

// Calcula normales por cara y radio de una malla
fn mesh(model: Model, colors: Vec<Color>) -> Mesh {
    let radius = model.vertices.iter().map(|v| v.length()).fold(0., f32::max);
    let normals = model
        .faces
        .iter()
        .map(|&[a, b, c]| {
            let v = &model.vertices;
            (v[b] - v[a]).cross(v[c] - v[a]).normalize_or_zero()
        })
        .collect();
    Mesh {
        model,
        colors,
        normals,
        radius,
    }
}

// Carga un OBJ de assets con el color de cada cara
fn load(name: &str) -> Result<Mesh, String> {
    let path = format!("assets/{name}.obj");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    let model = parse_obj(&text)?;
    let mut color = Color::new(170, 180, 185);
    let mut colors = Vec::with_capacity(model.faces.len());
    for line in text.lines() {
        if let Some(s) = line.strip_prefix("usemtl ") {
            let rgb = u32::from_str_radix(s, 16).map_err(|_| format!("Material invalido: {s}"))?;
            color = Color::new((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8);
        } else if line.starts_with("f ") {
            colors.push(color);
        }
    }
    Ok(mesh(model, colors))
}

// Genera una esfera; es la base de los meteoritos
fn sphere(r: f32, rings: usize, sectors: usize, palette: &[Color]) -> Mesh {
    let mut model = Model {
        vertices: vec![],
        faces: vec![],
    };
    let mut colors = vec![];
    for row in 0..=rings {
        let a = std::f32::consts::PI * row as f32 / rings as f32;
        for col in 0..sectors {
            let b = TAU * col as f32 / sectors as f32;
            model
                .vertices
                .push(r * Vec3::new(a.sin() * b.cos(), a.sin() * b.sin(), a.cos()));
        }
    }
    for row in 0..rings {
        for col in 0..sectors {
            let a = row * sectors + col;
            let b = row * sectors + (col + 1) % sectors;
            let band =
                ((row as f32 * 1.51 + (col as f32 * 0.52).sin()).abs() as usize) % palette.len();
            for face in [[a, a + sectors, b], [b, a + sectors, b + sectors]] {
                model.faces.push(face);
                colors.push(palette[band]);
            }
        }
    }
    mesh(model, colors)
}

// Direccion hacia la que mira el portal: el sol
pub fn portal_normal() -> Vec3 {
    (-PORTAL_CENTER).normalize()
}

// Genera el disco del portal visible por las dos caras: radio 1 en el plano XY
fn disc(rings: usize, sectors: usize) -> Mesh {
    let mut model = Model {
        vertices: vec![Vec3::ZERO],
        faces: vec![],
    };
    for ring in 1..=rings {
        let r = ring as f32 / rings as f32;
        for sector in 0..sectors {
            let a = TAU * sector as f32 / sectors as f32;
            model.vertices.push(Vec3::new(r * a.cos(), r * a.sin(), 0.));
        }
    }
    let index = |ring: usize, sector: usize| 1 + (ring - 1) * sectors + sector % sectors;
    for sector in 0..sectors {
        model.faces.push([0, index(1, sector), index(1, sector + 1)]);
        for ring in 1..rings {
            let (a, b) = (index(ring, sector), index(ring, sector + 1));
            let (c, d) = (index(ring + 1, sector), index(ring + 1, sector + 1));
            model.faces.push([a, c, d]);
            model.faces.push([a, d, b]);
        }
    }
    let back: Vec<_> = model.faces.iter().map(|&[a, b, c]| [a, c, b]).collect();
    model.faces.extend(back);
    let colors = vec![Color::new(92, 244, 170); model.faces.len()];
    mesh(model, colors)
}

// Posicion de un mundo en su orbita segun el tiempo
pub fn center(i: usize, t: f32) -> Vec3 {
    let phase = PHASE[i] + t * ORBIT_SPEED;
    ORBIT_RADII[i] * Vec3::new(phase.cos(), phase.sin(), 0.08 * phase.sin())
}

// Camara: posicion orientacion y matrices de vista y proyeccion
struct View {
    eye: Vec3,
    orientation: Quat,
    matrix: Mat4,
    projection: Mat4,
    // Los mundos reciben la luz desde el sol; el Portal usa una luz fija
    sun_lit: bool,
    normal_strength: f32,
}
impl View {
    // Crea la camara desde su posicion y orientacion
    fn new(eye: Vec3, orientation: Quat, aspect: f32) -> Self {
        let matrix = Mat4::from_quat(orientation.conjugate()) * Mat4::from_translation(-eye);
        let projection = Mat4::perspective_rh_gl(FOV, aspect, NEAR, FAR);
        Self {
            eye,
            orientation,
            matrix,
            projection,
            sun_lit: false,
            normal_strength: NORMAL_STRENGTH,
        }
    }
    // Ilumina los mundos desde el sol
    fn with_sun(mut self) -> Self {
        self.sun_lit = true;
        self
    }
    // Enciende o apaga el mapeo de normales
    fn with_normals(mut self, on: bool) -> Self {
        self.normal_strength = if on { NORMAL_STRENGTH } else { 0. };
        self
    }
    // Pasa un punto de la camara a pixeles y profundidad
    fn project(&self, camera: Vec3, fb: &Framebuffer) -> Vec3 {
        let p = self.projection * camera.extend(1.);
        let q = p.truncate() / p.w;
        Vec3::new(
            (1. + q.x) * fb.width as f32 / 2.,
            (1. - q.y) * fb.height as f32 / 2.,
            (1. + q.z) / 2.,
        )
    }
}

// Shaders de fragmentos disponibles
#[derive(Clone, Copy)]
enum Effect {
    Land,
    Sun,
    Ship,
    Glass,
    Portal,
}

// Pipeline de una malla: descarte transformacion recorte y rasterizado
fn draw(
    fb: &mut Framebuffer,
    v: &View,
    item: &Mesh,
    transform: Mat4,
    effect: Effect,
    t: f32,
) -> usize {
    let model_view = v.matrix * transform;
    let p = model_view.transform_point3(Vec3::ZERO);
    let radius = item.radius * transform.x_axis.truncate().length();
    let tan = (FOV / 2.).tan();
    let aspect = fb.width as f32 / fb.height as f32;
    if p.z > radius - NEAR
        || p.z < -FAR - radius
        || p.y.abs() > -p.z * tan + radius * (1. + tan * tan).sqrt()
        || p.x.abs() > -p.z * tan * aspect + radius * (1. + (tan * aspect).powi(2)).sqrt()
    {
        return 0;
    }
    let points: Vec<_> = item
        .model
        .vertices
        .iter()
        .map(|&p| model_view.transform_point3(p))
        .collect();
    let screen: Vec<_> = points
        .iter()
        .map(|&p| {
            if p.z <= -NEAR {
                v.project(p, fb)
            } else {
                Vec3::ZERO
            }
        })
        .collect();
    let fixed_light = Vec3::new(-0.43, -0.54, 0.72).normalize();
    // Mezcla sol y luz cenital: el terreno mira a +Z y el sol esta en el plano orbital
    let light = if v.sun_lit && matches!(effect, Effect::Land) {
        (-transform.w_axis.truncate().normalize_or_zero() * 0.62 + Vec3::Z * 0.78).normalize()
    } else {
        fixed_light
    };
    let needs_object = matches!(effect, Effect::Land | Effect::Sun | Effect::Portal);
    let bump_map = normal_map::shared();
    let inverse = if needs_object {
        model_view.inverse()
    } else {
        Mat4::IDENTITY
    };
    let drift = Vec3::new(t * 0.17, -t * 0.11, t * 0.08);
    let mut submitted = 0;
    for (i, &face) in item.model.faces.iter().enumerate() {
        let [a, b, c] = face.map(|j| points[j]);
        if (b - a).cross(c - a).dot(a) >= 0. {
            continue;
        }
        let normal = transform
            .transform_vector3(item.normals[i])
            .normalize_or_zero();
        let illumination = 0.33 + 0.67 * normal.dot(light).max(0.);
        let color = item.colors[i];
        // Mapeo de normales: base tangente de la cara y luz expresada en esa base
        let distance = -(a.z + b.z + c.z) / 3.;
        let bump = if matches!(effect, Effect::Land) {
            v.normal_strength * (1.3 * (1. - distance / NORMAL_FADE)).clamp(0., 1.)
        } else {
            0.
        };
        let object_normal = item.normals[i];
        let abs = object_normal.abs();
        let (axis_u, axis_v) = if abs.z >= abs.x && abs.z >= abs.y {
            (Vec3::X, Vec3::Y)
        } else if abs.x >= abs.y {
            (Vec3::Y, Vec3::Z)
        } else {
            (Vec3::X, Vec3::Z)
        };
        let tangent_light = if bump > 0. {
            let tangent = (axis_u - object_normal * axis_u.dot(object_normal)).normalize_or_zero();
            let bitangent = (axis_v
                - object_normal * axis_v.dot(object_normal)
                - tangent * axis_v.dot(tangent))
            .normalize_or_zero();
            [
                transform.transform_vector3(tangent).normalize_or_zero().dot(light),
                transform.transform_vector3(bitangent).normalize_or_zero().dot(light),
                normal.dot(light),
            ]
        } else {
            [0.; 3]
        };
        let paint = |fb: &mut Framebuffer, camera: [Vec3; 3], projected: [Vec3; 3]| {
            let [sa, sb, sc] = projected;
            if matches!(effect, Effect::Glass) {
                let [ca, cb, cc] = camera;
                let normal = (cb - ca).cross(cc - ca).normalize_or_zero();
                let facing = normal
                    .dot(-((ca + cb + cc) / 3.).normalize_or_zero())
                    .clamp(0., 1.);
                /// Intensidad del shader de atmosfera
                let opacity = 0.065 + 0.50 * (1. - facing).powi(4);
                triangle_glass(fb, sa, sb, sc, color, opacity);
                return;
            }
            // Posicion en el objeto con correccion de perspectiva: pega el ruido a la superficie
            let object = camera.map(|p| {
                if needs_object {
                    inverse.transform_point3(p)
                } else {
                    p
                }
            });
            let inverse_depth = camera.map(|p| 1. / (-p.z).max(1e-5));
            let surface = |w: [f32; 3]| {
                let (k0, k1, k2) = (
                    w[0] * inverse_depth[0],
                    w[1] * inverse_depth[1],
                    w[2] * inverse_depth[2],
                );
                (object[0] * k0 + object[1] * k1 + object[2] * k2) / (k0 + k1 + k2)
            };
            let shader = |depth: f32, w: [f32; 3]| match effect {
                Effect::Sun => {
                    let p = surface(w);
                    let cells = ruido::fbm(p * 1.5 + drift, 3);
                    let granules = ruido::value(p * 7.5 - drift * 2.);
                    /// Intensidad del shader del sol
                    let intensity = (cells * 1.5 - 0.25 + granules * 0.3).clamp(0., 1.);
                    if color.g > 170 {
                        color.shade(0.87 + 0.13 * intensity)
                    } else {
                        Color::new(
                            255,
                            (92. + 150. * intensity) as u8,
                            (18. + 105. * intensity) as u8,
                        )
                    }
                }
                Effect::Glass => color,
                Effect::Ship => color.shade((illumination + 0.16).min(1.)),
                Effect::Land => {
                    let p = surface(w);
                    let haze = ((1. - (1. - depth) * 0.14 / NEAR - 0.98) * 11.).clamp(0., 0.18);
                    if bump > 0. {
                        let texel = bump_map.sample(
                            p.dot(axis_u) * NORMAL_TILING,
                            p.dot(axis_v) * NORMAL_TILING,
                        );
                        let lit = normal_map::diffuse(texel, bump, tangent_light).max(0.);
                        color.shade(0.33 + 0.67 * lit + haze)
                    } else {
                        let grain = ruido::value(p * 7.);
                        color.shade(illumination * (0.88 + grain * 0.24) + haze)
                    }
                }
                Effect::Portal => {
                    let p = surface(w);
                    meteoritos::portal_color(p.truncate().length(), p.y.atan2(p.x), t)
                }
            };
            triangle_shaded(fb, sa, sb, sc, &shader);
        };
        if [a, b, c].iter().all(|p| p.z <= -NEAR) {
            paint(fb, [a, b, c], face.map(|j| screen[j]));
            submitted += 1;
        } else {
            // Sutherland-Hodgman contra el plano cercano antes de dividir por w
            let mut clipped = [Vec3::ZERO; 4];
            let mut count = 0;
            let mut prev = c;
            for curr in [a, b, c] {
                let inside = curr.z <= -NEAR;
                if inside != (prev.z <= -NEAR) {
                    clipped[count] = prev.lerp(curr, (-NEAR - prev.z) / (curr.z - prev.z));
                    count += 1;
                }
                if inside {
                    clipped[count] = curr;
                    count += 1;
                }
                prev = curr;
            }
            for k in 1..count.saturating_sub(1) {
                let corners = [clipped[0], clipped[k], clipped[k + 1]];
                paint(fb, corners, corners.map(|p| v.project(p, fb)));
                submitted += 1;
            }
        }
    }
    submitted
}

// Dibuja una linea recortada a la pantalla con profundidad
fn line(fb: &mut Framebuffer, a: Vec3, b: Vec3) {
    let delta = b - a;
    let mut start: f32 = 0.;
    let mut end: f32 = 1.;
    for (p, q) in [
        (-delta.x, a.x),
        (delta.x, fb.width as f32 - 1. - a.x),
        (-delta.y, a.y),
        (delta.y, fb.height as f32 - 1. - a.y),
    ] {
        if p == 0. {
            if q < 0. {
                return;
            }
        } else if p < 0. {
            start = start.max(q / p);
        } else {
            end = end.min(q / p);
        }
        if start > end {
            return;
        }
    }
    let a = a + delta * start;
    let b = a + delta * (end - start);
    let steps = (b - a).truncate().length().min(1800.).ceil() as usize;
    for k in 0..=steps {
        let p = a.lerp(b, k as f32 / steps.max(1) as f32);
        fb.point(p.x as i32, p.y as i32, p.z);
    }
}

// Mallas de la escena: mundos sol nave estrellas y portal
pub struct World {
    planets: Vec<Mesh>,
    lod: Vec<[Mesh; LOD_CELLS.len()]>,
    atmospheres: Vec<Mesh>,
    sun: Mesh,
    ship: Mesh,
    canopy: Mesh,
    stars: Vec<Vec3>,
    portal: Mesh,
}

impl World {
    // Carga los modelos y genera niveles de detalle estrellas y portal
    pub fn new() -> Result<Self, String> {
        let planets: Vec<_> = FILES.iter().map(|s| load(s)).collect::<Result<_, _>>()?;
        let lod = planets
            .iter()
            .map(|full: &Mesh| {
                LOD_CELLS.map(|cell| {
                    let simple =
                        lod::cluster(&full.model.vertices, &full.model.faces, &full.colors, cell);
                    mesh(
                        Model {
                            vertices: simple.vertices,
                            faces: simple.faces,
                        },
                        simple.colors,
                    )
                })
            })
            .collect();
        let atmospheres = FILES
            .iter()
            .map(|s| load(&format!("{s}_vidrio")))
            .collect::<Result<_, _>>()?;
        let stars = (0u32..1300)
            .map(|i| {
                let z = 1. - (i as f32 + 0.5) * 2. / 1300.;
                let a = i as f32 * 2.3999631;
                let r = (1. - z * z).sqrt();
                270. * Vec3::new(r * a.cos(), r * a.sin(), z)
            })
            .collect();
        Ok(Self {
            planets,
            lod,
            atmospheres,
            sun: load("Sol")?,
            ship: load("Space_Cruiser")?,
            canopy: load("Space_Cruiser_vidrio")?,
            stars,
            portal: disc(10, 72),
        })
    }

    // Radio real de un mundo con su escala
    pub fn planet_radius(&self, i: usize) -> f32 {
        self.planets[i].radius * PLANET_SCALE[i]
    }

    // Radio real del sol con su escala
    pub fn sun_radius(&self) -> f32 {
        self.sun.radius * SUN_SCALE
    }

    // Radio de un mundo mas el margen de colision
    pub fn collision_radius_planet(&self, i: usize) -> f32 {
        self.planet_radius(i) + SURFACE_MARGIN
    }

    // Distancia desde la que una celda del nivel mide menos que el error permitido
    pub fn switch_distance(&self, i: usize, level: usize, height: usize) -> f32 {
        let cell = LOD_CELLS[level] * PLANET_SCALE[i];
        cell * height as f32 / (2. * (FOV / 2.).tan() * LOD_PIXEL_ERROR)
    }

    // Malla mas simple cuyo error no se alcanza a ver desde esta distancia
    fn detail(&self, i: usize, distance: f32, height: usize) -> &Mesh {
        (0..LOD_CELLS.len())
            .rev()
            .find(|&level| distance >= self.switch_distance(i, level, height))
            .map_or(&self.planets[i], |level| &self.lod[i][level])
    }

    // Matriz de modelo de un mundo: escala rotacion y orbita
    fn pose(&self, i: usize, t: f32) -> Mat4 {
        // Primero gira sobre su eje y despues se inclina: el eje queda fijo
        /// Rotacion sobre el propio eje
        let rotation = Quat::from_rotation_y(0.05 + i as f32 * 0.03)
            * Quat::from_rotation_z(t * (0.18 + i as f32 * 0.023));
        Mat4::from_scale_rotation_translation(Vec3::splat(PLANET_SCALE[i]), rotation, center(i, t))
    }

    // Indica si un tramo de vuelo atraviesa el disco del portal
    pub fn crosses_portal(&self, old: Vec3, next: Vec3) -> bool {
        let normal = portal_normal();
        let (before, after) = (
            (old - PORTAL_CENTER).dot(normal),
            (next - PORTAL_CENTER).dot(normal),
        );
        if before * after > 0. || before == after {
            return false;
        }
        let hit = old.lerp(next, before / (before - after));
        hit.distance(PORTAL_CENTER) < PORTAL_RADIUS
    }

    // Superficie mas cercana a un punto: el indice es `None` si es el sol
    pub fn nearest_surface(&self, eye: Vec3, t: f32) -> (Option<usize>, f32) {
        let mut best = (None, eye.length() - self.sun_radius());
        for i in 0..5 {
            let gap = eye.distance(center(i, t)) - self.planet_radius(i);
            if gap < best.1 {
                best = (Some(i), gap);
            }
        }
        best
    }

    // Dibuja el cuadro: estrellas orbitas sol portal mundos nave y vidrios
    fn render(
        &self,
        fb: &mut Framebuffer,
        view: &View,
        t: f32,
        warp: f32,
        show_orbits: bool,
    ) -> usize {
        fb.clear();
        for (i, &star) in self.stars.iter().enumerate() {
            let p = view.matrix.transform_vector3(star);
            if p.z > -10. {
                continue;
            }
            // Las estrellas siempre quedan al fondo
            let mut a = view.project(p, fb);
            a.z = 1.;
            if i % 7 == 0 {
                fb.set_color(Color::new(151, 195, 221));
            } else {
                fb.set_color(Color::new(80, 106, 143));
            }
            let offset = (a - Vec3::new(fb.width as f32 / 2., fb.height as f32 / 2., a.z)) * warp;
            line(fb, a, a + offset);
        }
        if show_orbits {
            fb.set_color(Color::new(40, 64, 88));
            for &r in &ORBIT_RADII {
                for i in 0..128 {
                    let point = |j: usize| {
                        let a = j as f32 * TAU / 128.;
                        view.matrix
                            .transform_point3(r * Vec3::new(a.cos(), a.sin(), 0.08 * a.sin()))
                    };
                    let (a, b) = (point(i), point(i + 1));
                    if a.z < -NEAR && b.z < -NEAR {
                        line(fb, view.project(a, fb), view.project(b, fb));
                    }
                }
            }
        }
        let mut count = draw(
            fb,
            view,
            &self.sun,
            Mat4::from_scale(Vec3::splat(SUN_SCALE)),
            Effect::Sun,
            t,
        );
        count += draw(
            fb,
            view,
            &self.portal,
            Mat4::from_scale_rotation_translation(
                Vec3::splat(PORTAL_RADIUS),
                Quat::from_rotation_arc(Vec3::Z, portal_normal()),
                PORTAL_CENTER,
            ),
            Effect::Portal,
            t,
        );
        let poses = std::array::from_fn::<_, 5, _>(|i| self.pose(i, t));
        for (i, &pose) in poses.iter().enumerate() {
            let p = pose.w_axis.truncate();
            let mesh = self.detail(i, view.eye.distance(p), fb.height);
            count += draw(fb, view, mesh, pose, Effect::Land, t);
        }
        let mut order = [0, 1, 2, 3, 4];
        order.sort_by(|&a, &b| {
            view.matrix
                .transform_point3(poses[a].w_axis.truncate())
                .z
                .total_cmp(&view.matrix.transform_point3(poses[b].w_axis.truncate()).z)
        });
        // La nave sigue a la camara y comparte la prueba de profundidad
        let local = Vec3::new(0., -1.02, -3.5);
        let body = view.orientation * ship_angle();
        let transform = Mat4::from_scale_rotation_translation(
            Vec3::splat(0.32),
            body,
            view.eye + view.orientation * local,
        );
        count += draw(fb, view, &self.ship, transform, Effect::Ship, t);
        for i in order {
            count += draw(fb, view, &self.atmospheres[i], poses[i], Effect::Glass, t);
        }
        count + draw(fb, view, &self.canopy, transform, Effect::Glass, t)
    }

}

// Inclinacion fija de la nave respecto a la camara
fn ship_angle() -> Quat {
    // Inclina la cabina hacia la camara para que se vea la tripulacion
    Quat::from_rotation_x(-1.18) * Quat::from_rotation_z(-0.30)
}

// Orientacion que mira hacia una direccion con Z arriba
pub fn look(direction: Vec3) -> Quat {
    Quat::from_mat4(&Mat4::look_to_rh(Vec3::ZERO, direction, Vec3::Z).inverse())
}

// Efecto de warp sobre la imagen final: anillos que corren hacia el centro
fn warp_tunnel(fb: &mut Framebuffer, t: f32, amount: f32) {
    if amount < 0.02 {
        return;
    }
    let (cx, cy) = (fb.width as f32 * 0.5, fb.height as f32 * 0.5);
    let scale = 1. / cy;
    for y in 0..fb.height {
        for x in 0..fb.width {
            let (dx, dy) = ((x as f32 - cx) * scale, (y as f32 - cy) * scale);
            let r = (dx * dx + dy * dy).sqrt();
            let edge = ((r - 0.3) / 0.9).clamp(0., 1.);
            if edge == 0. {
                continue;
            }
            let wave = (r * 22. + t * 26. + dy.atan2(dx) * 5.).sin() * 0.5 + 0.5;
            /// Intensidad del tunel de warp
            let mix = amount * edge * (0.25 + 0.6 * wave);
            let index = (y * fb.width + x) * 4;
            for (pixel, tint) in fb.pixels[index..index + 3].iter_mut().zip([92., 244., 214.]) {
                *pixel = (*pixel as f32 * (1. - mix) + tint * mix) as u8;
            }
        }
    }
}

// Datos del texto en pantalla
struct Hud {
    triangles: usize,
    fps: f32,
    normals: bool,
}

// Escribe destino modo distancia triangulos y FPS
fn hud(fb: &mut Framebuffer, flight: &Flight, info: &Hud) {
    let tint = Color::new(170, 236, 225);
    let dim = Color::new(120, 160, 178);
    let px = (fb.width / 480).max(1);
    let (name, distance) = flight.target();
    let top = format!(
        "{} - {} - {:.0} U",
        name.to_uppercase().replace('\'', ""),
        flight.mode_name(),
        distance
    );
    meteoritos::caption(fb, &top, 8 * px, 8 * px, px, tint);
    let gravity = if flight.gravity {
        format!("GRAVEDAD {:.1}", flight.gravity_now)
    } else {
        "GRAVEDAD NO".to_string()
    };
    let stats = format!(
        "{} TRI - {:.0} FPS - {} - NORMALES {}",
        info.triangles,
        info.fps,
        gravity,
        if info.normals { "SI" } else { "NO" }
    );
    meteoritos::caption(fb, &stats, 8 * px, 16 * px, px, dim);
    if flight.time < 6. {
        let hint = "OPTIONS O H - MENU";
        let y = fb.height / px - 14;
        meteoritos::caption(fb, hint, 8 * px, y * px, px, dim);
    }
}

// Filas del menu: accion control de PlayStation y teclado
const MENU: [(&str, &[[&str; 3]]); 4] = [
    (
        "VUELO LIBRE",
        &[
            ["MOVERSE", "STICK IZQUIERDO", "W A S D"],
            ["MIRAR", "STICK DERECHO", "FLECHAS"],
            ["SUBIR / BAJAR", "R1 / L1", "E / Q"],
            ["TURBO", "R2", "SHIFT"],
            ["FRENAR Y FLOTAR", "L2", "ESPACIO"],
            ["ALABEO", "-", "Z / X"],
        ],
    ),
    (
        "ORBITA ALREDEDOR DE UN MUNDO",
        &[
            ["RODEAR EL MUNDO", "STICK IZQ O DER", "A / D"],
            ["ACERCARSE / ALEJARSE", "STICK IZQUIERDO", "W / S"],
            ["SUBIR / BAJAR", "STICK DER O R1 / L1", "E / Q"],
            ["VER DESDE ARRIBA", "TRIANGULO", "T"],
        ],
    ),
    (
        "EN CUALQUIER MODO",
        &[
            ["ELEGIR MUNDO", "CRUCETA", "1 A 5"],
            ["WARP AL MUNDO ELEGIDO", "CUADRADO", "F"],
            ["ENTRAR / SALIR DE ORBITA", "X", "TAB"],
            ["MOSTRAR ORBITAS", "CIRCULO", "O"],
            ["MAPEO DE NORMALES", "L3", "N"],
            ["GRAVEDAD", "R3", "G"],
            ["ABRIR / CERRAR ESTE MENU", "OPTIONS", "H"],
            ["CAPTURA / SALIR", "-", "F2 / ESC"],
        ],
    ),
    (
        "PORTAL VERDE",
        &[
            ["ENTRAR", "VUELA HACIA EL ANILLO VERDE DEL FONDO", ""],
            ["ESQUIVAR METEORITOS", "STICK IZQUIERDO", "W A S D"],
            ["REINICIAR AL PERDER", "X", "R"],
            ["VOLVER A LA GALAXIA", "SHARE", "P"],
        ],
    ),
];

// Mezcla un rectangulo de color sobre la imagen
fn fill(fb: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize, color: Color, mix: f32) {
    for row in y..(y + h).min(fb.height) {
        for column in x..(x + w).min(fb.width) {
            let index = (row * fb.width + column) * 4;
            for (pixel, tint) in fb.pixels[index..index + 3]
                .iter_mut()
                .zip([color.r, color.g, color.b])
            {
                *pixel = (*pixel as f32 * (1. - mix) + tint as f32 * mix) as u8;
            }
        }
    }
}

// Ventana de instrucciones dibujada sobre la escena en pausa
fn menu(fb: &mut Framebuffer) {
    let px = (fb.width / 480).max(1);
    let (width, height) = (fb.width, fb.height);
    fill(fb, 0, 0, width, height, Color::new(6, 10, 18), 0.72);
    let title = Color::new(170, 236, 225);
    let heading = Color::new(120, 200, 190);
    let text = Color::new(205, 222, 230);
    let (left, control, keyboard) = (24 * px, 180 * px, 330 * px);
    meteoritos::caption(fb, "INSTRUCCIONES", left, 8 * px, px * 2, title);
    let mut y = 24 * px;
    for (section, rows) in MENU {
        let box_height = (rows.len() + 1) * 8 * px + 3 * px;
        fill(fb, 16 * px, y, 448 * px, box_height, Color::new(30, 52, 66), 0.55);
        fill(fb, 16 * px, y, 448 * px, 9 * px, Color::new(38, 84, 88), 0.8);
        meteoritos::caption(fb, section, left, y + 2 * px, px, title);
        meteoritos::caption(fb, "CONTROL", control, y + 2 * px, px, heading);
        meteoritos::caption(fb, "TECLADO", keyboard, y + 2 * px, px, heading);
        y += 10 * px;
        for row in rows {
            meteoritos::caption(fb, row[0], left, y, px, text);
            meteoritos::caption(fb, row[1], control, y, px, text);
            meteoritos::caption(fb, row[2], keyboard, y, px, text);
            y += 8 * px;
        }
        y += 4 * px;
    }
}

// Dibuja la escena y el efecto de warp de un cuadro
fn frame(
    world: &World,
    fb: &mut Framebuffer,
    flight: &Flight,
    show_orbits: bool,
    normals: bool,
) -> usize {
    let view = View::new(flight.eye, flight.orientation, fb.width as f32 / fb.height as f32)
        .with_sun()
        .with_normals(normals);
    let triangles = world.render(fb, &view, flight.time, flight.streaks, show_orbits);
    warp_tunnel(fb, flight.time, flight.tunnel * 0.8);
    triangles
}

// El video usa la misma fisica que el juego: solo cambia quien da las entradas
fn demo_input(t: f32, previous: f32) -> Input {
    let fired = |at: f32| previous < at && t >= at;
    let mut input = Input::default();
    match t {
        t if t < 1.2 => {
            input.forward = 1.;
            input.boost = 0.4;
        }
        t if (5.6..7.6).contains(&t) => input.forward = 0.8,
        t if (7.6..13.5).contains(&t) => input.strafe = 1.,
        t if (17.5..20.).contains(&t) => input.forward = -1.,
        t if (22.5..26.).contains(&t) => input.strafe = -1.,
        t if t >= 26.2 => {
            input.forward = -1.;
            input.boost = 0.3;
            input.lift = 0.5;
        }
        _ => {}
    }
    input.warp = fired(3.);
    input.top_view = fired(13.5) || fired(16.);
    if fired(20.) {
        input.select = Some(3);
    }
    input.toggle_orbit = fired(26.);
    input
}

// Genera el video con FFmpeg a partir de un recorrido programado
fn demo(world: &World, fb: &mut Framebuffer) -> Result<(), String> {
    let mut encoder = std::process::Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-s",
            &format!("{}x{}", fb.width, fb.height),
            "-r",
            "30",
            "-i",
            "-",
            "-an",
            "-c:v",
            "libx264",
            "-preset",
            "fast",
            "-crf",
            "19",
            "-pix_fmt",
            "yuv420p",
            "../Media/Demostracion.mp4",
        ])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut stream = encoder.stdin.take().ok_or("ffmpeg sin entrada")?;
    let mut flight = Flight::new();
    let dt = 1. / 30.;
    for f in 0..(30 * 30) {
        let t = f as f32 / 30.;
        flight.step(world, &demo_input(t, t - dt), dt);
        let triangles = frame(world, fb, &flight, false, true);
        let info = Hud {
            triangles,
            fps: 30.,
            normals: true,
        };
        hud(fb, &flight, &info);
        stream.write_all(&fb.pixels).map_err(|e| e.to_string())?;
    }
    drop(stream);
    if !encoder.wait().map_err(|e| e.to_string())?.success() {
        return Err("Fallo ffmpeg".into());
    }
    Ok(())
}

// Junta teclado y control en una sola entrada
fn read_input(window: &Window) -> Input {
    let held = |a, b| window.key(a) as i32 as f32 - window.key(b) as i32 as f32;
    let pad = window.sticks();
    let pad_button = |b| window.button(b) as i32 as f32;
    let select = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5]
        .into_iter()
        .position(|k| window.pressed(k));
    Input {
        strafe: (held(Key::D, Key::A) + pad.left_x).clamp(-1., 1.),
        forward: (held(Key::W, Key::S) - pad.left_y).clamp(-1., 1.),
        lift: (held(Key::E, Key::Q) + pad_button(Button::R1) - pad_button(Button::L1))
            .clamp(-1., 1.),
        look_x: (held(Key::Right, Key::Left) + pad.right_x).clamp(-1., 1.),
        look_y: (held(Key::Up, Key::Down) - pad.right_y).clamp(-1., 1.),
        roll: held(Key::X, Key::Z),
        boost: if window.key(Key::LShift) || window.key(Key::RShift) {
            1.
        } else {
            pad.r2
        },
        brake: window.key(Key::Space) || pad.l2 > 0.5,
        toggle_orbit: window.pressed(Key::Tab) || window.button_pressed(Button::Cross),
        warp: window.pressed(Key::F) || window.button_pressed(Button::Square),
        top_view: window.pressed(Key::T) || window.button_pressed(Button::Triangle),
        select,
        cycle: window.button_pressed(Button::DpadRight) as i32
            - window.button_pressed(Button::DpadLeft) as i32,
    }
}

// Procesa las opciones y ejecuta el bucle principal del juego
fn run() -> Result<(), String> {
    if !std::path::Path::new("assets/Resort_World.obj").exists() {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let root = exe
            .ancestors()
            .find(|p| p.join("assets/Resort_World.obj").exists())
            .ok_or("No se encontro la carpeta assets")?;
        std::env::set_current_dir(root).map_err(|e| e.to_string())?;
    }
    let world = World::new()?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    let (w, h) = if args.iter().any(|a| a == "--4k") {
        (3840, 2160)
    } else if args.iter().any(|a| a == "--fluido") {
        (640, 360)
    } else {
        (W, H)
    };
    let mut fb = Framebuffer::new(w, h);
    if args.iter().any(|a| a == "--demo") {
        return demo(&world, &mut fb);
    }
    let mut flight = Flight::new();
    if args.iter().any(|a| a == "--captura") {
        frame(&world, &mut fb, &flight, false, true);
        return fb.save_png(&PathBuf::from("Captura.png"));
    }
    if args.iter().any(|a| a == "--captura-meteoritos") {
        return meteoritos::capture(&mut fb, &world);
    }
    let mut window = Window::new(w, h)?;
    if args.iter().any(|a| a == "--probar-control") {
        // Guarda durante 10 s lo que raylib lee del control
        let start = Instant::now();
        let mut lines = String::new();
        while window.poll() && start.elapsed().as_secs_f32() < 10. {
            window.present(&fb.pixels);
            let report = window.gamepad_report();
            if report.is_empty() {
                lines += "sin control\n";
            }
            for line in report {
                lines += &line;
                lines.push('\n');
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        return std::fs::write("control.txt", lines).map_err(|e| e.to_string());
    }
    let once = args.iter().any(|a| a == "--prueba-ventana");
    window.load_audio(
        MUSIC_FILE,
        PORTAL_MUSIC_FILE,
        MUSIC_VOLUME,
        &sonido::life_lost(),
        sonido::SAMPLE_RATE,
    );
    if args.iter().any(|a| a == "--meteoritos")
        && (!meteoritos::play(&window, &mut fb, &world, once)? || once)
    {
        return Ok(());
    }
    let mut last = Instant::now();
    window.play_solar_music();
    let mut show_orbits = false;
    let mut normals = true;
    let mut menu_open = false;
    let (mut fps, mut frames, mut window_time) = (0., 0u32, 0.);
    while window.poll() {
        if window.pressed(Key::H) || window.button_pressed(Button::Options) {
            menu_open = !menu_open;
        }
        if menu_open {
            // La simulacion queda en pausa mientras el menu esta abierto
            frame(&world, &mut fb, &flight, show_orbits, normals);
            menu(&mut fb);
            window.present(&fb.pixels);
            last = Instant::now();
            if once {
                break;
            }
            continue;
        }
        if window.pressed(Key::F2) {
            fb.save_png(&PathBuf::from("Captura.png"))?;
        }
        if window.pressed(Key::O) || window.button_pressed(Button::Circle) {
            show_orbits = !show_orbits;
        }
        if window.pressed(Key::N) || window.button_pressed(Button::L3) {
            normals = !normals;
        }
        if window.pressed(Key::G) || window.button_pressed(Button::R3) {
            flight.gravity = !flight.gravity;
        }
        let now = Instant::now();
        // Limite alto: por encima de 10 FPS todo sigue en tiempo real
        let dt = (now - last).as_secs_f32().min(0.1);
        last = now;
        flight.step(&world, &read_input(&window), dt);
        if flight.take_portal() {
            // Se entra volando; el portal se expande y empieza el juego de meteoritos
            if !meteoritos::play(&window, &mut fb, &world, once)? {
                break;
            }
            flight.leave_portal();
            window.play_solar_music();
            // Consume la pulsacion que cerro el portal
            window.present(&fb.pixels);
            last = Instant::now();
            continue;
        }
        let triangles = frame(&world, &mut fb, &flight, show_orbits, normals);
        frames += 1;
        window_time += dt;
        if window_time >= 0.25 {
            fps = frames as f32 / window_time;
            frames = 0;
            window_time = 0.;
            let (name, distance) = flight.target();
            window.title(&format!(
                "GALAXIA | {} | {} | {:.1} u | {} tri | {:.0} FPS",
                name,
                flight.mode_name(),
                distance,
                triangles,
                fps
            ));
        }
        let info = Hud {
            triangles,
            fps,
            normals,
        };
        hud(&mut fb, &flight, &info);
        window.present(&fb.pixels);
        if once {
            println!(
                "raylib listo | control: {}",
                window
                    .gamepad()
                    .map_or("no detectado".to_string(), |id| format!("#{id}"))
            );
            break;
        }
    }
    Ok(())
}

// Funciones de Windows para la consola y los avisos de error
#[cfg(windows)]
extern "system" {
    fn AttachConsole(process: u32) -> i32;
    fn MessageBoxW(window: *mut std::ffi::c_void, text: *const u16, title: *const u16, kind: u32)
        -> i32;
}

// Conecta las herramientas a la consola desde la que se abrieron
fn attach_console_for_tools() {
    let tool = std::env::args().skip(1).any(|a| {
        matches!(
            a.as_str(),
            "--probar-control" | "--prueba-ventana" | "--demo" | "--captura" | "--captura-meteoritos"
        )
    });
    #[cfg(windows)]
    if tool {
        const PARENT: u32 = u32::MAX;
        unsafe { AttachConsole(PARENT) };
    }
    #[cfg(not(windows))]
    let _ = tool;
}

// Sin consola los errores se muestran en un cuadro de aviso
fn show_error(message: &str) {
    eprintln!("Error: {message}");
    #[cfg(windows)]
    {
        let wide = |text: &str| text.encode_utf16().chain([0]).collect::<Vec<u16>>();
        const MB_ICONERROR: u32 = 0x10;
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                wide(message).as_ptr(),
                wide("Proyecto Galaxia").as_ptr(),
                MB_ICONERROR,
            );
        }
    }
}

// Punto de entrada: muestra el error si el juego no puede arrancar
fn main() {
    attach_console_for_tools();
    if let Err(error) = run() {
        show_error(&error);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod lod_tests {
    use super::*;

    // Foto de un mundo con su posicion fija; `t` solo cambia su rotacion
    fn shot(world: &World, i: usize, item: &Mesh, distance: f32, t: f32) -> Framebuffer {
        let target = center(i, 0.);
        let eye = target + Vec3::new(0.82, -0.3, 0.48).normalize() * distance;
        let view = View::new(eye, look(target - eye), W as f32 / H as f32).with_sun();
        let mut pose = world.pose(i, t);
        pose.w_axis = world.pose(i, 0.).w_axis;
        let mut fb = Framebuffer::new(W, H);
        fb.clear();
        draw(&mut fb, &view, item, pose, Effect::Land, t);
        fb
    }

    fn changed_pixels(a: &Framebuffer, b: &Framebuffer) -> usize {
        a.pixels
            .chunks_exact(4)
            .zip(b.pixels.chunks_exact(4))
            .filter(|(p, q)| p.iter().zip(q.iter()).any(|(x, y)| x.abs_diff(*y) > 40))
            .count()
    }

    // El cambio de nivel debe alterar menos que el mundo rotando medio segundo
    #[test]
    fn changing_level_of_detail_is_not_visible() {
        let world = World::new().unwrap();
        for i in 0..5 {
            let meshes: Vec<&Mesh> = std::iter::once(&world.planets[i])
                .chain(world.lod[i].iter())
                .collect();
            for level in 0..LOD_CELLS.len() {
                let distance = world.switch_distance(i, level, H) * 1.001;
                let before = shot(&world, i, meshes[level], distance, 0.);
                let after = shot(&world, i, meshes[level + 1], distance, 0.);
                let rotated = shot(&world, i, meshes[level], distance, 0.5);
                let switch = changed_pixels(&before, &after);
                let motion = changed_pixels(&before, &rotated);
                assert!(
                    switch < motion,
                    "{} nivel {level} a {distance:.0} u: {switch} px contra {motion} px",
                    NAMES[i]
                );
            }
        }
    }

    #[test]
    fn project_files_are_where_the_code_expects_them() {
        for file in [
            MUSIC_FILE,
            PORTAL_MUSIC_FILE,
            "../Media/Demostracion.mp4",
            "../Modelado/generar_blender.py",
            "assets/Resort_World.obj",
        ] {
            assert!(std::path::Path::new(file).exists(), "falta {file}");
        }
    }

    #[test]
    fn levels_really_reduce_the_work() {
        let world = World::new().unwrap();
        for i in 0..5 {
            let mut previous = world.planets[i].model.faces.len();
            for mesh in &world.lod[i] {
                assert!(mesh.model.faces.len() < previous);
                previous = mesh.model.faces.len();
            }
            assert!(previous * 3 < world.planets[i].model.faces.len());
        }
    }

    #[test]
    fn the_level_gets_coarser_with_distance_and_finer_with_resolution() {
        let world = World::new().unwrap();
        let faces = |d: f32, h: usize| world.detail(0, d, h).model.faces.len();
        assert!(faces(10., H) >= faces(150., H));
        assert!(faces(150., H) >= faces(800., H));
        assert!(faces(150., H * 4) >= faces(150., H));
    }

    #[test]
    fn worlds_are_still_drawn_very_far_away() {
        let world = World::new().unwrap();
        let eye = Vec3::new(0., -1500., 500.);
        let view = View::new(eye, look(-eye), W as f32 / H as f32).with_sun();
        let mut fb = Framebuffer::new(W, H);
        assert!(world.render(&mut fb, &view, 0., 0., false) > 0);
    }

    #[test]
    fn stars_never_cover_a_distant_world() {
        let world = World::new().unwrap();
        let eye = center(0, 0.) + Vec3::new(0., -400., 0.);
        let view = View::new(eye, look(center(0, 0.) - eye), W as f32 / H as f32).with_sun();
        let mut fb = Framebuffer::new(W, H);
        world.render(&mut fb, &view, 0., 0., false);
        let centre = (H / 2 * W + W / 2) * 4;
        assert_ne!(&fb.pixels[centre..centre + 3], &[80, 106, 143]);
    }
}
