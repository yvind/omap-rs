use std::str::FromStr;

use geo_types::Coord;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use crate::{Error, Result, geo_referencing::Transform, notes, try_get_attr};

#[derive(Debug, Clone, Default)]
pub enum CrsType {
    #[default]
    Local,
    Epsg(u16),
    Proj4(String),
    GaussKrueger(u8),
    Utm(i8),
}

impl CrsType {
    fn get_epsg_code(&self) -> Option<u16> {
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

    fn get_proj_string(&self) -> Option<String> {
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

/// The georeferencing of the map
/// You should probably not change any of these for maps with objects
#[derive(Debug, Clone)]
pub struct GeoRef {
    /// Map scale
    pub scale_denominator: u32,
    /// Grid scale factor times auxiliary_scale_factor
    pub combined_scale_factor: f64,
    /// Scale factor due too elevation
    pub auxiliary_scale_factor: f64,
    /// Angle between grid north and magnetic north
    /// When changing this all map objects should be rotated
    pub declination_deg: f64,
    /// Angle between geographic north and projected north at the projected reference point
    pub grivation_deg: f64,
    /// The coordinate reference system definition
    pub crs_type: CrsType,
    /// in millimeters on map
    pub map_ref_point: Coord,
    /// in whatever units the projection is in, but should be meters
    pub projected_ref_point: Coord,
    /// in WGS84 degrees
    pub geographic_ref_point_deg: Coord,
}

impl GeoRef {
    /// The transform is used to go from map coordinates to projected coordinates or back
    pub fn get_transform(&self) -> Transform {
        Transform::from_geo_ref(&self)
    }

    pub fn new(scale: u32) -> Self {
        GeoRef {
            scale_denominator: scale,
            combined_scale_factor: 1.,
            auxiliary_scale_factor: 1.,
            declination_deg: 0.,
            grivation_deg: 0.,
            crs_type: CrsType::Local,
            map_ref_point: Coord::zero(),
            projected_ref_point: Coord::zero(),
            geographic_ref_point_deg: Coord::zero(),
        }
    }

    pub fn get_proj_string(&self) -> Option<String> {
        self.crs_type.get_proj_string()
    }

    // returns Some(epsg_code) if the map is georeferenced using a epsg code or by a proj string containing the code
    pub fn get_epsg_code(&self) -> Option<u16> {
        self.crs_type.get_epsg_code()
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut Writer<W>) -> Result<()> {
        writer.write_event(Event::Start(
            BytesStart::new("georeferencing").with_attributes([
                ("scale", self.scale_denominator.to_string().as_str()),
                (
                    "grid_scale_factor",
                    format!("{:.6}", self.combined_scale_factor).as_str(),
                ),
                (
                    "auxiliary_scale_factor",
                    format!("{:.6}", self.auxiliary_scale_factor).as_str(),
                ),
                (
                    "declination",
                    format!("{:.3}", self.declination_deg).as_str(),
                ),
                ("grivation", format!("{:.3}", self.grivation_deg).as_str()),
            ]),
        ))?;
        if self.map_ref_point != Coord::zero() {
            // for some reason in mm and not µm, but y is flipped
            writer.write_event(Event::Empty(BytesStart::new("ref_point").with_attributes(
                [
                    ("x", self.map_ref_point.x.to_string().as_str()),
                    ("y", (-self.map_ref_point.y).to_string().as_str()),
                ],
            )))?;
        }

        let is_local_crs = matches!(self.crs_type, CrsType::Local);
        self.crs_type.write(writer)?;
        if self.projected_ref_point != Coord::zero() {
            writer.write_event(Event::Empty(BytesStart::new("ref_point").with_attributes(
                [
                    ("x", self.projected_ref_point.x.to_string().as_str()),
                    ("y", self.projected_ref_point.y.to_string().as_str()),
                ],
            )))?;
        }
        writer.write_event(Event::End(BytesEnd::new("projected_crs")))?;

        if !is_local_crs {
            writer.write_event(Event::Start(
                BytesStart::new("geographic_crs")
                    .with_attributes([("id", "Geographic coordinates")]),
            ))?;
            writer.write_event(Event::Start(
                BytesStart::new("spec").with_attributes([("language", "PROJ.4")]),
            ))?;
            writer.write_event(Event::Text(BytesText::new("+proj=latlong +datum=WGS84")))?;
            writer.write_event(Event::Empty(
                BytesStart::new("ref_point_deg").with_attributes([
                    ("lat", self.geographic_ref_point_deg.y.to_string().as_str()),
                    ("lon", self.geographic_ref_point_deg.x.to_string().as_str()),
                ]),
            ))?;
            writer.write_event(Event::End(BytesEnd::new("geographic_crs")))?;
        }

        writer.write_event(Event::End(BytesEnd::new("georeferencing")))?;
        Ok(())
    }

    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        event: &BytesStart<'_>,
    ) -> Result<Self> {
        let scale = try_get_attr(event, "scale").ok_or(Error::ParseOmapFileError(
            "Could not find the map scale".to_string(),
        ))?;
        let auxiliary_scale_factor = try_get_attr(event, "auxiliary_scale_factor").unwrap_or(1.);
        let combined_scale_factor =
            try_get_attr(event, "grid_scale_factor").unwrap_or(auxiliary_scale_factor);
        let declination = try_get_attr(event, "declination").unwrap_or(0.);
        let grivation = try_get_attr(event, "grivation").unwrap_or(0.);

        let mut crs_type = CrsType::Local;
        let mut map_ref_point = Coord::<f64>::zero();
        let mut projected_ref_point = Coord::zero();
        let mut geographic_ref_point_deg = Coord::zero();

        let mut buf = Vec::new();
        loop {
            let event = reader.read_event_into(&mut buf)?;

            match event {
                Event::Start(bs) => match bs.local_name().as_ref() {
                    b"projected_crs" => {
                        (crs_type, projected_ref_point) = parse_projected_crs(reader, &bs)?
                    }
                    b"geographic_crs" => geographic_ref_point_deg = parse_geographic_crs(reader)?,
                    b"ref_point" => {
                        // for some reason in mm and not µm, but y is flipped
                        map_ref_point = Coord {
                            x: try_get_attr(&bs, "x").unwrap_or(map_ref_point.x),
                            y: -try_get_attr(&bs, "y").unwrap_or(-map_ref_point.y),
                        }
                    }
                    _ => (),
                },
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"georeferencing") {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(String::from(
                        "Unexpected EOF in Georeferencing",
                    )));
                }
                _ => (),
            }
        }

        Ok(GeoRef {
            scale_denominator: scale,
            combined_scale_factor,
            auxiliary_scale_factor,
            declination_deg: declination,
            grivation_deg: grivation,
            crs_type,
            map_ref_point,
            projected_ref_point,
            geographic_ref_point_deg,
        })
    }
}

