use super::{draw, line, ship_angle, sphere, Effect, View, World};
use crate::display::{Button, Key, Window};
use carga_modelos::{color::Color, framebuffer::Framebuffer};
use glam::{Mat4, Quat, Vec2, Vec3};
use std::{f32::consts::TAU, path::Path, time::Instant};

const SHIP_DEPTH: f32 = -5.8;
const BOUNDS: Vec2 = Vec2::new(4.65, 2.40);
const HULL_RADIUS: f32 = 0.49;

// Meteorito que avanza hacia la nave
struct Rock {
    position: Vec3,
    radius: f32,
    phase: f32,
    crossed: bool,
}

// Estado de una partida del Portal Verde
struct Run {
    ship: Vec2,
    time: f32,
    until_spawn: f32,
    shield: f32,
    lives: u8,
    dodged: u32,
    rng: u32,
    rocks: Vec<Rock>,
}

impl Run {
    // Empieza una partida con tres vidas
    fn new() -> Self {
        Self {
            ship: Vec2::new(0., -0.55),
            time: 0.,
            until_spawn: 0.38,
            shield: 0.,
            lives: 3,
            dodged: 0,
            rng: 181_374,
            rocks: Vec::with_capacity(36),
        }
    }

    // Numero pseudoaleatorio reproducible de 0 a 1
    fn random(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 8) as f32 / 16_777_216.
    }

    // Avanza la partida: mueve la nave crea meteoritos y detecta choques
    fn step(&mut self, dt: f32, movement: Vec2) {
        if self.lives == 0 {
            return;
        }
        let previous_ship = self.ship;
        self.ship = (self.ship + dt * 6.6 * movement.clamp_length_max(1.)).clamp(-BOUNDS, BOUNDS);
        self.time += dt;
        self.shield = (self.shield - dt).max(0.);
        self.until_spawn -= dt;
        if self.until_spawn <= 0. {
            let lane = Vec2::new(
                (self.random() * 2. - 1.) * 5.0,
                (self.random() * 2. - 1.) * 2.5,
            );
            let lane = if self.random() < 0.38 {
                self.ship.lerp(lane, 0.28)
            } else {
                lane
            };
            let radius = 0.35 + self.random() * 0.62;
            let phase = self.random() * TAU;
            self.rocks.push(Rock {
                position: lane.extend(-44.),
                radius,
                phase,
                crossed: false,
            });
            self.until_spawn += (0.74 - self.time * 0.011).max(0.27);
        }
        let advance = dt * (13. + self.time * 0.27).min(30.);
        for rock in &mut self.rocks {
            let old_z = rock.position.z;
            rock.position.z += advance;
            if !rock.crossed && old_z <= SHIP_DEPTH && rock.position.z >= SHIP_DEPTH {
                rock.crossed = true;
                let blend = (SHIP_DEPTH - old_z) / advance.max(1e-6);
                let at_crossing = previous_ship.lerp(self.ship, blend);
                let hit =
                    at_crossing.distance(rock.position.truncate()) < HULL_RADIUS + rock.radius;
                if hit {
                    if self.shield == 0. {
                        self.lives = self.lives.saturating_sub(1);
                        self.shield = 0.85;
                    }
                } else {
                    self.dodged += 1;
                }
            }
        }
        self.rocks.retain(|rock| rock.position.z < 1.);
    }
}

// Pinta un pixel sin prueba de profundidad
fn pixel(fb: &mut Framebuffer, x: usize, y: usize, color: Color) {
    let index = (y * fb.width + x) * 4;
    fb.pixels[index..index + 4].copy_from_slice(&[color.r, color.g, color.b, 255]);
}

// Brillo y color del portal: `r` vale 1 en el anillo y `a` es el angulo
pub(super) fn portal_color(r: f32, a: f32, time: f32) -> Color {
    let arms = (a * 4. + r * 17. - time * 6. + 0.51 * (a * 7. + r * 11.).sin()).sin();
    let halo = (1. - ((r - 0.91) / 0.17).abs()).max(0.);
    let mouth = ((r - 0.15) * 1.35).clamp(0., 1.);
    /// Brillo del portal
    let intensity = (halo * (0.42 + 0.58 * arms.max(0.)) + mouth * 0.19 * (1. + arms))
        * (1.17 - r * r).mul_add(9., 0.).clamp(0., 1.);
    Color::new(
        (5. + 92. * intensity) as u8,
        (17. + 229. * intensity) as u8,
        (17. + 111. * intensity) as u8,
    )
}

