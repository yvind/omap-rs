use std::str::FromStr as _;

use geo_types::Coord;
#[cfg(feature = "geo_ref")]
use proj_core::{CrsDef, LinearUnit, ProjectedCrsDef, ProjectionMethod, Transform};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::CrsType;
use crate::{
    Error, OmapSection, Result, geo_referencing::MapTransform, notes, utils::try_get_attr_raw,
};

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
    pub fn get_transform(&self) -> MapTransform {
        MapTransform::from_geo_ref(self)
    }

    /// Create a new local georeferencing with the given map scale.
    pub fn new(scale: u32) -> Self {
        Self {
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

    /// Get the combined grid and auxiliary scale factor.
    pub fn combined_scale_factor(&self) -> f64 {
        self.auxiliary_scale_factor * self.grid_scale_factor
    }

    /// Get the PROJ.4 projection string for this CRS, if available.
    pub fn get_proj_string(&self) -> Option<String> {
        self.crs_type.get_proj_string()
    }

    // Returns Some(epsg_code) if the map is georeferenced using a epsg code or by a proj string containing the code
    /// Get the EPSG code for this CRS, if available.
    pub fn get_epsg_code(&self) -> Option<u16> {
        self.crs_type.get_epsg_code()
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut Writer<W>) -> Result<()> {
        let mut bytes_start = BytesStart::new("georeferencing")
            .with_attributes([("scale", self.scale_denominator.to_string().as_str())]);
        if self.combined_scale_factor() != 1. {
            bytes_start.push_attribute((
                "grid_scale_factor",
                self.combined_scale_factor().to_string().as_str(),
            ));
        }
        if self.auxiliary_scale_factor != 1. {
            bytes_start.push_attribute((
                "auxiliary_scale_factor",
                self.auxiliary_scale_factor.to_string().as_str(),
            ));
        }
        if self.declination_deg != 0. {
            bytes_start.push_attribute(("declination", self.declination_deg.to_string().as_str()));
        }
        if self.grivation_deg() != 0. {
            bytes_start.push_attribute(("grivation", self.grivation_deg().to_string().as_str()));
        }

        writer.write_event(Event::Start(bytes_start))?;
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
            writer.write_event(Event::End(BytesEnd::new("spec")))?;
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
        let scale = try_get_attr_raw(event, "scale")?.ok_or(Error::MissingMapScale)?;
        let auxiliary_scale_factor = try_get_attr_raw(event, "auxiliary_scale_factor")
            .ok()
            .flatten()
            .unwrap_or(1.);
        let grid_scale_factor = try_get_attr_raw(event, "grid_scale_factor")
            .ok()
            .flatten()
            .unwrap_or(1.)
            / auxiliary_scale_factor;
        let declination_deg = try_get_attr_raw(event, "declination")?.unwrap_or(0.);
        let convergence_deg = declination_deg - try_get_attr_raw(event, "grivation")?.unwrap_or(0.);

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
                        (crs_type, projected_ref_point) = parse_projected_crs(reader, &bs)?;
                    }
                    b"geographic_crs" => geographic_ref_point_deg = parse_geographic_crs(reader)?,
                    b"ref_point" => {
                        // for some reason in mm and not µm, but y is flipped
                        map_ref_point = Coord {
                            x: try_get_attr_raw(&bs, "x")?.unwrap_or(map_ref_point.x),
                            y: try_get_attr_raw(&bs, "y")?
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
                    return Err(Error::UnexpectedEof(OmapSection::Georeferencing));
                }
                _ => (),
            }
        }

        Ok(Self {
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
                        return Err(Error::InvalidGeoreferencing);
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
                        x: try_get_attr_raw(&bs, "x")?.unwrap_or(proj_ref_point.x),
                        y: try_get_attr_raw(&bs, "y")?.unwrap_or(proj_ref_point.y),
                    }
                }
            }
            Event::End(bytes_end) => {
                if matches!(bytes_end.local_name().as_ref(), b"projected_crs") {
                    break;
                }
            }
            Event::Eof => {
                return Err(Error::UnexpectedEof(OmapSection::Georeferencing));
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
                        x: try_get_attr_raw(&bs, "lon")?.unwrap_or(geo_ref_point.x),
                        y: try_get_attr_raw(&bs, "lat")?.unwrap_or(geo_ref_point.y),
                    }
                }
            }
            Event::End(bytes_end) => {
                if matches!(bytes_end.local_name().as_ref(), b"geographic_crs") {
                    break;
                }
            }
            Event::Eof => {
                return Err(Error::UnexpectedEof(OmapSection::Georeferencing));
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
            Event::Start(bytes_start)
                if bytes_start.local_name().as_ref() == event_name => {
                    return notes::parse(reader);
                }
            Event::Eof => {
                return Err(Error::UnexpectedEof(OmapSection::Georeferencing));
            }
            _ => (),
        }
    }
}

