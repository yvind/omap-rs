use crate::{MapObject, OmapResult, Scale};
use chrono::Datelike;
use geo_types::Coord;
use log::{log, Level};

use proj4rs::{transform::transform, Proj};
use world_magnetic_model::{
    time::Date,
    uom::si::f32::{Angle, Length},
    uom::si::{angle::radian, length::meter},
    GeomagneticField,
};

use std::io::{BufWriter, Write};
use std::{
    ffi::OsStr,
    fs::File,
    path::{Path, PathBuf},
};

/// Struct representing an Orienteering map
/// ALL COORDINATES ARE RELATIVE THE ref_point
/// If epsg.is_some() the map is written georefrenced
/// else it is written in Local space
pub struct Omap {
    elevation_scale_factor: f64,
    combined_scale_factor: f64,
    declination: f64,
    grivation: f64,
    scale: Scale,
    epsg: Option<u16>,
    ref_point: Coord,

    objects: Vec<MapObject>,
}

impl Omap {
    pub fn new(georef_point: Coord, epsg_crs: Option<u16>, scale: Scale) -> Self {
        // should use a magnetic model to figure out the declination (angle between true north and magnetic north) at the ref_point
        // and proj4rs for the convergence (angle between true north and grid north)
        //
        // the grivation (angle between magnetic north and grid north) must be used when calulating map coords as the axes are magnetic
        // grivation = declination - convergence
        //
        // proj4rs should be able to give the grid_scale_factor which relates ellipsoid distances to grid distances
        // in proj4rs this is Proj::projdata.k0 (crate public only) but should be a function of lat/lon??
        //
        // further the elevation factor (called auxiliary scale factor in mapper) relates real distances to ellipsoid distances
        // this is (ellipsoid_radius / (ellipsoid_radius + m_above_ellipsoid)), which also is a function of lat/lon and height
        //
        // to calculate map units the combined scale factor and scale of map is needed to go from grid coordinates to real coordinates to map coordinates
        //
        // in summary to calculate map coordinates we need:
        // - a crs
        // - grivation (declination - convergence)
        // - the combined scale factor

        let declination = if let Some(epsg) = epsg_crs {
            Self::declination(epsg, georef_point).unwrap_or(0.)
        } else {
            0.
        };

        let (grid_scale_factor, elevation_scale_factor, convergence) = if let Some(epsg) = epsg_crs
        {
            Self::scale_factors_and_convergence(epsg, georef_point).unwrap_or((1., 1., 0.))
        } else {
            (1., 1., 0.)
        };

        Omap {
            elevation_scale_factor,
            combined_scale_factor: grid_scale_factor * elevation_scale_factor,
            declination,
            grivation: declination - convergence,
            scale,
            epsg: epsg_crs,
            ref_point: georef_point,
            objects: vec![],
        }
    }

    fn scale_factors_and_convergence(epsg: u16, ref_point: Coord) -> OmapResult<(f64, f64, f64)> {
        let geographic_proj = Proj::from_epsg_code(4326)?;
        let local_proj = Proj::from_epsg_code(epsg)?;

        // transform ref_point to lat/lon
        let mut geo_ref_point = (ref_point.x, ref_point.y);
        transform(&local_proj, &geographic_proj, &mut geo_ref_point)?;

        let baseline_proj = Proj::from_proj_string(
            format!(
                "+proj=sterea +lat_0={} +lon_0={} +ellps=WGS84 +units=m",
                geo_ref_point.1, geo_ref_point.0
            )
            .as_str(),
        )?;

        const DELTA: f64 = 1000.0;
        let mut base_line_points = [
            (DELTA / 2., 0.),  // EAST
            (0., DELTA / 2.),  // NORTH
            (-DELTA / 2., 0.), // WEST
            (0., -DELTA / 2.), // SOUTH
        ];

        // Determine 1 km baselines west-east and south-north on the ellipsoid
        transform(
            &baseline_proj,
            &geographic_proj,
            base_line_points.as_mut_slice(),
        )?;

        //reproject the points down to the grid
        transform(
            &geographic_proj,
            &local_proj,
            base_line_points.as_mut_slice(),
        )?;

        // Points on the same meridian
        let d_northing_dy = (base_line_points[1].1 - base_line_points[3].1) / DELTA;
        let d_easting_dy = (base_line_points[1].0 - base_line_points[3].0) / DELTA;

        // Points on the same parallel
        let d_northing_dx = (base_line_points[0].1 - base_line_points[2].1) / DELTA;
        let d_easting_dx = (base_line_points[0].0 - base_line_points[2].0) / DELTA;

        // Check determinant
        let determinant = d_easting_dx * d_northing_dy - d_northing_dx * d_easting_dy;
        if determinant < 0.01 {
            Err(proj4rs::errors::Error::ToleranceConditionError)?;
        }

        let convergence = (d_northing_dx - d_easting_dy).atan2(d_easting_dx + d_northing_dy);
        let grid_scale_factor = determinant.sqrt();

        Ok((grid_scale_factor, 1., convergence))
    }

