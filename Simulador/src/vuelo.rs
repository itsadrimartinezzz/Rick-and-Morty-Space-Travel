// Control de la nave: vuelo libre orbita local y warp sin depender de la ventana
use super::{center, look, portal_normal, World, NAMES, PORTAL_CENTER, PORTAL_RADIUS};
use glam::{EulerRot, Quat, Vec3};
use std::f32::consts::PI;

// Duracion del salto de warp en segundos
const WARP_TIME: f32 = 1.25;
// Elevacion maxima: casi cenital porque en el polo `look` no tiene arriba
const MAX_ELEVATION: f32 = 1.48;
const DEFAULT_ELEVATION: f32 = 0.42;
// Distancia a la superficie en la que un mundo pasa a ser el destino
const NEAR_RANGE: f32 = 30.;

// Entrada de un cuadro venga del teclado el control o el video
#[derive(Clone, Copy, Default, Debug)]
pub struct Input {
    // Izquierda(-) / derecha(+)
    pub strafe: f32,
    // Atras(-) / adelante(+)
    pub forward: f32,
    // Bajar(-) / subir(+)
    pub lift: f32,
    pub look_x: f32,
    pub look_y: f32,
    pub roll: f32,
    pub boost: f32,
    pub brake: bool,
    pub toggle_orbit: bool,
    pub warp: bool,
    pub top_view: bool,
    pub select: Option<usize>,
    pub cycle: i32,
}

// Modos de vuelo
#[derive(Clone, Copy, Debug)]
pub enum Mode {
    Free,
    Orbit {
        azimuth: f32,
        elevation: f32,
        distance: f32,
        goal_elevation: f32,
        goal_distance: f32,
        spin: f32,
    },
    Warp {
        from: Vec3,
        elapsed: f32,
        azimuth: f32,
    },
}

// Estado de la nave y la camara
pub struct Flight {
    pub eye: Vec3,
    pub orientation: Quat,
    pub velocity: Vec3,
    turn: Vec3,
    pub selected: usize,
    pub mode: Mode,
    pub time: f32,
    // Largo de las estelas de estrellas
    pub streaks: f32,
    // Intensidad del tunel de warp de 0 a 1
    pub tunnel: f32,
    // Se puede apagar para comparar la trayectoria con y sin atraccion
    pub gravity: bool,
    // Aceleracion gravitatoria actual para mostrarla en pantalla
    pub gravity_now: f32,
    entered_portal: bool,
    near_world: Option<usize>,
    near_sun: bool,
}

// Direccion a partir de azimut y elevacion
fn direction(azimuth: f32, elevation: f32) -> Vec3 {
    Vec3::new(
        elevation.cos() * azimuth.cos(),
        elevation.cos() * azimuth.sin(),
        elevation.sin(),
    )
}

// Factor de suavizado independiente de los FPS
fn smooth(rate: f32, dt: f32) -> f32 {
    1. - (-rate * dt).exp()
}

// Curva de entrada y salida suaves
fn ease(a: f32) -> f32 {
    let a = a.clamp(0., 1.);
    a * a * a * (a * (a * 6. - 15.) + 10.)
}

impl Flight {
    // Empieza en la vista general del sistema
    pub fn new() -> Self {
        // Vista general: el sol al centro y los cinco mundos alrededor
        let eye = Vec3::new(0., -78., 56.);
        Self {
            eye,
            orientation: look(-eye),
            velocity: Vec3::ZERO,
            turn: Vec3::ZERO,
            selected: 0,
            mode: Mode::Free,
            time: 0.,
            streaks: 0.,
            tunnel: 0.,
            gravity: true,
            gravity_now: 0.,
            entered_portal: false,
            near_world: None,
            near_sun: false,
        }
    }

    // Avisa una sola vez que la nave cruzo el portal
    pub fn take_portal(&mut self) -> bool {
        std::mem::take(&mut self.entered_portal)
    }

    // Al volver del Portal Verde la nave sale frente a el mirando hacia el sistema
    pub fn leave_portal(&mut self) {
        let out = portal_normal();
        self.eye = PORTAL_CENTER + out * (PORTAL_RADIUS + 26.);
        self.orientation = look(out);
        self.velocity = Vec3::ZERO;
        self.turn = Vec3::ZERO;
        self.mode = Mode::Free;
    }

