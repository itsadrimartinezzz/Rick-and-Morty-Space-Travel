// Ventana entrada audio y presentacion con raylib; lo 3D lo hace el rasterizador propio
use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void, CStr, CString};

// Estructuras con el mismo formato que en la cabecera de raylib
#[repr(C)]
#[derive(Clone, Copy)]
struct Image {
    data: *mut c_void,
    width: c_int,
    height: c_int,
    mipmaps: c_int,
    format: c_int,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Texture {
    id: u32,
    width: c_int,
    height: c_int,
    mipmaps: c_int,
    format: c_int,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Rectangle {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Vector2 {
    x: f32,
    y: f32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct RayColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AudioStream {
    buffer: *mut c_void,
    processor: *mut c_void,
    sample_rate: u32,
    sample_size: u32,
    channels: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Music {
    stream: AudioStream,
    frame_count: u32,
    looping: bool,
    ctx_type: c_int,
    ctx_data: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Wave {
    frame_count: u32,
    sample_rate: u32,
    sample_size: u32,
    channels: u32,
    data: *mut c_void,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Sound {
    stream: AudioStream,
    frame_count: u32,
}

const LOG_WARNING: c_int = 4;
const FLAG_WINDOW_RESIZABLE: u32 = 0x0000_0004;
const PIXELFORMAT_R8G8B8A8: c_int = 7;
const TEXTURE_FILTER_BILINEAR: c_int = 1;

// Funciones de raylib: solo ventana entrada audio y dibujo 2D de la textura
extern "C" {
    fn SetTraceLogLevel(level: c_int);
    fn SetConfigFlags(flags: u32);
    fn InitWindow(width: c_int, height: c_int, title: *const c_char);
    fn IsWindowReady() -> bool;
    fn CloseWindow();
    fn WindowShouldClose() -> bool;
    fn SetExitKey(key: c_int);
    fn SetWindowTitle(title: *const c_char);
    fn GetScreenWidth() -> c_int;
    fn GetScreenHeight() -> c_int;
    fn LoadTextureFromImage(image: Image) -> Texture;
    fn UnloadTexture(texture: Texture);
    fn UpdateTexture(texture: Texture, pixels: *const c_void);
    fn SetTextureFilter(texture: Texture, filter: c_int);
    fn BeginDrawing();
    fn EndDrawing();
    fn DrawTexturePro(
        texture: Texture,
        source: Rectangle,
        dest: Rectangle,
        origin: Vector2,
        rotation: f32,
        tint: RayColor,
    );
    fn InitAudioDevice();
    fn IsAudioDeviceReady() -> bool;
    fn CloseAudioDevice();
    fn LoadMusicStream(file: *const c_char) -> Music;
    fn UnloadMusicStream(music: Music);
    fn PlayMusicStream(music: Music);
    fn UpdateMusicStream(music: Music);
    fn PauseMusicStream(music: Music);
    fn StopMusicStream(music: Music);
    fn LoadSoundFromWave(wave: Wave) -> Sound;
    fn UnloadSound(sound: Sound);
    fn PlaySound(sound: Sound);
    fn SetSoundVolume(sound: Sound, volume: f32);
    fn SetSoundPitch(sound: Sound, pitch: f32);
    fn ResumeMusicStream(music: Music);
    fn SetMusicVolume(music: Music, volume: f32);
    fn IsKeyDown(key: c_int) -> bool;
    fn IsKeyPressed(key: c_int) -> bool;
    fn IsGamepadAvailable(gamepad: c_int) -> bool;
    fn GetGamepadName(gamepad: c_int) -> *const c_char;
    fn IsGamepadButtonDown(gamepad: c_int, button: c_int) -> bool;
    fn IsGamepadButtonPressed(gamepad: c_int, button: c_int) -> bool;
    fn GetGamepadAxisMovement(gamepad: c_int, axis: c_int) -> f32;
}

// Codigos KeyboardKey de raylib
#[repr(i32)]
#[derive(Clone, Copy)]
pub enum Key {
    Space = 32,
    Num1 = 49,
    Num2 = 50,
    Num3 = 51,
    Num4 = 52,
    Num5 = 53,
    A = 65,
    D = 68,
    E = 69,
    F = 70,
    G = 71,
    H = 72,
    N = 78,
    O = 79,
    P = 80,
    Q = 81,
    R = 82,
    S = 83,
    T = 84,
    W = 87,
    X = 88,
    Z = 90,
    Escape = 256,
    Tab = 258,
    Right = 262,
    Left = 263,
    Down = 264,
    Up = 265,
    F2 = 291,
    LShift = 340,
    RShift = 344,
}

// Botones GamepadButton de raylib con su nombre en un control de PlayStation
#[repr(i32)]
#[derive(Clone, Copy)]
pub enum Button {
    DpadLeft = 4,
    DpadRight = 2,
    Triangle = 5,
    Circle = 6,
    Cross = 7,
    Square = 8,
    L1 = 9,
    R1 = 11,
    Share = 13,
    Options = 15,
    L3 = 16,
    R3 = 17,
}

// Estado analogico del control con zona muerta ya aplicada
#[derive(Clone, Copy, Default)]
pub struct Sticks {
    pub left_x: f32,
    pub left_y: f32,
    pub right_x: f32,
    pub right_y: f32,
    pub l2: f32,
    pub r2: f32,
}

// Ignora la deriva del stick y aplica una curva para mas precision
fn dead_zone(value: f32) -> f32 {
    const DEAD: f32 = 0.14;
    if value.abs() < DEAD {
        return 0.;
    }
    let scaled = (value.abs() - DEAD) / (1. - DEAD);
    // Curva cuadratica: mas precision con el stick apenas inclinado
    (scaled * scaled).min(1.) * value.signum()
}

// Convierte un gatillo a un rango de 0 a 1: en reposo vale -1 o 0 segun el driver
fn trigger(value: f32) -> f32 {
    if value == 0. {
        return 0.;
    }
    let pressed = (value + 1.) * 0.5;
    if pressed < 0.08 {
        0.
    } else {
        pressed.min(1.)
    }
}

// Ventana de raylib con la textura del framebuffer y el audio
pub struct Window {
    texture: Texture,
    source_w: f32,
    source_h: f32,
    audio_ready: bool,
    solar_music: Option<Music>,
    portal_music: Option<Music>,
    // La cancion que suena ahora; es la unica que necesita actualizarse
    active_music: Cell<Option<Music>>,
    solar_started: Cell<bool>,
    life_lost: Option<Sound>,
}

impl Window {
    // Crea la ventana y la textura donde se muestra el framebuffer
    pub fn new(w: usize, h: usize) -> Result<Self, String> {
        unsafe {
            SetTraceLogLevel(LOG_WARNING);
            SetConfigFlags(FLAG_WINDOW_RESIZABLE);
            InitWindow(1280, 720, c"Proyecto Galaxia".as_ptr());
            if !IsWindowReady() {
                return Err("raylib no pudo crear la ventana".into());
            }
            // Esc se maneja en el bucle para que tambien funcione dentro del Portal
            SetExitKey(0);
            let mut pixels = vec![0u8; w * h * 4];
            let image = Image {
                data: pixels.as_mut_ptr().cast(),
                width: w as c_int,
                height: h as c_int,
                mipmaps: 1,
                format: PIXELFORMAT_R8G8B8A8,
            };
            let texture = LoadTextureFromImage(image);
            if texture.id == 0 {
                CloseWindow();
                return Err("raylib no pudo crear la textura del framebuffer".into());
            }
            SetTextureFilter(texture, TEXTURE_FILTER_BILINEAR);
            Ok(Self {
                texture,
                source_w: w as f32,
                source_h: h as f32,
                audio_ready: false,
                solar_music: None,
                portal_music: None,
                active_music: Cell::new(None),
                solar_started: Cell::new(false),
                life_lost: None,
            })
        }
    }

    // Carga canciones y efecto; si falta algo el juego sigue sin eso
    pub fn load_audio(
        &mut self,
        solar: &str,
        portal: &str,
        volume: f32,
        life_lost: &[i16],
        sample_rate: u32,
    ) {
        unsafe {
            InitAudioDevice();
            if !IsAudioDeviceReady() {
                return;
            }
        }
        self.audio_ready = true;
        let track = |file: &str| -> Option<Music> {
            let path = CString::new(file).ok()?;
            unsafe {
                let mut music = LoadMusicStream(path.as_ptr());
                if music.frame_count == 0 {
                    return None;
                }
                music.looping = true;
                SetMusicVolume(music, volume);
                Some(music)
            }
        };
        self.solar_music = track(solar);
        self.portal_music = track(portal);
        // raylib copia las muestras al crear el sonido
        let sound = unsafe {
            LoadSoundFromWave(Wave {
                frame_count: life_lost.len() as u32,
                sample_rate,
                sample_size: 16,
                channels: 1,
                data: life_lost.as_ptr() as *mut c_void,
            })
        };
        if sound.frame_count > 0 {
            self.life_lost = Some(sound);
        }
    }

    // Pausa la cancion que este sonando
    fn pause_active(&self) {
        if let Some(music) = self.active_music.get() {
            unsafe { PauseMusicStream(music) };
        }
    }

    // La cancion del sistema solar sigue donde iba
    pub fn play_solar_music(&self) {
        self.pause_active();
        if let Some(music) = self.solar_music {
            unsafe {
                if self.solar_started.replace(true) {
                    ResumeMusicStream(music);
                } else {
                    PlayMusicStream(music);
                }
            }
        }
        self.active_music.set(self.solar_music);
    }

    // La cancion del portal empieza desde el inicio en cada entrada
    pub fn play_portal_music(&self) {
        self.pause_active();
        if let Some(music) = self.portal_music {
            unsafe {
                StopMusicStream(music);
                PlayMusicStream(music);
            }
        }
        self.active_music.set(self.portal_music);
    }

    // Efecto de perder una vida; la ultima suena mas grave
    pub fn play_life_lost(&self, last_life: bool) {
        if let Some(sound) = self.life_lost {
            unsafe {
                SetSoundVolume(sound, 0.9);
                SetSoundPitch(sound, if last_life { 0.7 } else { 1. });
                PlaySound(sound);
            }
        }
    }

    // Indica si el juego debe seguir abierto
    pub fn poll(&self) -> bool {
        unsafe { !WindowShouldClose() && !IsKeyDown(Key::Escape as c_int) }
    }

    // Indica si una tecla esta mantenida
    pub fn key(&self, key: Key) -> bool {
        unsafe { IsKeyDown(key as c_int) }
    }

    // Indica si una tecla se pulso en este cuadro
    pub fn pressed(&self, key: Key) -> bool {
        unsafe { IsKeyPressed(key as c_int) }
    }

    // Primer control conectado; admite conectarlo con el juego abierto
    pub fn gamepad(&self) -> Option<i32> {
        (0..4).find(|&id| unsafe { IsGamepadAvailable(id) })
    }

    // Nombre y ejes crudos de cada control para el diagnostico `--probar-control`
    pub fn gamepad_report(&self) -> Vec<String> {
        (0..4)
            .filter(|&id| unsafe { IsGamepadAvailable(id) })
            .map(|id| unsafe {
                let name = CStr::from_ptr(GetGamepadName(id)).to_string_lossy();
                let axes: Vec<String> = (0..6)
                    .map(|a| format!("{:+.2}", GetGamepadAxisMovement(id, a)))
                    .collect();
                let buttons: Vec<i32> = (1..18)
                    .filter(|&b| IsGamepadButtonDown(id, b))
                    .collect();
                format!("#{id} {name} | ejes {} | botones {buttons:?}", axes.join(" "))
            })
            .collect()
    }

    // Indica si un boton del control esta mantenido
    pub fn button(&self, button: Button) -> bool {
        self.gamepad()
            .is_some_and(|id| unsafe { IsGamepadButtonDown(id, button as c_int) })
    }

    // Indica si un boton del control se pulso en este cuadro
    pub fn button_pressed(&self, button: Button) -> bool {
        self.gamepad()
            .is_some_and(|id| unsafe { IsGamepadButtonPressed(id, button as c_int) })
    }

    // Lee sticks y gatillos del control
    pub fn sticks(&self) -> Sticks {
        let Some(id) = self.gamepad() else {
            return Sticks::default();
        };
        let axis = |index| unsafe { GetGamepadAxisMovement(id, index) };
        Sticks {
            left_x: dead_zone(axis(0)),
            left_y: dead_zone(axis(1)),
            right_x: dead_zone(axis(2)),
            right_y: dead_zone(axis(3)),
            l2: trigger(axis(4)),
            r2: trigger(axis(5)),
        }
    }

    // Muestra el framebuffer en la ventana y actualiza audio y entrada
    pub fn present(&self, pixels: &[u8]) {
        unsafe {
            // raylib necesita rellenar el buffer de audio en cada cuadro
            if let Some(music) = self.active_music.get() {
                UpdateMusicStream(music);
            }
            UpdateTexture(self.texture, pixels.as_ptr().cast());
            BeginDrawing();
            DrawTexturePro(
                self.texture,
                Rectangle {
                    x: 0.,
                    y: 0.,
                    width: self.source_w,
                    height: self.source_h,
                },
                Rectangle {
                    x: 0.,
                    y: 0.,
                    width: GetScreenWidth() as f32,
                    height: GetScreenHeight() as f32,
                },
                Vector2 { x: 0., y: 0. },
                0.,
                RayColor {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            );
            // EndDrawing tambien actualiza teclado y control
            EndDrawing();
        }
    }

    // Cambia el titulo de la ventana
    pub fn title(&self, title: &str) {
        if let Ok(title) = CString::new(title) {
            unsafe { SetWindowTitle(title.as_ptr()) };
        }
    }
}

impl Drop for Window {
    // Libera audio textura y ventana al cerrar
    fn drop(&mut self) {
        unsafe {
            for music in [self.solar_music, self.portal_music].into_iter().flatten() {
                UnloadMusicStream(music);
            }
            if let Some(sound) = self.life_lost {
                UnloadSound(sound);
            }
            if self.audio_ready {
                CloseAudioDevice();
            }
            UnloadTexture(self.texture);
            CloseWindow();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_zone_ignores_drift_and_keeps_full_range() {
        assert_eq!(dead_zone(0.1), 0.);
        assert_eq!(dead_zone(-0.1), 0.);
        assert!((dead_zone(1.) - 1.).abs() < 1e-5);
        assert!((dead_zone(-1.) + 1.).abs() < 1e-5);
        assert!(dead_zone(0.5) < 0.5);
    }

    #[test]
    fn trigger_at_rest_does_not_accelerate() {
        assert_eq!(trigger(-1.), 0.);
        assert_eq!(trigger(0.), 0.);
        assert!((trigger(1.) - 1.).abs() < 1e-5);
    }
}
