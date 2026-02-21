mod color;
mod color_set;

use std::str::FromStr;

pub use color::{Color, ColorComponent, MixedColor, SpotColor, SymbolColor, WeakColor};
pub use color_set::ColorSet;
use quick_xml::{
    Writer,
    events::{BytesStart, Event},
};

use crate::{Error, Result};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
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

    /// Get the CMYK values as fractions rounded to the nearest `decimals` decimals
    pub fn as_rounded_fractions(self, decimals: i32) -> [f64; 4] {
        let factor = 10_f64.powi(decimals);
        let inv_factor = 1. / factor;
        [
            (self.c * factor).round() * inv_factor,
            (self.m * factor).round() * inv_factor,
            (self.y * factor).round() * inv_factor,
            (self.k * factor).round() * inv_factor,
        ]
    }
}

impl From<Rgb> for Cmyk {
    fn from(value: Rgb) -> Self {
        let r = value.r;
        let g = value.g;
        let b = value.b;

        let k = 1.0 - r.max(g).max(b);
        if k >= 1.0 {
            return Cmyk::new(0.0, 0.0, 0.0, 1.0);
        }

        let c = (1.0 - r - k) / (1.0 - k);
        let m = (1.0 - g - k) / (1.0 - k);
        let y = (1.0 - b - k) / (1.0 - k);

        Cmyk::new(c, m, y, k)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CmykMode {
    #[default]
    FromSpotColors,
    FromRgb,
    Cmyk(Cmyk),
}

impl CmykMode {
    fn write<W: std::io::Write>(self, writer: &mut Writer<W>) -> Result<()> {
        let string = match self {
            CmykMode::FromSpotColors => "spotcolor",
            CmykMode::FromRgb => "rgb",
            CmykMode::Cmyk(_) => "custom",
        };
        writer.write_event(Event::Empty(
            BytesStart::new("cmyk").with_attributes([("method", string)]),
        ))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl FromStr for Rgb {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Rgb::from_hexstring(s)
    }
}

impl Rgb {
    pub fn to_hexstring(self) -> String {
        fn to_hex(value: f64) -> String {
            fn u8_to_hex(v: u8) -> char {
                assert!(v < 16);
                if v < 10 {
                    (v + 48) as char
                } else {
                    (v + 55) as char
                }
            }
            let value = (value * 255.).round() as u8;

            let first = u8_to_hex(value / 16);
            let last = u8_to_hex(value % 16);

            format!("{first}{last}")
        }
        format!("#{}{}{}", to_hex(self.r), to_hex(self.g), to_hex(self.b))
    }

    pub fn from_hexstring(s: &str) -> Result<Self> {
        fn hex_to_u8(c: char) -> Result<u8> {
            let v = c as u8;

            if (48..=57).contains(&v) {
                // '0'-'9'
                Ok(v - 48)
            } else if (65..=70).contains(&v) {
                // 'A'-'F'
                Ok(v - 55)
            } else if (97..=102).contains(&v) {
                // 'a'-'f'
                Ok(v - 87)
            } else {
                Err(Error::ColorError)
            }
        }

        let len = s.len();
        let mut chars = s.chars();
        if chars.next() != Some('#') {
            return Err(Error::ColorError);
        }
        if len == 4 {
            // a hex string of the form #b25, which means #bb2255
            let r = chars.next().unwrap();
            let g = chars.next().unwrap();
            let b = chars.next().unwrap();

            let r = hex_to_u8(r)?;
            let g = hex_to_u8(g)?;
            let b = hex_to_u8(b)?;
            Ok(Rgb {
                r: (r * 16 + r) as f64 / 255.,
                g: (g * 16 + g) as f64 / 255.,
                b: (b * 16 + b) as f64 / 255.,
            })
        } else if len == 7 {
            let r1 = chars.next().unwrap();
            let r2 = chars.next().unwrap();
            let g1 = chars.next().unwrap();
            let g2 = chars.next().unwrap();
            let b1 = chars.next().unwrap();
            let b2 = chars.next().unwrap();

            let r1 = hex_to_u8(r1)?;
            let r2 = hex_to_u8(r2)?;
            let g1 = hex_to_u8(g1)?;
            let g2 = hex_to_u8(g2)?;
            let b1 = hex_to_u8(b1)?;
            let b2 = hex_to_u8(b2)?;

            Ok(Rgb {
                r: (r1 * 16 + r2) as f64 / 255.,
                g: (g1 * 16 + g2) as f64 / 255.,
                b: (b1 * 16 + b2) as f64 / 255.,
            })
        } else {
            return Err(Error::ColorError);
        }
    }
}
impl From<Cmyk> for Rgb {
    fn from(value: Cmyk) -> Self {
        let r = (1.0 - value.c) * (1.0 - value.k);
        let g = (1.0 - value.m) * (1.0 - value.k);
        let b = (1.0 - value.y) * (1.0 - value.k);

        Rgb { r, g, b }
    }
}

impl Default for Rgb {
    fn default() -> Self {
        Self {
            r: 1.,
            g: 1.,
            b: 1.,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RgbMode {
    #[default]
    FromSpotColors,
    FromCmyk,
    Rgb(Rgb),
}

impl RgbMode {
    fn write<W: std::io::Write>(self, writer: &mut Writer<W>) -> Result<()> {
        let bs = BytesStart::new("rgb");
        let bs = match self {
            RgbMode::FromSpotColors => bs.with_attributes([("method", "spotcolor")]),
            RgbMode::FromCmyk => bs.with_attributes([("method", "cmyk")]),
            RgbMode::Rgb(rgb) => bs.with_attributes([
                ("method", "custom"),
                ("r", format!("{:.3}", rgb.r).as_str()),
                ("g", format!("{:.3}", rgb.g).as_str()),
                ("b", format!("{:.3}", rgb.b).as_str()),
            ]),
        };
        writer.write_event(Event::Empty(bs))?;
        Ok(())
    }
}