#[cfg(feature = "geo_ref")]
impl GeoRef {
    /// A compiled transform from this map's projected CRS to WGS84.
    ///
    /// Takes coordinates in the projected CRS's own units (normally metres)
    /// and yields degrees, `x` longitude and `y` latitude — the convention
    /// [`GeoRef::geographic_ref_point_deg`] follows.
    ///
    /// Together with [`GeoRef::get_transform`] this completes the paper ↔
    /// WGS84 round trip: [`MapTransform::to_projected`] to leave paper
    /// millimetres, then this. Requires the `geo_ref` feature.
    ///
    /// # Errors
    ///
    /// Returns an error if the map is not georeferenced ([`CrsType::Local`]),
    /// or if its CRS cannot be resolved or related to WGS84.
    pub fn to_wgs84(&self) -> Result<Transform> {
        Ok(Transform::from_crs_defs(
            &self.crs_type.to_crs_def()?,
            &wgs84_crs_def()?,
        )?)
    }

    /// A compiled transform from WGS84 to this map's projected CRS.
    ///
    /// The inverse of [`GeoRef::to_wgs84`]: takes degrees, `x` longitude and
    /// `y` latitude, and yields the projected CRS's own units. Feed the result
    /// to [`MapTransform::to_map`] to land back in paper millimetres.
    ///
    /// Requires the `geo_ref` feature.
    ///
    /// # Errors
    ///
    /// Returns an error if the map is not georeferenced ([`CrsType::Local`]),
    /// or if its CRS cannot be resolved or related to WGS84.
    pub fn from_wgs84(&self) -> Result<Transform> {
        Ok(Transform::from_crs_defs(
            &wgs84_crs_def()?,
            &self.crs_type.to_crs_def()?,
        )?)
    }

    /// Initialise full georeferencing from a projected reference point, CRS, elevation and scale.
    ///
    /// Computes declination, convergence and scale factors automatically.
    /// Requires the `geo_ref` feature.
    ///
    /// # Errors
    ///
    /// Returns an error if the CRS cannot be parsed or transformed, the
    /// reference point is invalid, or magnetic-model data cannot be computed.
    pub fn initialize(
        projected_ref_point: Coord,
        crs: CrsType,
        meters_above_sea: f64,
        scale: u32,
    ) -> Result<Self> {
        if matches!(crs, CrsType::Local) {
            let mut gr = Self::new(scale);
            gr.projected_ref_point = projected_ref_point;
            return Ok(gr);
        }

        let local_crs = crs.to_crs_def()?;
        let geographic_crs = wgs84_crs_def()?;

        let transform = Transform::from_crs_defs(&local_crs, &geographic_crs)?;

        // get geographic ref point
        let geographic_ref_point_deg = transform.convert(projected_ref_point)?;

        // get magnetic declination
        let declination_deg = Self::get_declination(geographic_ref_point_deg, meters_above_sea)?;
        let auxiliary_scale_factor =
            Self::get_elevation_scale_factor(geographic_ref_point_deg, meters_above_sea);

        let (convergence_deg, grid_scale_factor) =
            Self::get_convergence_and_grid_scale_factor(&local_crs, geographic_ref_point_deg)?;

        Ok(Self {
            scale_denominator: scale,
            grid_scale_factor,
            auxiliary_scale_factor,
            declination_deg,
            convergence_deg,
            crs_type: crs,
            map_ref_point: Coord::zero(),
            projected_ref_point,
            geographic_ref_point_deg,
        })
    }

    #[cfg(feature = "geo_ref")]
    fn get_convergence_and_grid_scale_factor(
        local_proj: &CrsDef,
        geo_ref_point: Coord,
    ) -> Result<(f64, f64)> {
        let baseline_proj = CrsDef::Projected(ProjectedCrsDef::new_with_base_geographic_crs(
            0,
            4326,
            proj_core::datum::WGS84,
            ProjectionMethod::ObliqueStereographic {
                lon0: geo_ref_point.x,
                lat0: geo_ref_point.y,
                k0: 1.0,
                false_easting: 0.0,
                false_northing: 0.0,
            },
            LinearUnit::metre(),
            "WGS 84 convergence baseline",
        ));

        // The projected CRS is anonymous, but its registered WGS84 base CRS
        // lets proj-core select an operation directly to the local CRS.
        let transform = Transform::from_crs_defs(&baseline_proj, local_proj)?;

        const D: f64 = 1000.0;
        let meridian =
            geo_types::Line::new(Coord { x: 0., y: -D / 2. }, Coord { x: 0., y: D / 2. });
        let parallel =
            geo_types::Line::new(Coord { x: -D / 2., y: 0. }, Coord { x: D / 2., y: 0. });

        // Project the stereographic baselines to the local grid
        let projected_meridian = transform.convert_geometry(meridian)?;
        let projected_parallel = transform.convert_geometry(parallel)?;

        // Points on the same meridian
        let meridian_delta = projected_meridian.delta() / D;
        let parallel_delta = projected_parallel.delta() / D;

        // Check determinant
        let determinant = parallel_delta.x * meridian_delta.y - parallel_delta.y * meridian_delta.x;
        if determinant < 0.00001 {
            Err(Error::ProjScaleToleranceError)?;
        }

        let convergence =
            (parallel_delta.y - meridian_delta.x).atan2(parallel_delta.x + meridian_delta.y);

        let grid_scale_factor = determinant.sqrt();

        Ok((convergence.to_degrees(), grid_scale_factor))
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
        use chrono::Datelike as _;
        use world_magnetic_model::{
            GeomagneticField,
            time::Date,
            uom::si::{
                angle::{Angle, degree},
                length::{Length, meter},
            },
        };

        let date = chrono::Local::now();
        let year = date.year();
        let day = date.ordinal() as u16;

        let model_date = Date::from_ordinal_date(year, day)
            .or_else(|_| Date::from_ordinal_date(2026, 180))
            .map_err(|_invalid_date| Error::InvalidGeoreferencing)?;

        let field = GeomagneticField::new(
            Length::new::<meter>(meters_above_sea_level as f32),
            Angle::new::<degree>(geo_ref_point.y as f32),
            Angle::new::<degree>(geo_ref_point.x as f32),
            model_date,
        )?;
        let dec = field.declination().get::<degree>();

        Ok(dec as f64)
    }
}

