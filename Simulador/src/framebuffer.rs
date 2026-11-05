use crate::color::{Color, BACKGROUND};
use std::{fs::File, io::BufWriter, path::Path};

pub const SCREEN_WIDTH: usize = 1000;
pub const SCREEN_HEIGHT: usize = 750;

// Imagen RGBA y buffer de profundidad donde dibuja el rasterizador
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
    pub depth: Vec<f32>,
    current_color: Color,
}

impl Framebuffer {
    // Crea un framebuffer ya limpio
    pub fn new(width: usize, height: usize) -> Self {
        let mut buffer = Self {
            width,
            height,
            pixels: vec![0; width * height * 4],
            depth: vec![f32::INFINITY; width * height],
            current_color: Color::new(255, 255, 255),
        };
        buffer.clear();
        buffer
    }

    // Elige el color de los proximos pixeles
    pub fn set_color(&mut self, color: Color) {
        self.current_color = color;
    }

    // Pinta el fondo y reinicia la profundidad
    pub fn clear(&mut self) {
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[BACKGROUND.r, BACKGROUND.g, BACKGROUND.b, 255]);
        }
        self.depth.fill(f32::INFINITY);
    }

    // Pinta un pixel solo si pasa la prueba de profundidad
    pub fn point(&mut self, x: i32, y: i32, depth: f32) {
        if x < 0
            || y < 0
            || x as usize >= self.width
            || y as usize >= self.height
            || !(0.0..=1.0).contains(&depth)
        {
            return;
        }
        let index = y as usize * self.width + x as usize;
        // Un fragmento solo pinta si esta mas cerca que el que ya estaba
        if depth < self.depth[index] {
            self.depth[index] = depth;
            let offset = index * 4;
            let c = self.current_color;
            self.pixels[offset..offset + 4].copy_from_slice(&[c.r, c.g, c.b, 255]);
        }
    }

    // Guarda la imagen en un archivo PNG
    pub fn save_png(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let file = File::create(path)
            .map_err(|e| format!("No se pudo guardar {}: {e}", path.display()))?;
        let mut encoder =
            png::Encoder::new(BufWriter::new(file), self.width as u32, self.height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer
            .write_image_data(&self.pixels)
            .map_err(|e| e.to_string())?;
        writer.finish().map_err(|e| e.to_string())
    }
}
