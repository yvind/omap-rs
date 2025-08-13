use std::str::FromStr;

use geo_types::Coord;
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
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
    geographic_ref_point_deg: Coord,
}

#[derive(Debug, Clone)]
pub enum CrsType {
    Local,
    EPSG(u16),
    PROJ4(String),
    GaussKrueger(u8),
    UTM(i8),
}

impl CrsType {
    fn get_epsg_code(&self) -> Option<u16> {
        match self {
            CrsType::EPSG(c) => Some(*c),
            CrsType::PROJ4(string) => {
                if let Some(index) = string.find("+init=epsg:") {
                    let mut code = 0;

                    let mut chars = string.chars().skip(index + 11);
                    while let Some(c) = chars.next() {
                        if c.is_ascii_digit() {
                            code = code * 10 + (c as u16 - 48);
                        } else {
                            break;
                        }
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
            CrsType::EPSG(code) => Some(format!("+init=epsg:{}", code)),
            CrsType::PROJ4(proj_string) => Some(proj_string.clone()),
            CrsType::GaussKrueger(code) => {
                let lon = 3 * (*code as u16);
                let x = 500_000 + (*code as u32 * 1_000_000);

                Some(format!(
                    "+proj=tmerc +lat_0=0 +lon_0={} +k=1.000000 +x_0={} +y_0=0 +ellps=bessel +datum=potsdam +units=m +no_defs",
                    lon, x
                ))
            }
            CrsType::UTM(code) => Some(format!("+proj=utm +datum=WGS84 +zone={}", code.abs())),
        }
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        let string = match self {
            CrsType::Local => format!("<projected_crs id=\"Local\">"),
            CrsType::EPSG(code) => {
                format!(
                    "<projected_crs id=\"EPSG\"><spec language=\"PROJ.4\">+init=epsg:{}</spec><parameter>{}</parameter>",
                    code, code
                )
            }
            CrsType::PROJ4(proj_string) => {
                format!(
                    "<projected_crs id=\"PROJ.4\"><spec language=\"PROJ.4\">{}</spec><parameter>{}</parameter>",
                    proj_string, proj_string
                )
            }
            CrsType::GaussKrueger(code) => {
                let lon = 3 * (code as u16);
                let x = 500_000 + (code as u32 * 1_000_000);

                format!(
                    "<projected_crs id=\"Gauss-Krueger, datum: Potsdam\"><spec language=\"PROJ.4\">+proj=tmerc +lat_0=0 +lon_0={} +k=1.000000 +x_0={} +y_0=0 +ellps=bessel +datum=potsdam +units=m +no_defs</spec><parameter>{}</parameter>",
                    lon, x, code
                )
            }
            CrsType::UTM(code) => {
                if code < 0 {
                    // south
                    format!(
                        "<projected_crs id=\"UTM\"><spec language=\"PROJ.4\">+proj=utm +datum=WGS84 +zone={} +south</spec><parameter>{} S</parameter>",
                        code.abs(),
                        code.abs()
                    )
                } else {
                    // north
                    format!(
                        "<projected_crs id=\"UTM\"><spec language=\"PROJ.4\">+proj=utm +datum=WGS84 +zone={}</spec><parameter>{} N</parameter>",
                        code.abs(),
                        code.abs(),
                    )
                }
            }
        };

        writer.write_all(string.as_bytes())?;
        Ok(())
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

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        writer.write_all(
            format!(
                "<georeferencing scale=\"{}\" grid_scale_factor=\"{}\" auxiliary_scale_factor=\"{}\" declination=\"{}\" grivation=\"{}\">",
                self.scale, self.combined_scale_factor, self.auxiliary_scale_factor, self.declination, self.grivation
            )
            .as_bytes()
        );
        if self.map_ref_point != Coord::zero() {
            writer.write_all(
                format!(
                    "<ref_point x=\"{}\" y=\"{}\">",
                    self.map_ref_point.x, self.map_ref_point.y
                )
                .as_bytes(),
            )?;
        }

        let local_crs = matches!(self.crs_type, CrsType::Local);
        self.crs_type.write(writer)?;
        if self.projected_ref_point != Coord::zero() {
            writer.write_all(
                format!(
                    "<ref_point x=\"{}\" y=\"{}\"/>",
                    self.projected_ref_point.x, self.projected_ref_point.y
                )
                .as_bytes(),
            )?;
        }
        writer.write_all("</projected_crs>".as_bytes())?;

        if !local_crs {
            writer.write_all(
                format!(
                    "<geographic_crs id=\"Geographic coordinates\"><spec language=\"PROJ.4\">+proj=latlong +datum=WGS84</spec><ref_point_deg lat=\"{}\" lon=\"{}\"/></geographic_crs>",
                    self.geographic_ref_point_deg.x, self.geographic_ref_point_deg.y
                )
                .as_bytes()
            )?;
        }

        writer.write_all("</georeferencing>\n".as_bytes())?;
        Ok(())
    }

    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        event: &BytesStart,
    ) -> Result<Self> {
        let mut scale = None;
        let mut combined_scale_factor = 1.;
        let mut auxiliary_scale_factor = 1.;
        let mut declination = 0.;
        let mut grivation = 0.;

        for attr in event.attributes() {
            let attr = attr?;

            match attr.key.local_name().as_ref() {
                b"scale" => scale = Some(u32::from_str(&attr.unescape_value()?)?),
                b"grid_scale_factor" => {
                    combined_scale_factor = f64::from_str(&attr.unescape_value()?)?
                }
                b"auxiliary_scale_factor" => {
                    auxiliary_scale_factor = f64::from_str(&attr.unescape_value()?)?
                }
                b"declination" => declination = f64::from_str(&attr.unescape_value()?)?,
                b"grivation" => grivation = f64::from_str(&attr.unescape_value()?)?,
                _ => (),
            }
        }

        if scale.is_none() {
            // early escape if no scale is found
            return Err(Error::ParseOmapFileError("No scale found".to_string()));
        }

        let mut crs_type = CrsType::Local;
        let mut map_ref_point = Coord::zero();
        let mut projected_ref_point = Coord::zero();
        let mut geographic_ref_point_deg = Coord::zero();

        let mut buf = vec![];
        loop {
            let event = reader.read_event_into(&mut buf)?;

            match event {
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"projected_crs" => {
                        (crs_type, projected_ref_point) = parse_projected_crs(reader, &bytes_start)?
                    }
                    b"geographic_crs" => geographic_ref_point_deg = parse_geographic_crs(reader)?,
                    _ => (),
                },
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"georeferencing") {
                        break;
                    }
                }
                Event::Empty(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"ref_point" => {
                        let mut x = None;
                        let mut y = None;
                        for attr in bytes_start.attributes() {
                            let attr = attr?;

                            match attr.key.local_name().as_ref() {
                                b"x" => x = Some(f64::from_str(attr.unescape_value()?.as_ref())?),
                                b"y" => y = Some(f64::from_str(attr.unescape_value()?.as_ref())?),
                                _ => (),
                            }
                        }
                        if x.is_some() && y.is_some() {
                            map_ref_point = Coord {
                                x: x.unwrap(),
                                y: y.unwrap(),
                            };
                        }
                    }
                    b"projected_crs" => crs_type = CrsType::Local,
                    _ => (),
                },
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(String::from(
                        "Unexpected EOF in Georeferencing",
                    )));
                }
                _ => (),
            }
        }

        Ok(GeoRef {
            scale: scale.unwrap(),
            combined_scale_factor,
            auxiliary_scale_factor,
            declination,
            grivation,
            crs_type,
            map_ref_point,
            projected_ref_point,
            geographic_ref_point_deg,
        })
    }
}

