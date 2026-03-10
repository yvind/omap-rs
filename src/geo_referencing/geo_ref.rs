use std::str::FromStr;

use geo_types::Coord;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::CrsType;
use crate::{Error, Result, geo_referencing::Transform, notes, utils::try_get_attr};

/// The georeferencing information of the map
#[derive(Debug, Clone)]
pub struct GeoRef {
    /// Map scale
    /// Remember to scale all map coordinates after changing this
    pub scale_denominator: u32,
    /// Grid scale factor
    /// Remember to scale all map coordinates after changing this
    pub grid_scale_factor: f64,
    /// Scale factor due too elevation
    /// Remember to scale all map coordinates after changing this
    pub auxiliary_scale_factor: f64,
    /// Angle between geographic north and magnetic north at the projected reference point
    /// Remember to rotate all map coordinates around the map center after changing this
    pub declination_deg: f64,
    /// Angle between projected north and geographic north at the projected reference point
    /// Remember to rotate all map coordinates around the map center after changing this
    pub convergence_deg: f64,
    /// The coordinate reference system definition
    /// Changing this might invalidate the ref points, scale factors and declination/convergence
    pub crs_type: CrsType,
    /// in millimeters on map
    /// Remember to translate all map coordinates after changing this
    pub map_ref_point: Coord,
    /// in whatever units the projection is in (should be meters)
    /// Changing this might invalidate the scale factors, declination/grivation and geographic reference point
    pub projected_ref_point: Coord,
    /// in WGS84 degrees
    /// Should be the inverse projection of the projected ref point into lat lon (ignored for local crs type)
    pub geographic_ref_point_deg: Coord,
}

impl GeoRef {
    /// The transform is used to go from map coordinates to projected coordinates or back
    pub fn get_transform(&self) -> Transform {
        Transform::from_geo_ref(self)
    }

    pub fn new(scale: u32) -> Self {
        GeoRef {
            scale_denominator: scale,
            grid_scale_factor: 1.,
            auxiliary_scale_factor: 1.,
            declination_deg: 0.,
            convergence_deg: 0.,
            crs_type: CrsType::Local,
            map_ref_point: Coord::zero(),
            projected_ref_point: Coord::zero(),
            geographic_ref_point_deg: Coord::zero(),
        }
    }

    /// Get the angle between projected north and magnetic north (map north)
    /// grivation = declination - convergence
    pub fn grivation_deg(&self) -> f64 {
        self.declination_deg - self.convergence_deg
    }

    pub fn combined_scale_factor(&self) -> f64 {
        self.auxiliary_scale_factor * self.grid_scale_factor
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
                    format!("{:.6}", self.combined_scale_factor()).as_str(),
                ),
                (
                    "auxiliary_scale_factor",
                    format!("{:.6}", self.auxiliary_scale_factor).as_str(),
                ),
                (
                    "declination",
                    format!("{:.3}", self.declination_deg).as_str(),
                ),
                ("grivation", format!("{:.3}", self.grivation_deg()).as_str()),
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
        let grid_scale_factor =
            try_get_attr(event, "grid_scale_factor").unwrap_or(1.) / auxiliary_scale_factor;
        let declination_deg = try_get_attr(event, "declination").unwrap_or(0.);
        let convergence_deg = try_get_attr(event, "grivation").unwrap_or(0.) + declination_deg;

