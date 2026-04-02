mod color;
mod color_set;

use std::str::FromStr;

pub use color::{Color, ColorComponent, MixedColor, SpotColor, SymbolColor, WeakColor};
pub use color_set::ColorSet;
use quick_xml::{
    Writer,
    events::{BytesStart, Event},
};

use crate::utils::UnitF64;
use crate::{Error, Result};

/// A CMYK color value with each component in the range `[0, 1]`.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Cmyk {
    /// Cyan component.
    pub c: UnitF64,
    /// Magenta component.
    pub m: UnitF64,
    /// Yellow component.
    pub y: UnitF64,
    /// Key (black) component.
    pub k: UnitF64,
}

impl Cmyk {
    /// Create a new Cmyk value. Range of 0..=1
    pub fn new(c: f64, m: f64, y: f64, k: f64) -> Result<Cmyk> {
        Ok(Cmyk {
            c: c.try_into()?,
            m: m.try_into()?,
            y: y.try_into()?,
            k: k.try_into()?,
        })
    }

    /// Get the CMYK values rounded to the nearest `decimals` decimals
    pub fn as_rounded_fractions(self, decimals: i32) -> [f64; 4] {
        let factor = 10_f64.powi(decimals);
        let inv_factor = 1. / factor;
        [
            (self.c.get() * factor).round() * inv_factor,
            (self.m.get() * factor).round() * inv_factor,
            (self.y.get() * factor).round() * inv_factor,
            (self.k.get() * factor).round() * inv_factor,
        ]
    }
}

impl From<Rgb> for Cmyk {
    fn from(value: Rgb) -> Self {
        let r = value.r.get();
        let g = value.g.get();
        let b = value.b.get();

        let k = 1.0 - r.max(g).max(b);
        if (1.0 - k) < 0.001 {
            return Cmyk::new(0.0, 0.0, 0.0, 1.0).unwrap();
        }

        let c = (1.0 - r - k) / (1.0 - k);
        let m = (1.0 - g - k) / (1.0 - k);
        let y = (1.0 - b - k) / (1.0 - k);

        Cmyk::new(c, m, y, k).unwrap()
    }
}

/// How the CMYK values are determined.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CmykMode {
    /// Derived from the spot-color composition.
    #[default]
    FromSpotColors,
    /// Converted from the RGB values.
    FromRgb,
    /// Explicitly specified CMYK values.
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

/// An RGB color value with each component in the range `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    /// Red component.
    pub r: UnitF64,
    /// Green component.
    pub g: UnitF64,
    /// Blue component.
    pub b: UnitF64,
}

impl FromStr for Rgb {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Rgb::from_hexstring(s)
    }
}

impl std::fmt::Display for Rgb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn to_hex(value: UnitF64, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let value = (value.get() * 255.).round() as u32;

            let first = char::from_digit(value / 16, 16).unwrap();
            let last = char::from_digit(value % 16, 16).unwrap();

            write!(f, "{first}{last}")
        }
        to_hex(self.r, f)?;
        to_hex(self.g, f)?;
        to_hex(self.b, f)
    }
}

impl From<Argb> for Rgb {
    fn from(value: Argb) -> Self {
        Rgb {
            r: value.r,
            g: value.g,
            b: value.b,
        }
    }
}

impl Rgb {
    fn from_hexstring(s: &str) -> Result<Self> {
        let (_, s) = s.split_once('#').ok_or(Error::ColorError)?;
        if s.len() < 6 {
            return Err(Error::ColorError);
        }
        let mut pieces = s.as_bytes().chunks(2).map(|b| str::from_utf8(b));
        let r = u8::from_str_radix(pieces.next().unwrap()?, 16)?;
        let g = u8::from_str_radix(pieces.next().unwrap()?, 16)?;
        let b = u8::from_str_radix(pieces.next().unwrap()?, 16)?;

        Ok(Rgb {
            r: UnitF64::clamped_from(r as f64 / 255.),
            g: UnitF64::clamped_from(g as f64 / 255.),
            b: UnitF64::clamped_from(b as f64 / 255.),
        })
    }
}

impl From<Cmyk> for Rgb {
    fn from(value: Cmyk) -> Self {
        let r = (1.0 - value.c.get()) * (1.0 - value.k.get());
        let g = (1.0 - value.m.get()) * (1.0 - value.k.get());
        let b = (1.0 - value.y.get()) * (1.0 - value.k.get());

        Rgb {
            r: UnitF64::clamped_from(r),
            g: UnitF64::clamped_from(g),
            b: UnitF64::clamped_from(b),
        }
    }
}

impl Default for Rgb {
    fn default() -> Self {
        Self {
            r: UnitF64::zero(),
            g: UnitF64::zero(),
            b: UnitF64::zero(),
        }
    }
}

/// How the RGB values are determined.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RgbMode {
    /// Derived from the spot-color composition.
    #[default]
    FromSpotColors,
    /// Converted from the CMYK values.
    FromCmyk,
    /// Explicitly specified RGB values.
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
                ("r", format!("{:.3}", rgb.r.get()).as_str()),
                ("g", format!("{:.3}", rgb.g.get()).as_str()),
                ("b", format!("{:.3}", rgb.b.get()).as_str()),
            ]),
        };
        writer.write_event(Event::Empty(bs))?;
        Ok(())
    }
}

/// An RGB color value with each component in the range `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Argb {
    /// Alpha component
    pub a: UnitF64,
    /// Red component.
    pub r: UnitF64,
    /// Green component.
    pub g: UnitF64,
    /// Blue component.
    pub b: UnitF64,
}

impl FromStr for Argb {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let (_, split) = s.split_once('#').ok_or(Error::ColorError)?;
        if split.len() < 8 {
            Ok(Rgb::from_hexstring(s)?.into())
        } else {
            Argb::from_hexstring(s)
        }
    }
}

impl std::fmt::Display for Argb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn to_hex(value: UnitF64, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let value = (value.get() * 255.).round() as u32;

            let first = char::from_digit(value / 16, 16).unwrap();
            let last = char::from_digit(value % 16, 16).unwrap();

            write!(f, "{first}{last}")
        }

        to_hex(self.a, f)?;
        to_hex(self.r, f)?;
        to_hex(self.g, f)?;
        to_hex(self.b, f)
    }
}

impl From<Rgb> for Argb {
    fn from(value: Rgb) -> Self {
        Argb {
            a: UnitF64::one(),
            r: value.r,
            g: value.g,
            b: value.b,
        }
    }
}

impl Default for Argb {
    fn default() -> Self {
        Self {
            a: UnitF64::one(),
            r: UnitF64::zero(),
            g: UnitF64::zero(),
            b: UnitF64::zero(),
        }
    }
}

impl Argb {
    fn from_hexstring(s: &str) -> Result<Self> {
        let (_, s) = s.split_once('#').ok_or(Error::ColorError)?;
        if s.len() < 8 {
            return Err(Error::ColorError);
        }
        let mut pieces = s.as_bytes().chunks(2).map(|b| str::from_utf8(b));
        let a = u8::from_str_radix(pieces.next().unwrap()?, 16)?;
        let r = u8::from_str_radix(pieces.next().unwrap()?, 16)?;
        let g = u8::from_str_radix(pieces.next().unwrap()?, 16)?;
        let b = u8::from_str_radix(pieces.next().unwrap()?, 16)?;

        Ok(Argb {
            a: UnitF64::clamped_from(a as f64 / 255.),
            r: UnitF64::clamped_from(r as f64 / 255.),
            g: UnitF64::clamped_from(g as f64 / 255.),
            b: UnitF64::clamped_from(b as f64 / 255.),
        })
    }
}
