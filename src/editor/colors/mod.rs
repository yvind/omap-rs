mod color;
mod color_set;

pub use color::Color;
pub use color_set::ColorSet;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cmyk {
    pub c: f64,
    pub m: f64,
    pub y: f64,
    pub k: f64,
}

impl Cmyk {
    /// Create a new Cmyk value. Range of 0-255
    pub fn new(c: f64, m: f64, y: f64, k: f64) -> Cmyk {
        Cmyk { c, m, y, k }
    }

    /// Get the CMYK values as fractions rounded to the nearest 'decimals' decimals
    pub fn as_rounded_fractions(self, decimals: u8) -> [f64; 4] {
        let factor = 10_f64.powi(decimals as i32);
        let inv_factor = 1. / factor;
        [
            (self.c * factor).round() * inv_factor,
            (self.m * factor).round() * inv_factor,
            (self.y * factor).round() * inv_factor,
            (self.k * factor).round() * inv_factor,
        ]
    }
}