// Dibuja el portal en 2D de fondo en el minijuego
fn portal(fb: &mut Framebuffer, time: f32, radius: f32) {
    let cy = fb.height as f32 * 0.46;
    let cx = fb.width as f32 * 0.5;
    for y in 0..fb.height {
        let v = (y as f32 - cy) / radius;
        for x in 0..fb.width {
            let u = (x as f32 - cx) / (radius * 0.89);
            let d2 = u * u + v * v;
            if d2 > 1.17 {
                continue;
            }
            pixel(fb, x, y, portal_color(d2.sqrt(), v.atan2(u), time));
        }
    }
}

// Escribe texto con la fuente de pixeles propia
pub(super) fn caption(fb: &mut Framebuffer, text: &str, x: usize, y: usize, scale: usize, color: Color) {
    // Letras de 3 x 5 guardadas en los 15 bits bajos por filas
    for (i, ch) in text.chars().enumerate() {
        let pattern = match ch {
            '0' => 0x7b6f,
            '1' => 0x2c97,
            '2' => 0x73e7,
            '3' => 0x73cf,
            '4' => 0x5bc9,
            '5' => 0x79cf,
            '6' => 0x79ef,
            '7' => 0x7249,
            '8' => 0x7bef,
            '9' => 0x7bcf,
            'A' => 0x2bed,
            'B' => 0x6bae,
            'C' => 0x7927,
            'D' => 0x6b6e,
            'E' => 0x79e7,
            'F' => 0x79e4,
            'G' => 0x796f,
            'H' => 0x5bed,
            'I' => 0x7497,
            'J' => 0x126a,
            'K' => 0x5bad,
            'L' => 0x4927,
            'M' => 0x5fed,
            'N' => 0x7b6d,
            'O' => 0x2b6a,
            'P' => 0x6ba4,
            'Q' => 0x7b79,
            'R' => 0x6bad,
            'S' => 0x79cf,
            'T' => 0x7492,
            'U' => 0x5b6f,
            'V' => 0x5b6a,
            'W' => 0x5bfd,
            'X' => 0x5aad,
            'Y' => 0x5a92,
            'Z' => 0x72a7,
            '/' => 0x12a4,
            '+' => 0x05d0,
            '-' => 0x01c0,
            ':' => 0x0410,
            '.' => 0x0002,
            _ => 0,
        };
        for row in 0..5 {
            for col in 0..3 {
                if pattern & (1 << (14 - row * 3 - col)) == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = x + (i * 4 + col) * scale + dx;
                        let py = y + row * scale + dy;
                        if px < fb.width && py < fb.height {
                            pixel(fb, px, py, color);
                        }
                    }
                }
            }
        }
    }
}

// Escena del minijuego: camara fija y malla del meteorito
struct Arena<'a> {
    world: &'a World,
    rock: super::Mesh,
    view: View,
}

impl<'a> Arena<'a> {
    // Deforma una esfera para crear el meteorito
    fn new(world: &'a World, aspect: f32) -> Self {
        let base = sphere(
            1.,
            7,
            11,
            &[
                Color::new(123, 109, 128),
                Color::new(163, 143, 135),
                Color::new(101, 95, 120),
                Color::new(181, 150, 125),
            ],
        );
        let mut model = base.model;
        for point in &mut model.vertices {
            let variation = 0.86 + 0.14 * (point.x * 18. + point.y * 11. + point.z * 8.).cos();
            *point *= variation;
        }
        Self {
            world,
            rock: super::mesh(model, base.colors),
            view: View::new(Vec3::ZERO, Quat::IDENTITY, aspect),
        }
    }