fn parse_projected_crs<R: std::io::BufRead>(
    reader: &mut Reader<R>,
    bytes_start: &BytesStart<'_>,
) -> Result<(CrsType, Coord)> {
    let mut buf = Vec::new();

    let crs_type = if let Some(attr) = bytes_start.try_get_attribute(b"id")? {
        match attr.value.as_ref() {
            b"Gauss-Krueger, datum: Potsdam" => {
                // get the parameter
                let param_string = get_projected_crs_spec(reader, b"parameter")?;
                CrsType::GaussKrueger(u8::from_str(param_string.as_str())?)
            }
            b"EPSG" => {
                let param_string = get_projected_crs_spec(reader, b"parameter")?;
                CrsType::Epsg(u16::from_str(param_string.as_str())?)
            }
            b"UTM" => {
                let mut param_string = get_projected_crs_spec(reader, b"parameter")?;
                let sign = match param_string.pop() {
                    Some('N') => 1_i8,
                    Some('S') => -1_i8,
                    _ => {
                        return Err(Error::ParseOmapFileError(
                            "Could not parse georeferencing".to_string(),
                        ));
                    }
                };
                CrsType::Utm(sign * i8::from_str(param_string.trim())?)
            }
            b"Local" => CrsType::Local,
            _ => {
                let spec_string = get_projected_crs_spec(reader, b"spec")?;
                CrsType::Proj4(spec_string)
            }
        }
    } else {
        let spec_string = get_projected_crs_spec(reader, b"spec")?;
        CrsType::Proj4(spec_string)
    };

    let mut proj_ref_point = Coord::zero();
    loop {
        let event = reader.read_event_into(&mut buf)?;

        match event {
            Event::Start(bs) => {
                if matches!(bs.local_name().as_ref(), b"ref_point") {
                    proj_ref_point = Coord {
                        x: try_get_attr(&bs, "x").unwrap_or(proj_ref_point.x),
                        y: try_get_attr(&bs, "y").unwrap_or(proj_ref_point.y),
                    }
                }
            }
            Event::End(bytes_end) => {
                if matches!(bytes_end.local_name().as_ref(), b"projected_crs") {
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
            Event::Start(bs) => {
                if matches!(bs.local_name().as_ref(), b"ref_point_deg") {
                    geo_ref_point = Coord {
                        x: try_get_attr(&bs, "lon").unwrap_or(geo_ref_point.x),
                        y: try_get_attr(&bs, "lat").unwrap_or(geo_ref_point.y),
                    }
                }
            }
            Event::End(bytes_end) => {
                if matches!(bytes_end.local_name().as_ref(), b"geographic_crs") {
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

fn get_projected_crs_spec<R: std::io::BufRead>(
    reader: &mut Reader<R>,
    event_name: &[u8],
) -> Result<String> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(bytes_start) => {
                if bytes_start.local_name().as_ref() == event_name {
                    return notes::parse(reader);
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
}
