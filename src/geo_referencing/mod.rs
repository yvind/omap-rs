mod geo_ref;
mod transform;

pub use geo_ref::GeoRef;
pub use transform::Transform;

use quick_xml::{
    Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use crate::Result;

/// The coordinate reference system type.
#[derive(Debug, Clone, Default)]
pub enum CrsType {
    /// Local (non-georeferenced) coordinates.
    #[default]
    Local,
    /// An EPSG-registered CRS identified by code.
    Epsg(u16),
    /// A custom CRS given as a PROJ.4 string.
    Proj4(String),
    /// Gauss-Krüger zone (datum: Potsdam).
    GaussKrueger(u8),
    /// UTM zone (negative for southern hemisphere).
    Utm(i8),
}

impl CrsType {
    /// Get the EPSG code, if this CRS is defined by one (or contains one in a PROJ string).
    pub fn get_epsg_code(&self) -> Option<u16> {
        match self {
            CrsType::Epsg(c) => Some(*c),
            CrsType::Proj4(string) => {
                if let Some(index) = string.find("+init=epsg:") {
                    let mut code = 0;
                    for char in string.chars().skip(index + 11) {
                        if (48..=57_u8).contains(&(char as u8)) {
                            code = code * 10 + (char as u8 - 48) as u16;
                        } else {
                            break;
                        }
                    }
                    if (1024..=32767).contains(&code) {
                        Some(code)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Get the PROJ.4 string for this CRS, if available.
    pub fn get_proj_string(&self) -> Option<String> {
        match self {
            CrsType::Local => None,
            CrsType::Epsg(code) => Some(format!("+init=epsg:{}", code)),
            CrsType::Proj4(proj_string) => Some(proj_string.clone()),
            CrsType::GaussKrueger(code) => {
                let lon = 3 * (*code as u16);
                let x = 500_000 + (*code as u32 * 1_000_000);

                Some(format!(
                    "+proj=tmerc +lat_0=0 +lon_0={} +k=1.000000 +x_0={} +y_0=0 +ellps=bessel +datum=potsdam +units=m +no_defs",
                    lon, x
                ))
            }
            CrsType::Utm(code) => {
                if *code < 0 {
                    Some(format!(
                        "+proj=utm +datum=WGS84 +zone={} +south",
                        code.abs()
                    ))
                } else {
                    Some(format!("+proj=utm +datum=WGS84 +zone={}", code.abs()))
                }
            }
        }
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut Writer<W>) -> Result<()> {
        let (id, proj_str, parameter) = match self {
            CrsType::Local => {
                writer.write_event(Event::Start(
                    BytesStart::new("projected_crs").with_attributes([("id", "Local")]),
                ))?;
                return Ok(());
            }
            CrsType::Epsg(code) => ("EPSG", format!("+init=epsg:{code}"), format!("{code}")),
            CrsType::Proj4(proj_string) => ("PROJ.4", proj_string.clone(), proj_string),
            CrsType::GaussKrueger(code) => {
                let lon = 3 * (code as u16);
                let x = 500_000 + (code as u32 * 1_000_000);
                (
                    "Gauss-Krueger, datum: Potsdam",
                    format!(
                        "+proj=tmerc +lat_0=0 +lon_0={lon} +k=1.000000 +x_0={x} +y_0=0 +ellps=bessel +datum=potsdam +units=m +no_defs"
                    ),
                    format!("{code}"),
                )
            }
            CrsType::Utm(code) => {
                let (proj_str, param_str) = if code < 0 {
                    // south
                    (
                        format!("+proj=utm +datum=WGS84 +zone={} +south", code.abs()),
                        format!("{} S", code.abs()),
                    )
                } else {
                    // north
                    (
                        format!("+proj=utm +datum=WGS84 +zone={}", code.abs()),
                        format!("{} N", code.abs()),
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