    fn declination(epsg: u16, ref_point: Coord) -> OmapResult<f64> {
        let geographic_proj = Proj::from_epsg_code(4326)?;
        let local_proj = Proj::from_epsg_code(epsg)?;

        // transform ref_point to lat/lon
        let mut geo_ref_point = (ref_point.x, ref_point.y);
        transform(&local_proj, &geographic_proj, &mut geo_ref_point)?;

        let date = chrono::Local::now();
        let year = date.year();
        let day = date.ordinal() as u16;

        let field = GeomagneticField::new(
            Length::new::<meter>(0.),
            Angle::new::<radian>(geo_ref_point.1 as f32),
            Angle::new::<radian>(geo_ref_point.0 as f32),
            Date::from_ordinal_date(year, day)
                .unwrap_or(Date::from_ordinal_date(2025, 180).unwrap()),
        )?;
        let dec = field.declination().get::<radian>();

        Ok(dec as f64)
    }

    pub fn add_object(&mut self, obj: MapObject) {
        self.objects.push(obj);
    }

    pub fn write_to_file(
        self,
        filename: &OsStr,
        dir: &Path,
        bezier_error: Option<f64>,
    ) -> OmapResult<()> {
        let mut filepath = PathBuf::from(dir);
        filepath.push(filename);
        filepath.set_extension("omap");

        let f = File::create(&filepath)?;
        let mut f = BufWriter::new(f);

        self.write_header(&mut f)?;
        self.write_colors_symbols(&mut f)?;
        self.write_objects(&mut f, bezier_error)?;
        Self::write_end_of_file(&mut f)?;
        Ok(())
    }

    fn write_header(&self, f: &mut BufWriter<File>) -> OmapResult<()> {
        match self.scale {
            Scale::S15_000 => (),
            _ => log!(Level::Warn, "Only 1:15_000 supported yet"),
        }

        f.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<map xmlns=\"http://openorienteering.org/apps/mapper/xml/v2\" version=\"9\">\n<notes></notes>\n")?;

        if let Some(epsg) = self.epsg {
            let geographic_proj = Proj::from_epsg_code(4326)?;
            let local_proj = Proj::from_epsg_code(epsg)?;

            // transform ref_point to lat/lon
            let mut geo_ref_point = (self.ref_point.x, self.ref_point.y);
            transform(&local_proj, &geographic_proj, &mut geo_ref_point)?;

            f.write_all(format!("<georeferencing scale=\"{}\" auxiliary_scale_factor=\"{}\" declination=\"{}\">\
            <projected_crs id=\"EPSG\"><spec language=\"PROJ.4\">+init=epsg:{}</spec><parameter>{}</parameter>\
            <ref_point x=\"{}\" y=\"{}\"/></projected_crs><geographic_crs id=\"Geographic coordinates\">\
            <spec language=\"PROJ.4\">+proj=latlong +datum=WGS84</spec>\
            <ref_point_deg lat=\"{}\" lon=\"{}\"/></geographic_crs></georeferencing>",
            self.scale, self.elevation_scale_factor, self.declination, epsg, epsg, self.ref_point.x, self.ref_point.y, geo_ref_point.1, geo_ref_point.0).as_bytes())?;
        } else {
            f.write_all(format!("<georeferencing scale=\"{}\"><projected_crs id=\"Local\"><ref_point x=\"{}\" y=\"{}\"/></projected_crs></georeferencing>\n", self.scale, self.ref_point.x, self.ref_point.y).as_bytes())?;
        }

        Ok(())
    }

    fn write_colors_symbols(&self, f: &mut BufWriter<File>) -> OmapResult<()> {
        match self.scale {
            Scale::S10_000 => {
                log!(Level::Warn, "Only 1:15_000 symbols implemented yet");
                f.write_all(include_str!("colors_and_symbols_omap_15.txt").as_bytes())?;
            }
            Scale::S15_000 => {
                f.write_all(include_str!("colors_and_symbols_omap_15.txt").as_bytes())?;
            }
        }
        Ok(())
    }

    fn write_objects(self, f: &mut BufWriter<File>, bezier_error: Option<f64>) -> OmapResult<()> {
        f.write_all(
            format!(
                "<parts count=\"1\" current=\"0\">\n<part name=\"map\"><objects count=\"{}\">\n",
                self.objects.len()
            )
            .as_bytes(),
        )?;

        for object in self.objects.into_iter() {
            object.write_to_map(
                f,
                bezier_error,
                self.scale,
                self.grivation,
                self.combined_scale_factor,
            )?;
        }

        f.write_all(b"</objects></part>\n</parts>\n")?;
        Ok(())
    }

    fn write_end_of_file(f: &mut BufWriter<File>) -> OmapResult<()> {
        f.write_all(b"<templates count=\"0\" first_front_template=\"0\">\n<defaults use_meters_per_pixel=\"true\" meters_per_pixel=\"0\" dpi=\"0\" scale=\"0\"/></templates>\n<view>\n")?;
        f.write_all(b"<grid color=\"#646464\" display=\"0\" alignment=\"0\" additional_rotation=\"0\" unit=\"1\" h_spacing=\"500\" v_spacing=\"500\" h_offset=\"0\" v_offset=\"0\" snapping_enabled=\"true\"/>\n")?;
        f.write_all(b"<map_view zoom=\"1\" position_x=\"0\" position_y=\"0\"><map opacity=\"1\" visible=\"true\"/><templates count=\"0\"/></map_view>\n</view>\n</barrier>\n</map>")?;
        Ok(())
    }
}
