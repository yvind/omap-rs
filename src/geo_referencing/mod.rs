mod geo_ref;
mod map_transform;

use std::str::FromStr;

pub use geo_ref::GeoRef;
pub use map_transform::MapTransform;

#[cfg(feature = "geo_ref")]
use proj_core::CrsDef;
use quick_xml::{
    Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use crate::Error;
use crate::Result;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UtmCode(i8);

impl UtmCode {
    pub fn get(&self) -> i8 {
        self.0
    }

    /// Create a new [`UtmCode`], positive values for N and negative for S
    ///
    /// # Errors
    ///
    /// Fails with [`Error::InvalidGeoreferencing`] if the code is not a valid UTM code (in ±[1..=60])
    pub fn new(zone: i8) -> Result<Self> {
        if (-60..=60).contains(&zone) && zone != 0 {
            Ok(Self(zone))
        } else {
            Err(Error::InvalidGeoreferencing)
        }
    }
}

impl FromStr for UtmCode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        // code N/S
        let mut parts = s.split_whitespace();

        let zone: u8 = parts.next().ok_or(Error::InvalidGeoreferencing)?.parse()?;
        let sign_part = parts.next().ok_or(Error::InvalidGeoreferencing)?;
        let sign = if sign_part == "N" {
            1
        } else if sign_part == "S" {
            -1
        } else {
            return Err(Error::InvalidGeoreferencing);
        };

        if (1..=60).contains(&zone) {
            Ok(Self(zone as i8 * sign))
        } else {
            Err(Error::InvalidGeoreferencing)
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GaussKrueger(u8);

impl GaussKrueger {
    pub fn get(&self) -> u8 {
        self.0
    }

    /// Create a new [`GaussKrueger`]
    ///
    /// # Errors
    ///
    /// Fails with [`Error::InvalidGeoreferencing`] if the code is not a valid code (in [1..=119])
    pub fn new(zone: u8) -> Result<Self> {
        if (1..=119).contains(&zone) {
            Ok(Self(zone))
        } else {
            Err(Error::InvalidGeoreferencing)
        }
    }
}

impl FromStr for GaussKrueger {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let zone: u8 = s.parse()?;

        if (1..=119).contains(&zone) {
            Ok(Self(zone))
        } else {
            Err(Error::InvalidGeoreferencing)
        }
    }
}

/// The coordinate reference system type.
#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
pub enum CrsType {
    /// Local (non-georeferenced) coordinates.
    #[default]
    Local,
    /// An EPSG-registered CRS identified by code.
    Epsg(u16),
    /// A custom CRS given as a PROJ.4 string.
    Proj4(String),
    /// Gauss-Krüger zone (datum: Potsdam).
    GaussKrueger(GaussKrueger),
    /// UTM zone (negative for southern hemisphere).
    Utm(UtmCode),
}

impl CrsType {
    /// Get the EPSG code, if this CRS is defined by one (or contains one in a PROJ string).
    pub fn epsg_code(&self) -> Option<u16> {
        match self {
            Self::Epsg(c) => Some(*c),
            Self::Proj4(string) => {
                if let Some((_, code_str)) = string.split_once("+init=epsg:") {
                    #[expect(clippy::unwrap_used)]
                    let first_part = code_str.split_whitespace().next().unwrap();
                    let code_opt = first_part.parse().ok();
                    if let Some(code) = code_opt
                        && (1024..=32767).contains(&code)
                    {
                        return Some(code);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Get the PROJ.4 string for this CRS, if available.
    pub fn proj_string(&self) -> Option<String> {
        match self {
            Self::Local => None,
            Self::Epsg(code) => Some(format!("+init=epsg:{code}")),
            Self::Proj4(proj_string) => Some(proj_string.clone()),
            Self::GaussKrueger(code) => {
                let lon = 3 * (code.get() as u16);
                let x = 500_000 + (code.get() as u32 * 1_000_000);

                Some(format!(
                    "+proj=tmerc +lat_0=0 +lon_0={lon} +k=1.000000 +x_0={x} +y_0=0 +ellps=bessel +datum=potsdam +units=m +no_defs"
                ))
            }
            Self::Utm(code) => {
                if code.get() < 0 {
                    Some(format!(
                        "+proj=utm +datum=WGS84 +zone={} +south",
                        code.get().abs()
                    ))
                } else {
                    Some(format!("+proj=utm +datum=WGS84 +zone={}", code.get().abs()))
                }
            }
        }
    }

    pub(crate) fn write<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        let (id, proj_str, parameter) = match &self {
            Self::Local => {
                writer.write_event(Event::Start(
                    BytesStart::new("projected_crs").with_attributes([("id", "Local")]),
                ))?;
                return Ok(());
            }
            Self::Epsg(code) => ("EPSG", format!("+init=epsg:{code}"), format!("{code}")),
            Self::Proj4(proj_string) => ("PROJ.4", proj_string.clone(), proj_string.clone()),
            Self::GaussKrueger(code) => {
                let lon = 3 * (code.get() as u16);
                let x = 500_000 + (code.get() as u32 * 1_000_000);
                (
                    "Gauss-Krueger, datum: Potsdam",
                    format!(
                        "+proj=tmerc +lat_0=0 +lon_0={lon} +k=1.000000 +x_0={x} +y_0=0 +ellps=bessel +datum=potsdam +units=m +no_defs"
                    ),
                    format!("{}", code.get()),
                )
            }
            Self::Utm(code) => {
                let (proj_str, param_str) = if code.get() < 0 {
                    // south
                    (
                        format!("+proj=utm +datum=WGS84 +zone={} +south", code.get().abs()),
                        format!("{} S", code.get().abs()),
                    )
                } else {
                    // north
                    (
                        format!("+proj=utm +datum=WGS84 +zone={}", code.get().abs()),
                        format!("{} N", code.get().abs()),
                    )
                };
                ("UTM", proj_str, param_str)
            }
        };
        writer.write_event(Event::Start(
            BytesStart::new("projected_crs").with_attributes([("id", id)]),
        ))?;
        writer.write_event(Event::Start(
            BytesStart::new("spec").with_attributes([("language", "PROJ.4")]),
        ))?;
        writer.write_event(Event::Text(BytesText::new(&proj_str)))?;
        writer.write_event(Event::End(BytesEnd::new("spec")))?;
        writer.write_event(Event::Start(BytesStart::new("parameter")))?;
        writer.write_event(Event::Text(BytesText::new(&parameter)))?;
        writer.write_event(Event::End(BytesEnd::new("parameter")))?;

        Ok(())
    }
}

#[cfg(feature = "geo_ref")]
impl CrsType {
    /// The CRS definition this type denotes.
    ///
    /// [`CrsType::Epsg`] resolves through the EPSG registry, every other
    /// georeferenced variant through its [PROJ.4 string][Self::proj_string]
    /// — the same precedence [`GeoRef::initialize`] applies internally, so a
    /// consumer projecting map coordinates itself resolves the CRS exactly as
    /// this crate does.
    ///
    /// Requires the `geo_ref` feature.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::LocalCrsHasNoDefinition`] for [`CrsType::Local`],
    /// which denotes the absence of a CRS, and a parse error if the definition
    /// is not one `proj-wkt` recognises.
    pub fn to_crs_def(&self) -> Result<CrsDef> {
        let definition = match self {
            Self::Local => return Err(Error::LocalCrsHasNoDefinition),
            Self::Epsg(code) => format!("EPSG:{code}"),
            other => other.proj_string().ok_or(Error::InvalidGeoreferencing)?,
        };

        Ok(proj_wkt::parse_crs(definition.as_str())?)
    }
}