/// The geographic CRS every georeferenced omap file relates its projected CRS to.
#[cfg(feature = "geo_ref")]
fn wgs84_crs_def() -> Result<CrsDef> {
    Ok(proj_wkt::parse_crs("EPSG:4326")?)
}

#[cfg(all(test, feature = "geo_ref"))]
mod tests {
    use geo_types::Coord;

    use super::{CrsType, GeoRef};
    use crate::{Error, Result};

    /// On the UTM zone 32 central meridian, so the expected longitude is exact.
    const UTM_32N_POINT: Coord = Coord {
        x: 500_000.,
        y: 6_650_000.,
    };

    fn georeferenced(crs_type: CrsType) -> GeoRef {
        GeoRef {
            crs_type,
            ..GeoRef::new(10_000)
        }
    }

    #[test]
    fn local_crs_has_no_definition() {
        assert!(matches!(
            CrsType::Local.to_crs_def(),
            Err(Error::LocalCrsHasNoDefinition)
        ));
        assert!(georeferenced(CrsType::Local).to_wgs84().is_err());
        assert!(georeferenced(CrsType::Local).from_wgs84().is_err());
    }

    #[test]
    fn to_wgs84_reproduces_the_geographic_ref_point() -> Result<()> {
        // initialize resolves the CRS internally; to_wgs84 must resolve it the
        // same way, or a consumer projecting its own coordinates would drift
        // from the reference point stored in the file.
        let geo_ref = GeoRef::initialize(UTM_32N_POINT, CrsType::Utm(32), 100., 10_000)?;

        let converted = geo_ref.to_wgs84()?.convert(UTM_32N_POINT)?;

        assert!(
            (converted.x - geo_ref.geographic_ref_point_deg.x).abs() < 1e-9,
            "longitude {} != {}",
            converted.x,
            geo_ref.geographic_ref_point_deg.x
        );
        assert!(
            (converted.y - geo_ref.geographic_ref_point_deg.y).abs() < 1e-9,
            "latitude {} != {}",
            converted.y,
            geo_ref.geographic_ref_point_deg.y
        );
        Ok(())
    }

    #[test]
    fn wgs84_transforms_are_degrees_lon_lat_and_round_trip() -> Result<()> {
        let geo_ref = georeferenced(CrsType::Utm(32));

        let degrees = geo_ref.to_wgs84()?.convert(UTM_32N_POINT)?;
        assert!(
            (degrees.x - 9.).abs() < 1e-9,
            "x must be the longitude in degrees, got {}",
            degrees.x
        );
        assert!(
            (degrees.y - 59.9).abs() < 0.1,
            "y must be the latitude in degrees, got {}",
            degrees.y
        );

        let projected = geo_ref.from_wgs84()?.convert(degrees)?;
        assert!(
            (projected.x - UTM_32N_POINT.x).abs() < 1e-3,
            "easting drift"
        );
        assert!(
            (projected.y - UTM_32N_POINT.y).abs() < 1e-3,
            "northing drift"
        );
        Ok(())
    }

    #[test]
    fn epsg_code_and_proj_string_resolution_agree() -> Result<()> {
        // EPSG:32632 is WGS84 / UTM 32N: the registry path, the PROJ.4 path
        // and an EPSG code embedded in a PROJ.4 string must all land together.
        let reference = georeferenced(CrsType::Utm(32))
            .to_wgs84()?
            .convert(UTM_32N_POINT)?;

        for crs_type in [
            CrsType::Epsg(32632),
            CrsType::Proj4("+init=epsg:32632".to_owned()),
        ] {
            let converted = georeferenced(crs_type.clone())
                .to_wgs84()?
                .convert(UTM_32N_POINT)?;

            assert!(
                (converted.x - reference.x).abs() < 1e-9
                    && (converted.y - reference.y).abs() < 1e-9,
                "{crs_type:?} resolved to {converted:?}, expected {reference:?}"
            );
        }
        Ok(())
    }
}