    // Nombre que se muestra y distancia: el mundo al que la nave se acerca o el sol
    pub fn target(&self) -> (String, f32) {
        if self.near_sun {
            ("SOL".to_string(), self.eye.length())
        } else {
            (
                format!("{} {}", self.selected + 1, NAMES[self.selected]),
                self.distance_to_selected(),
            )
        }
    }

    // Distancia al centro del mundo elegido
    pub fn distance_to_selected(&self) -> f32 {
        self.eye.distance(center(self.selected, self.time))
    }

    // Nombre del modo para mostrar en pantalla
    pub fn mode_name(&self) -> &'static str {
        match self.mode {
            Mode::Free => "VUELO LIBRE",
            Mode::Orbit { .. } => "ORBITA",
            Mode::Warp { .. } => "WARP",
        }
    }

    // Fija la orbita desde la posicion actual o salta con warp si el mundo esta lejos
    fn lock_on(&mut self, world: &World) {
        let radius = world.planet_radius(self.selected);
        let target = center(self.selected, self.time);
        let offset = self.eye - target;
        if offset.length() > radius * 4.5 {
            self.begin_warp();
            return;
        }
        let distance = offset.length();
        let dir = offset / distance.max(1e-4);
        let elevation = dir.z.clamp(-1., 1.).asin();
        self.mode = Mode::Orbit {
            azimuth: dir.y.atan2(dir.x),
            elevation,
            distance,
            goal_elevation: elevation.clamp(-MAX_ELEVATION, MAX_ELEVATION),
            goal_distance: distance,
            spin: 0.,
        };
    }

    // Inicia el salto hacia el mundo elegido
    fn begin_warp(&mut self) {
        let target = center(self.selected, self.time);
        let toward = self.eye - target;
        self.mode = Mode::Warp {
            from: self.eye,
            elapsed: 0.,
            azimuth: toward.y.atan2(toward.x),
        };
        self.velocity = Vec3::ZERO;
    }

    // Avanza un cuadro: atiende la seleccion y ejecuta el modo actual
    pub fn step(&mut self, world: &World, input: &Input, dt: f32) {
        self.time += dt;
        let previous_selected = self.selected;
        if let Some(index) = input.select {
            self.selected = index;
        }
        self.selected = (self.selected as i32 + input.cycle).rem_euclid(5) as usize;
        let changed = self.selected != previous_selected;

        match self.mode {
            Mode::Free => {
                if input.toggle_orbit {
                    self.lock_on(world);
                } else if input.warp {
                    self.begin_warp();
                }
            }
            Mode::Orbit { .. } => {
                if input.toggle_orbit {
                    self.mode = Mode::Free;
                } else if changed || input.warp {
                    self.begin_warp();
                }
            }
            Mode::Warp { .. } => {
                if changed {
                    self.begin_warp();
                }
            }
        }

        match self.mode {
            Mode::Free => self.step_free(world, input, dt),
            Mode::Orbit { .. } => self.step_orbit(world, input, dt),
            Mode::Warp { .. } => self.step_warp(world, dt),
        }
    }

    // Vuelo libre con inercia gravedad y colision
    fn step_free(&mut self, world: &World, input: &Input, dt: f32) {
        // Giro progresivo: la velocidad angular persigue a la entrada
        let wanted = Vec3::new(input.look_y, input.look_x, input.roll) * 2.;
        self.turn = self.turn.lerp(wanted, smooth(8., dt));
        self.orientation = (Quat::from_rotation_z(-self.turn.y * dt)
            * self.orientation
            * Quat::from_euler(
                EulerRot::XYZ,
                self.turn.x * dt,
                0.,
                -self.turn.z * dt,
            ))
        .normalize();
        // Sin alabeo manual la nave vuelve sola a nivel para no desorientarse
        let front = self.orientation * Vec3::NEG_Z;
        if input.roll == 0. && front.z.abs() < 0.93 {
            self.orientation = self
                .orientation
                .slerp(look(front), smooth(1.6, dt))
                .normalize();
        }

        // Cerca de una superficie la nave va mas despacio
        let (near, gap) = world.nearest_surface(self.eye, self.time);
        // Cambia el destino solo al entrar en una zona para no pisar lo elegido a mano
        self.near_sun = near.is_none() && gap < NEAR_RANGE;
        match near {
            Some(index) if gap < NEAR_RANGE => {
                if self.near_world != Some(index) {
                    self.near_world = Some(index);
                    self.selected = index;
                }
            }
            _ => {
                if near.is_none() || gap > NEAR_RANGE + 10. {
                    self.near_world = None;
                }
            }
        }
        let caution = (gap / 22.).clamp(0.4, 1.);
        /// Velocidad de la nave
        let top = 32. * caution + 68. * input.boost * caution.max(0.6);
        // Subir siempre es hacia arriba del sistema mire donde mire la camara
        let wish = self.orientation * Vec3::new(input.strafe, 0., -input.forward)
            + Vec3::Z * input.lift;
        let goal = if input.brake {
            Vec3::ZERO
        } else {
            wish.clamp_length_max(1.) * top
        };
        // Con el motor la nave responde rapido; sin el conserva su inercia
        let response = if input.brake {
            6.
        } else if wish.length() > 0.01 {
            5.
        } else {
            0.9
        };
        self.velocity = self.velocity.lerp(goal, smooth(response, dt));
        // La gravedad dobla la trayectoria; al frenar los propulsores la contrarrestan
        let pull = if self.gravity {
            world.gravity_at(self.eye, self.time)
        } else {
            Vec3::ZERO
        };
        self.gravity_now = pull.length();
        if !input.brake {
            self.velocity += pull * dt;
        }

        // Cerca de un mundo la nave viaja con el y su orbita no la deja atras
        let carry = match near {
            Some(index) => {
                let blend = 1. - ((gap - 15.) / 30.).clamp(0., 1.);
                (center(index, self.time) - center(index, self.time - dt)) * blend
            }
            None => Vec3::ZERO,
        };
        let wanted_position = self.eye + self.velocity * dt + carry;
        let (next, normal) = world.safe_move(self.eye, wanted_position, self.time, dt);
        if world.crosses_portal(self.eye, next) {
            self.entered_portal = true;
        }
        // Deslizar: la nave rodea el obstaculo en vez de quedar pegada contra el
        if let Some(n) = normal {
            if goal.dot(n) < 0. {
                let mut slide = goal - n * goal.dot(n);
                let least = goal.length() * 0.5;
                if slide.length() < least {
                    let side = slide
                        .try_normalize()
                        .or_else(|| n.cross(Vec3::Z).try_normalize())
                        .unwrap_or(Vec3::X);
                    slide = side * least;
                }
                self.velocity = slide;
            } else if self.velocity.dot(n) < 0. {
                self.velocity -= n * self.velocity.dot(n);
            }
        }
        self.eye = next;
        let wanted_streaks = input.boost * 0.35;
        self.streaks += (wanted_streaks - self.streaks) * smooth(4., dt);
        self.tunnel += (0. - self.tunnel) * smooth(6., dt);
    }

    // Orbita local: rodea acerca y sube alrededor del mundo elegido
    fn step_orbit(&mut self, world: &World, input: &Input, dt: f32) {
        let Mode::Orbit {
            mut azimuth,
            mut elevation,
            mut distance,
            mut goal_elevation,
            mut goal_distance,
            mut spin,
        } = self.mode
        else {
            return;
        };
        let radius = world.planet_radius(self.selected);
        let closest = world.collision_radius_planet(self.selected) + 0.05;
        let farthest = radius * 5.;
        // A/D y los dos sticks rodean; W/S acerca y aleja; subir/bajar cambia la altura
        spin += ((input.strafe + input.look_x) * 1.35 - spin) * smooth(6., dt);
        azimuth += spin * dt;
        if input.top_view {
            goal_elevation = if goal_elevation > 1.2 {
                DEFAULT_ELEVATION
            } else {
                MAX_ELEVATION
            };
        }
        goal_elevation = (goal_elevation + (input.lift + input.look_y) * 1.5 * dt)
            .clamp(-MAX_ELEVATION, MAX_ELEVATION);
        goal_distance = (goal_distance - input.forward * (24. + 50. * input.boost) * dt)
            .clamp(closest, farthest.max(closest + 1.));
        elevation += (goal_elevation - elevation) * smooth(4., dt);
        distance += (goal_distance - distance) * smooth(5., dt);

        // La posicion sale de la distancia no de la velocidad: asi no hay espiral
        let target = center(self.selected, self.time);
        let wanted = target + direction(azimuth, elevation) * distance;
        let (next, _) = world.safe_move(self.eye, wanted, self.time, dt);
        self.eye = next;
        self.orientation = self
            .orientation
            .slerp(look(target - self.eye), smooth(6., dt))
            .normalize();
        self.velocity = Vec3::ZERO;
        self.turn = Vec3::ZERO;
        self.gravity_now = 0.;
        self.near_sun = false;
        self.near_world = Some(self.selected);
        self.streaks += (0. - self.streaks) * smooth(4., dt);
        self.tunnel += (0. - self.tunnel) * smooth(6., dt);
        self.mode = Mode::Orbit {
            azimuth,
            elevation,
            distance,
            goal_elevation,
            goal_distance,
            spin,
        };
    }

    // Salto animado por una curva hasta quedar en orbita
    fn step_warp(&mut self, world: &World, dt: f32) {
        let Mode::Warp {
            from,
            mut elapsed,
            azimuth,
        } = self.mode
        else {
            return;
        };
        elapsed += dt;
        self.near_sun = false;
        self.near_world = None;
        let progress = (elapsed / WARP_TIME).clamp(0., 1.);
        let eased = ease(progress);
        let target = center(self.selected, self.time);
        let distance = world.planet_radius(self.selected) * 2.3;
        let end = target + direction(azimuth, DEFAULT_ELEVATION) * distance;
        // Curva sobre el plano orbital para no cruzar el sol ni otros mundos
        let length = from.distance(end);
        let control = (from + end) * 0.5 + Vec3::Z * (length * 0.55 + 18.);
        let curve = from.lerp(control, eased).lerp(control.lerp(end, eased), eased);
        let (next, _) = world.safe_move(self.eye, curve, self.time, dt);
        self.eye = next;
        self.orientation = self
            .orientation
            .slerp(look(target - self.eye), smooth(7., dt))
            .normalize();
        let pulse = (PI * progress).sin();
        self.streaks = 1.25 * pulse;
        self.tunnel = pulse;
        if progress >= 1. {
            self.mode = Mode::Orbit {
                azimuth,
                elevation: DEFAULT_ELEVATION,
                distance,
                goal_elevation: DEFAULT_ELEVATION,
                goal_distance: distance,
                spin: 0.,
            };
        } else {
            self.mode = Mode::Warp {
                from,
                elapsed,
                azimuth,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{World, ORBIT_RADII};

    const DT: f32 = 1. / 60.;

    fn run(flight: &mut Flight, world: &World, input: Input, seconds: f32) {
        for _ in 0..(seconds / DT) as usize {
            flight.step(world, &input, DT);
            flight.assert_outside(world);
        }
    }

    impl Flight {
        fn distance_to_world(&self, index: usize) -> f32 {
            self.eye.distance(center(index, self.time))
        }

        fn assert_outside(&self, world: &World) {
            for i in 0..5 {
                let gap = self.eye.distance(center(i, self.time)) - world.planet_radius(i);
                assert!(gap > 0., "dentro del mundo {i}: {gap}");
            }
            assert!(self.eye.length() > world.sun_radius(), "dentro del sol");
        }
    }

    fn orbiting(world: &World, index: usize) -> Flight {
        let mut flight = Flight::new();
        let input = Input {
            select: Some(index),
            warp: true,
            ..Default::default()
        };
        flight.step(world, &input, DT);
        run(&mut flight, world, Input::default(), WARP_TIME + 0.2);
        assert!(matches!(flight.mode, Mode::Orbit { .. }));
        flight
    }

    #[test]
    fn worlds_never_touch_even_with_sun() {
        let world = World::new().unwrap();
        for step in 0..600 {
            let t = step as f32 * 0.5;
            for a in 0..5 {
                assert!(
                    center(a, t).length() - world.planet_radius(a) > world.sun_radius(),
                    "mundo {a} toca el sol"
                );
                for b in a + 1..5 {
                    let gap = center(a, t).distance(center(b, t))
                        - world.planet_radius(a)
                        - world.planet_radius(b);
                    assert!(gap > 3., "mundos {a} y {b} a {gap}");
                }
            }
        }
        assert!(ORBIT_RADII.iter().all(|&r| r < 60.));
    }

    #[test]
    fn warp_reaches_every_world_without_crossing_anything() {
        let world = World::new().unwrap();
        for index in 0..5 {
            let flight = orbiting(&world, index);
            let radius = world.planet_radius(index);
            assert!(flight.distance_to_selected() < radius * 2.6);
        }
    }

    #[test]
    fn circling_keeps_the_distance_constant() {
        let world = World::new().unwrap();
        for index in 0..5 {
            let mut flight = orbiting(&world, index);
            let before = flight.distance_to_selected();
            let input = Input {
                strafe: 1.,
                ..Default::default()
            };
            for _ in 0..(12. / DT) as usize {
                flight.step(&world, &input, DT);
                flight.assert_outside(&world);
                let drift = (flight.distance_to_selected() - before).abs();
                assert!(drift < 0.2, "espiral en {index}: {drift}");
            }
        }
    }

    #[test]
    fn top_view_goes_above_the_world_and_back() {
        let world = World::new().unwrap();
        let mut flight = orbiting(&world, 2);
        let press = Input {
            top_view: true,
            ..Default::default()
        };
        flight.step(&world, &press, DT);
        run(&mut flight, &world, Input::default(), 3.);
        let above = (flight.eye - center(2, flight.time)).normalize();
        assert!(above.z > 0.97, "no quedo arriba: {above}");
        let view = flight.orientation * Vec3::NEG_Z;
        assert!(view.z < -0.95, "no mira hacia abajo: {view}");
        flight.step(&world, &press, DT);
        run(&mut flight, &world, Input::default(), 3.);
        let back = (flight.eye - center(2, flight.time)).normalize();
        assert!(back.z < 0.6);
    }

    #[test]
    fn approach_stops_at_the_surface_and_zoom_out_works() {
        let world = World::new().unwrap();
        let mut flight = orbiting(&world, 3);
        let closer = Input {
            forward: 1.,
            ..Default::default()
        };
        run(&mut flight, &world, closer, 6.);
        let near = flight.distance_to_selected();
        assert!(near < world.collision_radius_planet(3) + 0.3);
        let farther = Input {
            forward: -1.,
            ..Default::default()
        };
        run(&mut flight, &world, farther, 3.);
        assert!(flight.distance_to_selected() > near + 15.);
    }

    #[test]
    fn free_flight_slides_past_obstacles_and_reaches_the_other_side() {
        let world = World::new().unwrap();
        // Vuelo directo contra el sol: debe rodearlo no quedar pegado
        let eye = Vec3::new(-52., 0.4, 0.3);
        let mut flight = Flight::new();
        flight.eye = eye;
        flight.orientation = look(Vec3::X);
        let input = Input {
            forward: 1.,
            boost: 1.,
            ..Default::default()
        };
        run(&mut flight, &world, input, 14.);
        assert!(flight.eye.x > -5., "se quedo pegado en {}", flight.eye);
    }

    #[test]
    fn speed_is_consistent_between_frame_rates() {
        let world = World::new().unwrap();
        let mut distances = vec![];
        for fps in [120., 60., 30.] {
            let mut flight = Flight::new();
            flight.eye = Vec3::new(240., 240., 80.);
            flight.orientation = look(Vec3::new(1., 1., 0.3));
            let start = flight.eye;
            let input = Input {
                forward: 1.,
                ..Default::default()
            };
            for _ in 0..(3. * fps) as usize {
                flight.step(&world, &input, 1. / fps as f32);
            }
            distances.push(flight.eye.distance(start));
        }
        let (min, max) = distances
            .iter()
            .fold((f32::MAX, 0f32), |(a, b), &d| (a.min(d), b.max(d)));
        assert!(max - min < 1.5, "{distances:?}");
    }

    #[test]
    fn gravity_bends_the_path_of_a_coasting_ship() {
        let world = World::new().unwrap();
        let glide = |gravity: bool| {
            let mut flight = Flight::new();
            flight.gravity = gravity;
            flight.eye = Vec3::new(-34., 0., 26.);
            flight.orientation = look(Vec3::X);
            flight.velocity = Vec3::X * 16.;
            run(&mut flight, &world, Input::default(), 3.);
            flight.eye
        };
        let (with, without) = (glide(true), glide(false));
        assert!(
            with.z < without.z - 1.,
            "la gravedad no desvio la nave: {with} contra {without}"
        );
    }

    #[test]
    fn idle_ship_falls_to_the_surface_without_crossing_it() {
        let world = World::new().unwrap();
        let mut flight = Flight::new();
        flight.eye = center(1, 0.) + Vec3::Z * 26.;
        let start = flight.distance_to_world(1);
        run(&mut flight, &world, Input::default(), 25.);
        let end = flight.distance_to_world(1);
        assert!(end < start - 3., "no cayo: {start} -> {end}");
    }

    #[test]
    fn brake_holds_the_ship_against_gravity() {
        let world = World::new().unwrap();
        let mut flight = Flight::new();
        flight.eye = center(1, 0.) + Vec3::Z * 22.;
        let brake = Input {
            brake: true,
            ..Default::default()
        };
        let start = flight.distance_to_world(1);
        run(&mut flight, &world, brake, 5.);
        assert!((flight.distance_to_world(1) - start).abs() < 0.5);
    }

    #[test]
    fn flying_through_the_portal_enters_it_without_a_button() {
        let world = World::new().unwrap();
        let out = portal_normal();
        let mut flight = Flight::new();
        flight.eye = PORTAL_CENTER + out * 40.;
        flight.orientation = look(-out);
        let forward = Input {
            forward: 1.,
            ..Default::default()
        };
        let mut entered = false;
        for _ in 0..(4. / DT) as usize {
            flight.step(&world, &forward, DT);
            entered |= flight.take_portal();
        }
        assert!(entered);
        flight.leave_portal();
        run(&mut flight, &world, Input::default(), 3.);
        assert!(!flight.take_portal(), "reentro solo al volver");
    }

    #[test]
    fn passing_beside_the_portal_does_not_enter() {
        let world = World::new().unwrap();
        let out = portal_normal();
        let side = out.cross(Vec3::Z).normalize();
        let start = PORTAL_CENTER + side * (PORTAL_RADIUS + 6.);
        assert!(world.crosses_portal(PORTAL_CENTER + out, PORTAL_CENTER - out));
        assert!(!world.crosses_portal(start + out, start - out));
    }

    #[test]
    fn approaching_a_world_names_it_and_makes_it_the_target() {
        let world = World::new().unwrap();
        for index in 0..5 {
            let mut flight = Flight::new();
            assert_eq!(flight.selected, 0);
            flight.eye = center(index, 0.) + Vec3::Z * (world.planet_radius(index) + 12.);
            flight.step(&world, &Input::default(), DT);
            assert_eq!(flight.selected, index);
            assert!(flight.target().0.contains(NAMES[index]));
        }
    }

    #[test]
    fn choosing_another_world_while_near_one_is_respected() {
        let world = World::new().unwrap();
        let mut flight = Flight::new();
        flight.eye = center(1, 0.) + Vec3::Z * (world.planet_radius(1) + 12.);
        flight.step(&world, &Input::default(), DT);
        assert_eq!(flight.selected, 1);
        let choose = Input {
            select: Some(4),
            ..Default::default()
        };
        flight.step(&world, &choose, DT);
        run(&mut flight, &world, Input::default(), 0.5);
        assert_eq!(flight.selected, 4);
    }

    #[test]
    fn near_the_sun_the_label_says_sol() {
        let world = World::new().unwrap();
        let mut flight = Flight::new();
        flight.eye = Vec3::Z * (world.sun_radius() + 12.);
        flight.step(&world, &Input::default(), DT);
        assert_eq!(flight.target().0, "SOL");
    }

    #[test]
    fn far_from_everything_the_label_keeps_the_chosen_world() {
        let world = World::new().unwrap();
        let mut flight = Flight::new();
        let choose = Input {
            select: Some(3),
            ..Default::default()
        };
        flight.step(&world, &choose, DT);
        assert!(flight.target().0.contains(NAMES[3]));
    }

    #[test]
    fn turning_builds_up_progressively() {
        let world = World::new().unwrap();
        let mut flight = Flight::new();
        flight.eye = Vec3::new(240., 240., 80.);
        let start = flight.orientation;
        let input = Input {
            look_x: 1.,
            ..Default::default()
        };
        flight.step(&world, &input, DT);
        let first = flight.orientation.angle_between(start);
        for _ in 0..30 {
            flight.step(&world, &input, DT);
        }
        let before = flight.orientation;
        flight.step(&world, &input, DT);
        let late = flight.orientation.angle_between(before);
        assert!(late > first * 3., "giro instantaneo: {first} vs {late}");
    }
}