    // Dibuja estelas portal meteoritos nave y marcador
    fn render(&self, fb: &mut Framebuffer, run: &Run, time: f32, entering: bool) {
        fb.clear();
        for i in 0..185 {
            let angle = i as f32 * 2.399963;
            let drift = (i as f32 * 0.0837 + time * if entering { 0.65 } else { 0.27 }).fract();
            let r = drift * drift * fb.width as f32 * 0.63;
            let direction = Vec3::new(angle.cos(), angle.sin() * 0.73, 0.);
            let at = Vec3::new(fb.width as f32 * 0.5, fb.height as f32 * 0.46, 0.999);
            fb.set_color(Color::new(
                (38. + drift * 64.) as u8,
                (80. + drift * 98.) as u8,
                99,
            ));
            line(
                fb,
                at + direction * r,
                at + direction * (r + 1. + drift * if entering { 34. } else { 11. }),
            );
        }
        let scale = if entering {
            0.14 + 0.89 * time * time
        } else {
            0.12
        };
        portal(fb, time, fb.height as f32 * scale);
        for rock in &run.rocks {
            let rotation = Quat::from_euler(
                glam::EulerRot::XYZ,
                rock.phase + run.time * 0.72,
                rock.phase * 0.41 + run.time * 0.42,
                rock.phase,
            );
            let matrix = Mat4::from_scale_rotation_translation(
                Vec3::splat(rock.radius),
                rotation,
                rock.position,
            );
            draw(fb, &self.view, &self.rock, matrix, Effect::Land, run.time);
        }
        let flicker = run.shield > 0. && (time * 15.) as u32 % 2 == 0;
        if !flicker {
            let rotation = ship_angle();
            let ship = Mat4::from_scale_rotation_translation(
                Vec3::splat(0.32),
                rotation,
                run.ship.extend(SHIP_DEPTH),
            );
            draw(fb, &self.view, &self.world.ship, ship, Effect::Ship, time);
            draw(
                fb,
                &self.view,
                &self.world.canopy,
                ship,
                Effect::Glass,
                time,
            );
        }
        let px = (fb.width / 480).max(1);
        let tint = Color::new(150, 244, 201);
        caption(fb, "PORTAL VERDE", 14 * px, 13 * px, px, tint);
        caption(
            fb,
            &format!("ESQUIVADOS {}", run.dodged),
            14 * px,
            24 * px,
            px,
            tint,
        );
        caption(
            fb,
            &format!("VIDAS {}", run.lives),
            14 * px,
            35 * px,
            px,
            tint,
        );
        if run.lives == 0 {
            caption(
                fb,
                "FIN DEL VUELO",
                fb.width / 2 - 26 * px,
                fb.height / 2 - 9 * px,
                px,
                Color::new(255, 153, 129),
            );
            caption(
                fb,
                "R REINICIA",
                fb.width / 2 - 20 * px,
                fb.height / 2 + 3 * px,
                px,
                tint,
            );
        }
    }
}

// Bucle del minijuego; devuelve false si se cerro la ventana
pub fn play(
    window: &Window,
    fb: &mut Framebuffer,
    world: &World,
    once: bool,
) -> Result<bool, String> {
    let arena = Arena::new(world, fb.width as f32 / fb.height as f32);
    let mut run = Run::new();
    let mut best = 0;
    let mut previous = Instant::now();
    let mut phase = 0.;
    window.play_portal_music();
    // La pulsacion que abrio el portal sigue activa durante el primer cuadro
    let mut armed = false;
    while window.poll() {
        if armed && (window.pressed(Key::P) || window.button_pressed(Button::Share)) {
            return Ok(true);
        }
        armed = true;
        let now = Instant::now();
        let dt = (now - previous).as_secs_f32().min(0.05);
        previous = now;
        phase += dt;
        let key = |a, b| i32::from(window.key(a) || window.key(b)) as f32;
        let sticks = window.sticks();
        let steer = Vec2::new(
            key(Key::D, Key::Right) - key(Key::A, Key::Left) + sticks.left_x,
            key(Key::W, Key::Up) - key(Key::S, Key::Down) - sticks.left_y,
        );
        if phase >= 1.1 {
            let lives = run.lives;
            run.step(dt, steer);
            if run.lives < lives {
                window.play_life_lost(run.lives == 0);
            }
        }
        best = best.max(run.dodged);
        if run.lives == 0 && (window.key(Key::R) || window.button(Button::Cross)) {
            run = Run::new();
            phase = 0.;
        }
        arena.render(fb, &run, phase, phase < 1.1);
        window.present(&fb.pixels);
        window.title(&format!("PORTAL VERDE | {} esquivados | {} vidas | Mejor {} | WASD / flechas / stick | R o X reinicia | P o Share vuelve a Galaxia",
                              run.dodged,run.lives,best));
        if once {
            return Ok(true);
        }
    }
    Ok(false)
}

// Guarda una imagen de ejemplo del minijuego
pub fn capture(fb: &mut Framebuffer, world: &World) -> Result<(), String> {
    let mut run = Run::new();
    for frame in 0..360 {
        run.step(1. / 60., Vec2::new((frame as f32 / 75.).cos() * 0.33, 0.));
    }
    Arena::new(world, fb.width as f32 / fb.height as f32).render(fb, &run, 6., false);
    fb.save_png(Path::new("../Media/Portal_Verde.png"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crossing_counts_once_and_collision_costs_a_life() {
        let mut run = Run::new();
        run.rocks.push(Rock {
            position: run.ship.extend(SHIP_DEPTH - 0.1),
            radius: 0.4,
            phase: 0.,
            crossed: false,
        });
        run.rocks.push(Rock {
            position: Vec3::new(4., 2., SHIP_DEPTH - 0.1),
            radius: 0.4,
            phase: 0.,
            crossed: false,
        });
        run.step(0.04, Vec2::ZERO);
        assert_eq!((run.lives, run.dodged), (2, 1));
        run.step(0.04, Vec2::ZERO);
        assert_eq!((run.lives, run.dodged), (2, 1));
    }
}