        let mut crs_type = CrsType::Local;
        let mut map_ref_point = Coord::zero();
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
                            y: try_get_attr(&bs, "y")
                                .map(|y: f64| -y)
                                .unwrap_or(map_ref_point.y),
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
            grid_scale_factor,
            auxiliary_scale_factor,
            declination_deg,
            convergence_deg,
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

#[cfg(feature = "geo_ref")]
use proj4rs::{Proj, transform::transform};
#[cfg(feature = "geo_ref")]
impl GeoRef {
    pub fn initialize(
        projected_ref_point: Coord,
        crs: CrsType,
        meters_above_sea: f64,
        scale: u32,
    ) -> Result<Self> {
        let local_proj = match &crs {
            CrsType::Local => {
                let mut gr = GeoRef::new(scale);
                gr.projected_ref_point = projected_ref_point;
                return Ok(gr);
            }
            CrsType::Epsg(e) => Proj::from_epsg_code(*e),
            c => Proj::from_proj_string(c.get_proj_string().unwrap().as_str()),
        }?;

        // get geographic ref point
        let mut geo_ref_point = projected_ref_point;
        let geo_proj = Proj::from_user_string("WGS84")?;
        transform(&local_proj, &geo_proj, &mut geo_ref_point)?;

        // get magnetic declination
        let declination = Self::get_declination(geo_ref_point, meters_above_sea)?;
        let auxiliary_scale_factor =
            Self::get_elevation_scale_factor(geo_ref_point, meters_above_sea);

        let (convergence, grid_scale_factor) =
            Self::get_convergence_and_grid_scale_factor(&local_proj, geo_ref_point)?;

        let geographic_ref_point_deg = Coord {
            x: geo_ref_point.x.to_degrees(),
            y: geo_ref_point.y.to_degrees(),
        };

        Ok(GeoRef {
            scale_denominator: scale,
            grid_scale_factor,
            auxiliary_scale_factor,
            declination_deg: declination.to_degrees(),
            convergence_deg: convergence.to_degrees(),
            crs_type: crs,
            map_ref_point: Coord::zero(),
            projected_ref_point,
            geographic_ref_point_deg,
        })
    }

    #[cfg(feature = "geo_ref")]
    fn get_convergence_and_grid_scale_factor(
        local_proj: &Proj,
        geo_ref_point: Coord,
    ) -> Result<(f64, f64)> {
        let baseline_proj = Proj::from_proj_string(
            format!(
                "+proj=sterea +lat_0={} +lon_0={} +ellps=WGS84 +units=m",
                geo_ref_point.y.to_degrees(),
                geo_ref_point.x.to_degrees()
            )
            .as_str(),
        )?;

        const D: f64 = 1000.0;
        let mut meridian =
            geo_types::Line::new(Coord { x: 0., y: -D / 2. }, Coord { x: 0., y: D / 2. });
        let mut parallel =
            geo_types::Line::new(Coord { x: -D / 2., y: 0. }, Coord { x: D / 2., y: 0. });

        // Project the stereographic baselines to the local grid
        transform(&baseline_proj, local_proj, &mut meridian)?;
        transform(&baseline_proj, local_proj, &mut parallel)?;

        // Points on the same meridian
        let meridian_delta = meridian.delta() / D;
        let parallel_delta = parallel.delta() / D;

        // Check determinant
        let determinant = parallel_delta.x * meridian_delta.y - parallel_delta.y * meridian_delta.x;
        if determinant < 0.00001 {
            Err(proj4rs::errors::Error::ToleranceConditionError)?;
        }

        let convergence =
            (parallel_delta.y - meridian_delta.x).atan2(parallel_delta.x + meridian_delta.y);

        let grid_scale_factor = determinant.sqrt();

        Ok((convergence, grid_scale_factor))
    }

    #[cfg(feature = "geo_ref")]
    fn get_elevation_scale_factor(geo_ref_point: Coord, meters_above_sea_level: f64) -> f64 {
        // this is (ellipsoid_radius / (ellipsoid_radius + m_above_ellipsoid))
        //
        // ellipsoid_radius = R_equator * (1 - f * sin^2(lat))
        // f = 1 / 298.257223563
        // R_equator = 6378137.0m
        const F: f64 = 1. / 298.257223563;
        const R_EQUATOR: f64 = 6378137.;

        let ellipsoid_radius = R_EQUATOR * (1. - F * geo_ref_point.y.sin().powi(2));

        ellipsoid_radius / (ellipsoid_radius + meters_above_sea_level)
    }

    #[cfg(feature = "geo_ref")]
    fn get_declination(geo_ref_point: Coord, meters_above_sea_level: f64) -> Result<f64> {
        use chrono::Datelike;
        use world_magnetic_model::{
            GeomagneticField,
            time::Date,
            uom::si::{
                angle::{Angle, radian},
                length::{Length, meter},
            },
        };

        let date = chrono::Local::now();
        let year = date.year();
        let day = date.ordinal() as u16;

        let field = GeomagneticField::new(
            Length::new::<meter>(meters_above_sea_level as f32),
            Angle::new::<radian>(geo_ref_point.y as f32),
            Angle::new::<radian>(geo_ref_point.x as f32),
            Date::from_ordinal_date(year, day)
                .unwrap_or(Date::from_ordinal_date(2026, 180).unwrap()),
        )?;
        let dec = field.declination().get::<radian>();

        Ok(dec as f64)
    }
}
