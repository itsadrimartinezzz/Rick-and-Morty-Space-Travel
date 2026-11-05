// Color RGB de 8 bits
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    // Multiplica el color por una intensidad de luz
    pub fn shade(self, intensity: f32) -> Self {
        Self::new(
            (self.r as f32 * intensity) as u8,
            (self.g as f32 * intensity) as u8,
            (self.b as f32 * intensity) as u8,
        )
    }
}

pub const BACKGROUND: Color = Color::new(15, 23, 38);
pub const SHIP_COLOR: Color = Color::new(255, 183, 206); // Rosa pastel: #FFB7CE
