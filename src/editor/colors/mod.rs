mod color;
mod color_set;

pub use color::Color;
pub use color_set::ColorSet;

/// Cmyk values in range 0-255
#[derive(Debug, Clone, Copy)]
pub struct Cmyk {
    pub c: u8,
    pub m: u8,
    pub y: u8,
    pub k: u8,
}

impl Cmyk {
    /// Create a new Cmyk value. Range of 0-255
    pub fn new(c: u8, m: u8, y: u8, k: u8) -> Cmyk {
        Cmyk { c, m, y, k }
    }

    /// Get the CMYK values as fractions rounded to the nearest 'decimals' decimals
    pub fn as_rounded_fractions(self, decimals: u8) -> [f32; 4] {
        let factor = 10_f32.powi(decimals as i32);
        let inv_factor = 1. / factor;
        [
            ((self.c as f32 / 255.) * factor).round() * inv_factor,
            ((self.m as f32 / 255.) * factor).round() * inv_factor,
            ((self.y as f32 / 255.) * factor).round() * inv_factor,
            ((self.k as f32 / 255.) * factor).round() * inv_factor,
        ]
    }
}
