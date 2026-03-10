use std::str::FromStr;

use geo_types::Coord;
use quick_xml::events::BytesStart;

use crate::{Error, Result};
const FILE_COORD_MAX: f64 = ((i32::MAX / 1000) - 1) as f64;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Code {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl FromStr for Code {
    type Err = Error;
    fn from_str(value: &str) -> Result<Self> {
        let mut parts = value.split('.').take(3);
        Ok(Code {
            major: parts.next().ok_or(Error::SymbolError)?.parse()?,
            minor: parts.next().and_then(|i| i.parse().ok()).unwrap_or(0),
            patch: parts.next().and_then(|i| i.parse().ok()).unwrap_or(0),
        })
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// parse helpers
pub(crate) fn parse_attr<T: FromStr>(value: std::borrow::Cow<'_, [u8]>) -> Option<T> {
    std::str::from_utf8(value.as_ref())
        .ok()
        .and_then(|s| T::from_str(s).ok())
}

pub(crate) fn try_get_attr<T: FromStr>(bytes: &BytesStart<'_>, attr: &str) -> Option<T> {
    bytes
        .try_get_attribute(attr)
        .ok()
        .flatten()
        .and_then(|a| parse_attr(a.value))
}

/// A f64, but only allowed to be in the unit interval 0.0..=1.0
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct UnitF64(f64);

impl UnitF64 {
    pub fn get(self) -> f64 {
        self.0
    }

    /// Get UnitF64 from a f64, clamp values outside of the unit interval and map NaN to 0.
    pub fn clamped_from(value: f64) -> Self {
        if value.is_nan() {
            Self(0.)
        } else {
            Self(value.clamp(0., 1.))
        }
    }
}

/// Tries to create a UnitF64 from a f64, but succeeds only for values in the unit interval
impl TryFrom<f64> for UnitF64 {
    type Error = Error;

    fn try_from(v: f64) -> std::result::Result<Self, Self::Error> {
        if v.is_finite() && (0.0..=1.0).contains(&v) {
            Ok(Self(v))
        } else {
            Err(Error::NotInUnitInterval)
        }
    }
}

/// A f64, but not allowed to be negative
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct NonNegativeF64(f64);

impl NonNegativeF64 {
    /// Get the inner f64
    pub fn get(self) -> f64 {
        self.0
    }

    /// Get NonNegativeF64 from a f64, clamp negative values to 0. and map NaN to 0.
    pub fn clamped_from(value: f64) -> Self {
        if value.is_nan() {
            Self(0.)
        } else {
            Self(value.max(0.))
        }
    }

    /// The files uses 1/1000 mm as the unit
    pub(crate) fn to_file_value(self) -> Result<u32> {
        Ok(to_file_value(self.0)? as u32)
    }

    /// Create from file value (1/1000 mm integer) to mm
    pub(crate) fn from_file_value(value: u32) -> Self {
        NonNegativeF64(from_file_value(value as i32))
    }
}

/// Tries to create a NonNegativeF64 from a f64, but succeeds only for non-negative values
impl TryFrom<f64> for NonNegativeF64 {
    type Error = Error;

    fn try_from(v: f64) -> std::result::Result<Self, Self::Error> {
        if v >= 0. {
            Ok(Self(v))
        } else {
            Err(Error::NotNonNegativeF64)
        }
    }
}

pub(crate) fn to_file_coords(map_coord: Coord) -> Result<Coord<i32>> {
    Ok(Coord {
        x: to_file_value(map_coord.x)?,
        y: -to_file_value(map_coord.y)?,
    })
}

pub(crate) fn to_file_value(value: f64) -> Result<i32> {
    if value.abs() > FILE_COORD_MAX {
        return Err(Error::MapCoordOutOfBounds);
    }
    Ok((value * 1000.).round() as i32)
}

pub(crate) fn from_file_value(value: i32) -> f64 {
    value as f64 / 1000.
}

pub(crate) fn from_file_coords(file_coord: Coord<i32>) -> Coord {
    Coord {
        x: from_file_value(file_coord.x),
        y: -from_file_value(file_coord.y),
    }
}