fn parse_projected_crs<R: std::io::BufRead>(
    reader: &mut Reader<R>,
    bytes_start: &BytesStart,
) -> Result<(CrsType, Coord)> {
    let mut buf = Vec::new();

    let mut crs_type = CrsType::Local;
    for attr in bytes_start.attributes() {
        let attr = attr?;

        if matches!(attr.key.local_name().as_ref(), b"id") {
            let mut string = get_projected_crs_parameter_string(reader)?;
            match attr.value.as_ref() {
                b"Gauss-Krueger, datum: Potsdam" => {
                    crs_type = CrsType::GaussKrueger(u8::from_str(string.as_str())?);
                }
                b"EPSG" => {
                    crs_type = CrsType::EPSG(u16::from_str(string.as_str())?);
                }
                b"UTM" => {
                    let sign = match string.pop() {
                        Some('N') => 1_i8,
                        Some('S') => -1_i8,
                        _ => {
                            return Err(Error::ParseOmapFileError(
                                "Could not parse georeferencing".to_string(),
                            ));
                        }
                    };

                    crs_type = CrsType::UTM(sign * i8::from_str(string.trim())?);
                }
                b"PROJ.4" => crs_type = CrsType::PROJ4(string),
                _ => (),
            }
        }
    }

    let mut proj_ref_point = Coord::zero();
    loop {
        let event = reader.read_event_into(&mut buf)?;

        match event {
            Event::Empty(bytes_start) => {
                if matches!(bytes_start.name().local_name().as_ref(), b"ref_point") {
                    for attr in bytes_start.attributes() {
                        let attr = attr?;

                        match attr.key.local_name().as_ref() {
                            b"y" => {
                                proj_ref_point.y = f64::from_str(std::str::from_utf8(&attr.value)?)?
                            }
                            b"x" => {
                                proj_ref_point.x = f64::from_str(std::str::from_utf8(&attr.value)?)?
                            }
                            _ => (),
                        }
                    }
                }
            }
            Event::End(bytes_end) => {
                if matches!(bytes_end.name().local_name().as_ref(), b"projected_crs") {
                    break;
                }
            }
            Event::Eof => {
                return Err(Error::ParseOmapFileError(
                    "Unexpected EOF in georeferencing".to_string(),
                ));
            }
            _ => (),
        }
    }
    Ok((crs_type, proj_ref_point))
}

