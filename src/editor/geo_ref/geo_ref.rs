use geo_types::Coord;
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::editor::{Error, Result};

#[derive(Debug, Clone)]
pub struct GeoRef {
    pub scale: u32,
    pub combined_scale_factor: f64,
    pub auxiliary_scale_factor: f64,
    pub declination: f64,
    pub grivation: f64,

    crs_type: CrsType,

    // in millimeters on map
    map_ref_point: Coord,
    // in whatever units the projection is in
    projected_ref_point: Coord,
    // in WGS84 degrees
    geographic_ref_point: Coord,
}

#[derive(Debug, Clone)]
pub enum CrsType {
    Local,
    Epsg(u16),
    Proj(String),
    GaussKrueger(u8),
    UTM(i8),
}

impl CrsType {
    fn get_epsg_code(&self) -> Option<u16> {
        match self {
            CrsType::Epsg(c) => Some(*c),
            CrsType::Proj(string) => {
                if let Some(index) = string.find("+init=epsg:") {
                    let mut code = 0;

                    let mut nums = Vec::with_capacity(5);

                    let mut chars = string.chars().skip(index + 11);
                    while let Some(c) = chars.next() {
                        if c.is_ascii_digit() {
                            nums.push(c as u8 - 48);
                        } else {
                            break;
                        }
                    }

                    for (i, num) in nums.into_iter().rev().enumerate() {
                        code += num as u16 * 10_u16.pow(i as u32)
                    }

                    if code < 1024 || code > 32767 {
                        None
                    } else {
                        Some(code)
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn get_proj_string(&self) -> Option<String> {
        match self {
            CrsType::Local => None,
            CrsType::Epsg(code) => Some(format!("+init=epsg:{}", code)),
            CrsType::Proj(proj_string) => Some(proj_string.clone()),
            CrsType::GaussKrueger(code) => {
                let lon = 3 * (*code as u16);
                let x = 500_000 + (*code as u32 * 1_000_000);

                Some(format!("+proj=tmerc +lat_0=0 +lon_0={} +k=1.000000 +x_0={} +y_0=0 +ellps=bessel +datum=potsdam +units=m +no_defs", lon, x))
            }
            CrsType::UTM(code) => Some(format!("+proj=utm +datum=WGS84 +zone={}", code.abs())),
        }
    }

    pub(crate) fn write<W: std::io::Write>(
        self,
        write: &mut W,
    ) -> std::result::Result<(), std::io::Error> {
        let string = match self {
            CrsType::Local => format!("<projected_crs id=\"Local\">"),
            CrsType::Epsg(code) => {
                format!("<projected_crs id=\"EPSG\"><spec language=\"PROJ.4\">+init=epsg:{}</spec><parameter>{}</parameter>", code, code)
            }
            CrsType::Proj(proj_string) => {
                format!("<projected_crs id=\"PROJ.4\"><spec language=\"PROJ.4\">{}</spec><parameter>{}</parameter>", proj_string, proj_string)
            }
            CrsType::GaussKrueger(code) => {
                let lon = 3 * (code as u16);
                let x = 500_000 + (code as u32 * 1_000_000);

                format!("<projected_crs id=\"Gauss-Krueger, datum: Potsdam\"><spec language=\"PROJ.4\">+proj=tmerc +lat_0=0 +lon_0={} +k=1.000000 +x_0={} +y_0=0 +ellps=bessel +datum=potsdam +units=m +no_defs</spec><parameter>{}</parameter>", lon, x, code)
            }
            CrsType::UTM(code) => {
                let code_string = if code > 0 {
                    format!("{} N", code.abs())
                } else {
                    format!("{} S", code.abs())
                };
                format!("<projected_crs id=\"UTM\"><spec language=\"PROJ.4\">+proj=utm +datum=WGS84 +zone={}</spec><parameter>{}</parameter>", code.abs(), code_string)
            }
        };

        write.write_all(string.as_bytes())
    }
}

impl GeoRef {
    pub fn get_proj_string(&self) -> Option<String> {
        self.crs_type.get_proj_string()
    }

    // returns Some(epsg_code) if the map is georeferenced using a epsg code or by a proj string containing the code
    pub fn get_epsg_code(&self) -> Option<u16> {
        self.crs_type.get_epsg_code()
    }

    pub(crate) fn write<W: std::io::Write>(
        self,
        write: &mut W,
    ) -> std::result::Result<(), std::io::Error> {
        write.write_all(
            format!(
                "<georeferencing scale=\"{}\" grid_scale_factor=\"{}\" auxiliary_scale_factor=\"{}\" declination=\"{}\" grivation=\"{}\">",
                self.scale, self.combined_scale_factor, self.auxiliary_scale_factor, self.declination, self.grivation
            )
            .as_bytes()
        );
        if self.map_ref_point != Coord::zero() {
            write.write_all(
                format!(
                    "<ref_point x=\"{}\" y=\"{}\">",
                    self.map_ref_point.x, self.map_ref_point.y
                )
                .as_bytes(),
            )?;
        }

        let local_crs = matches!(self.crs_type, CrsType::Local);
        self.crs_type.write(write)?;
        if self.projected_ref_point != Coord::zero() {
            write.write_all(
                format!(
                    "<ref_point x=\"{}\" y=\"{}\"/>",
                    self.projected_ref_point.x, self.projected_ref_point.y
                )
                .as_bytes(),
            )?;
        }
        write.write_all("</projected_crs>".as_bytes())?;

        if !local_crs {
            write.write_all(
                format!(
                    "<geographic_crs id=\"Geographic coordinates\"><spec language=\"PROJ.4\">+proj=latlong +datum=WGS84</spec><ref_point_deg lat=\"{}\" lon=\"{}\"/></geographic_crs>",
                    self.geographic_ref_point.x, self.geographic_ref_point.y
                )
                .as_bytes()
            )?;
        }

        write.write_all("</georeferencing>\n".as_bytes())
    }
}