fn parse_geographic_crs<R: std::io::BufRead>(reader: &mut Reader<R>) -> Result<Coord> {
    let mut buf = Vec::new();

    let mut geo_ref_point = Coord::zero();
    loop {
        let event = reader.read_event_into(&mut buf)?;

        match event {
            Event::Empty(bytes_start) => {
                if matches!(bytes_start.name().local_name().as_ref(), b"ref_point_deg") {
                    for attr in bytes_start.attributes() {
                        let attr = attr?;

                        match attr.key.local_name().as_ref() {
                            b"lat" => {
                                geo_ref_point.y = f64::from_str(std::str::from_utf8(&attr.value)?)?
                            }
                            b"lon" => {
                                geo_ref_point.x = f64::from_str(std::str::from_utf8(&attr.value)?)?
                            }
                            _ => (),
                        }
                    }
                }
            }
            Event::End(bytes_end) => {
                if matches!(bytes_end.name().local_name().as_ref(), b"geographic_crs") {
                    break;
                }
            }
            Event::Eof => {
                return Err(Error::ParseOmapFileError(
                    "Unexpected EOF in georeferencing".to_string(),
                ));
            }
            _ => (),
        }
    }
    Ok(geo_ref_point)
}

fn get_projected_crs_parameter_string<R: std::io::BufRead>(
    reader: &mut Reader<R>,
) -> Result<String> {
    let mut buf = Vec::new();

    let mut string = String::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(bytes_start) => {
                if matches!(bytes_start.local_name().as_ref(), b"parameter") {
                    match reader.read_event_into(&mut buf)? {
                        Event::End(bytes_end) => {
                            if matches!(bytes_end.local_name().as_ref(), b"parameter") {
                                break;
                            }
                        }
                        Event::Text(bytes_text) => {
                            string = String::from_utf8(bytes_text.to_vec())?;
                        }
                        _ => (),
                    }
                }
            }
            Event::End(bytes_end) => match bytes_end.local_name().as_ref() {
                b"parameter" => break,
                b"projected_crs" => {
                    return Err(Error::ParseOmapFileError(
                        "Could not parse projected crs parameter".to_string(),
                    ));
                }
                _ => (),
            },
            Event::Eof => {
                return Err(Error::ParseOmapFileError(
                    "Unexpected EOF in georeferencing".to_string(),
                ));
            }
            _ => (),
        }
    }
    Ok(string)
}
