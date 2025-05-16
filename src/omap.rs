use crate::{LineSymbol, MapObject, OmapResult, PointObject, PointSymbol, Scale, Symbol};
use chrono::Datelike;
use geo_types::{Coord, LineString, Point};
use kiddo::SquaredEuclidean;
use proj4rs::{transform::transform, Proj};
use std::{
    collections::HashMap,
    io::{BufWriter, Write},
};
use std::{ffi::OsStr, fs::File, path::PathBuf};
use world_magnetic_model::{
    time::Date,
    uom::si::f32::{Angle, Length},
    uom::si::{angle::radian, length::meter},
    GeomagneticField,
};

/// Struct representing an Orienteering map
/// ALL OBJECT COORDINATES ARE RELATIVE THE ref_point
/// If epsg.is_some() the map is written georeferenced
/// else it is written in Local space
#[derive(Debug)]
pub struct Omap {
    elevation_scale_factor: f64,
    combined_scale_factor: f64,
    declination: f64,
    grivation: f64,
    scale: Scale,
    epsg: Option<u16>,
    ref_point: Coord,

    /// the objects of the map
    pub objects: HashMap<Symbol, Vec<MapObject>>,
}

impl Omap {
    /// Create a new map in the given scale with an optional CRS centered att georef_point which is meters_above_sea in elevation
    pub fn new(
        georef_point: Coord,
        scale: Scale,
        epsg_crs: Option<u16>,
        meters_above_sea: Option<f64>,
    ) -> Self {
        // uses the world magnetic model to figure out the declination (angle between true north and magnetic north) at the ref_point at the current time
        // and proj4rs for the convergence (angle between true north and grid north)
        //
        // the grivation (angle between magnetic north and grid north) must be used when calculating map coords as the axes are magnetic
        // grivation = declination - convergence
        //
        // the grid scale factor is calculated by the same algorithm as in OOmapper (I tried to do a 1:1 port)
        //
        // further the elevation factor (called auxiliary scale factor in OOmapper) relates real distances to ellipsoid distances
        // this is (ellipsoid_radius / (ellipsoid_radius + m_above_ellipsoid))
        //
        // ellipsoid_radius = R_equator * (1 - f * sin^2(lat))
        // f = 1 / 298.257223563
        // R_equator = 6378137.0m
        //
        // to calculate map units the combined scale factor and scale of map is needed to go from grid coordinates to real coordinates to map coordinates
        //
        // in summary to calculate map coordinates we need:
        // - a crs
        // - grivation (declination - convergence)
        // - the combined scale factor

        let declination = if let Some(epsg) = epsg_crs {
            Self::declination(epsg, georef_point, meters_above_sea).unwrap_or(0.)
        } else {
            0.
        };

        let (grid_scale_factor, elevation_scale_factor, convergence) = if let Some(epsg) = epsg_crs
        {
            Self::scale_factors_and_convergence(epsg, georef_point, meters_above_sea)
                .unwrap_or((1., 1., 0.))
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
            objects: HashMap::new(),
        }
    }

    /// reserve capacity for cap elements in key symbol in the objects hashmap
    pub fn reserve_capacity(&mut self, symbol: Symbol, cap: usize) {
        if let Some(obj) = self.objects.get_mut(&symbol) {
            obj.reserve(cap);
        } else {
            let _ = self.objects.insert(symbol, Vec::with_capacity(cap));
        }
    }

    /// insert an object in the objects hashmap
    pub fn add_object(&mut self, obj: MapObject) {
        let key = obj.symbol();
        if let Some(val) = self.objects.get_mut(&key) {
            val.push(obj);
        } else {
            let _ = self.objects.insert(key, vec![obj]);
        }
    }

    /// get the CRS of the map represented by an EPSG code
    pub fn get_crs(&self) -> Option<u16> {
        self.epsg
    }

    /// get the ref point of the map
    pub fn get_ref_point(&self) -> Coord {
        self.ref_point
    }

    /// merge line objects across tile bounds
    pub fn merge_lines(&mut self, delta: f64) {
        for (key, map_objects) in self.objects.iter_mut() {
            if !key.is_line_symbol() {
                continue;
            }
            let delta = delta * delta; // adjust delta as squared euclidean is used

            let mut unclosed_objects = Vec::with_capacity(map_objects.len());

            let mut i = 0;
            while i < map_objects.len() {
                if let MapObject::LineObject(o) = &map_objects[i] {
                    if !o.line.is_closed() {
                        unclosed_objects.push(map_objects.swap_remove(i));
                    } else {
                        i += 1;
                    }
                }
            }

            // check for elevation tags
            let mut group_memberships = vec![0; unclosed_objects.len()];

            let mut unique_elevations = HashMap::new();

            let mut has_elevation_tags = true;
            for (i, obj) in unclosed_objects.iter().enumerate() {
                if let MapObject::LineObject(o) = obj {
                    let elevation_tag = o.tags.get("Elevation");
                    if elevation_tag.is_none() {
                        has_elevation_tags = false;
                        break;
                    }
                    let elevation_tag = elevation_tag.unwrap().parse::<f32>();
                    if elevation_tag.is_err() {
                        has_elevation_tags = false;
                        break;
                    }

                    let elevation_tag = (elevation_tag.unwrap() * 100.) as i32;

                    let id = if unique_elevations.contains_key(&elevation_tag) {
                        *unique_elevations.get(&elevation_tag).unwrap()
                    } else {
                        let id = unique_elevations.len();
                        let _ = unique_elevations.insert(elevation_tag, id);
                        id
                    };

                    group_memberships[i] = id;
                }
            }
            let elevation_groups = if has_elevation_tags {
                unique_elevations.into_values().collect()
            } else {
                group_memberships = vec![0; unclosed_objects.len()];
                vec![0]
            };

            let mut unclosed_object_groups = vec![Vec::new(); elevation_groups.len()];

            for (i, unclosed_object) in unclosed_objects.into_iter().enumerate() {
                if let MapObject::LineObject(o) = unclosed_object {
                    let group = group_memberships[i];

                    unclosed_object_groups[group].push(o);
                }
            }

            for mut unclosed_objects in unclosed_object_groups {
                let (line_ends, line_starts): (Vec<_>, Vec<_>) = unclosed_objects
                    .iter()
                    .map(|o| {
                        let line_start = o.line.0[0];
                        let line_end = o.line.0[o.line.0.len() - 1];

                        ([line_end.x, line_end.y], [line_start.x, line_start.y])
                    })
                    .collect();

                // detect the merges needed
                let end_tree = kiddo::ImmutableKdTree::new_from_slice(line_ends.as_slice());

                let mut merges = Vec::with_capacity(line_starts.len());
                for (start_i, line_start) in line_starts.iter().enumerate() {
                    let nn = end_tree.nearest_one::<SquaredEuclidean>(line_start);
                    if nn.distance <= delta {
                        merges.push((start_i, nn.item as usize));
                    }
                }

                // start doing merges keeping track of the moved objects
                while let Some(merge) = merges.pop() {
                    if merge.0 == merge.1 {
                        let mut line = unclosed_objects.swap_remove(merge.0);
                        line.line.close();

                        map_objects.push(MapObject::LineObject(line));
                    } else {
                        // merge
                        let part2 = unclosed_objects.swap_remove(merge.0);

                        let part1 = if merge.1 >= unclosed_objects.len() {
                            &mut unclosed_objects[merge.0]
                        } else {
                            &mut unclosed_objects[merge.1]
                        };

                        let _ = part1.line.0.pop();
                        part1.line.0.extend(part2.line.0);
                    }
                    // update map
                    let mut i = 0;
                    while i < merges.len() {
                        let other_merge = &mut merges[i];

                        // find merges made impossible
                        if other_merge.1 == merge.1 || other_merge.0 == merge.0 {
                            let _ = merges.swap_remove(i);
                            continue;
                        } else {
                            i += 1;
                        }

                        // update map as merge.0 is now called merge.1
                        if other_merge.0 == merge.0 {
                            other_merge.0 = merge.1
                        }
                        if other_merge.1 == merge.0 {
                            other_merge.1 = merge.1
                        }

                        // correct map for swap remove moving object
                        if other_merge.0 >= unclosed_objects.len() {
                            other_merge.0 = merge.0;
                        }
                        if other_merge.1 >= unclosed_objects.len() {
                            other_merge.1 = merge.0;
                        }
                    }
                }
                let unclosed = unclosed_objects.into_iter().map(|mut line_object| {
                    // check if it is almost closed
                    let start = line_object.line.0[0];
                    let end = line_object.line.0[line_object.line.0.len() - 1];

                    if (start.x - end.x).powi(2) + (start.y - end.y).powi(2) <= delta {
                        line_object.line.close();
                    }

                    MapObject::LineObject(line_object)
                });

                map_objects.extend(unclosed);
            }
        }
    }

    /// turn small contour loops to dotknolls and depression and remove the smallest ones
    pub fn make_dotknolls_and_depressions(
        &mut self,
        min_area: f64,
        max_area: f64,
        elongated_aspect: f64,
    ) {
        let keys = [
            Symbol::Line(LineSymbol::Contour),
            Symbol::Line(LineSymbol::FormLine),
            Symbol::Line(LineSymbol::IndexContour),
        ];

        for key in keys {
            let contours = self.objects.get_mut(&key);

            if contours.is_none() {
                continue;
            }

            let contours = contours.unwrap();
            let mut small_loops = Vec::with_capacity(contours.len());

            let mut i = 0;
            while i < contours.len() {
                let contour_object = &contours[i];
                if let MapObject::LineObject(o) = contour_object {
                    if o.line.is_closed() {
                        let area = line_string_signed_area(&o.line);

                        if area.abs() <= max_area {
                            small_loops.push(contours.swap_remove(i));
                        } else {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                } else {
                    panic!("Non-line object under contour symbol in objects hashmap");
                }
            }

            for small_loop in small_loops {
                if let MapObject::LineObject(o) = &small_loop {
                    let area = line_string_signed_area(&o.line);

                    // ignore too small loops
                    if area.abs() < min_area {
                        continue;
                    }

                    let (aspect, mid_point, rotation) =
                        line_string_aspect_midpoint_rotation(&o.line);

                    if area < 0. {
                        let u_depression =
                            PointObject::from_point(Point(mid_point), PointSymbol::UDepression, 0.);
                        self.add_object(MapObject::PointObject(u_depression));
                    } else if aspect < elongated_aspect {
                        let dot_knoll =
                            PointObject::from_point(Point(mid_point), PointSymbol::DotKnoll, 0.);
                        self.add_object(MapObject::PointObject(dot_knoll));
                    } else {
                        let long_dot_knoll = PointObject::from_point(
                            Point(mid_point),
                            PointSymbol::ElongatedDotKnoll,
                            rotation,
                        );
                        self.add_object(MapObject::PointObject(long_dot_knoll));
                    }
                }
            }
        }
    }

    /// mark closed basemap contour loops wound clockwise as depressions
    pub fn mark_basemap_depressions(&mut self) {
        let basemap = self
            .objects
            .get_mut(&Symbol::Line(LineSymbol::BasemapContour));
        if basemap.is_none() {
            return;
        }

        let basemap = basemap.unwrap();

        let mut neg_basemap = Vec::new();

        let mut i = 0;
        while i < basemap.len() {
            if let MapObject::LineObject(o) = &basemap[i] {
                if o.line.is_closed() {
                    if line_string_signed_area(&o.line) < 0. {
                        neg_basemap.push(basemap.swap_remove(i));
                    } else {
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            } else {
                panic!("Non LineObject under Basemap symbol in objects hashmap");
            }
        }

        let _ = self
            .objects
            .insert(Symbol::Line(LineSymbol::NegBasemapContour), neg_basemap);
    }

    /// write the map an omap file,
    /// if path is an invalid path then "./auto_generated_map.omap" is the new path
    pub fn write_to_file(self, mut path: PathBuf, bezier_error: Option<f64>) -> OmapResult<()> {
        if path.extension() != Some(OsStr::new("omap")) {
            let _ = path.set_extension("omap");
        }

        let f = File::create(&path)?;
        let mut f = BufWriter::new(f);

        self.write_header(&mut f)?;
        self.write_colors_symbols(&mut f)?;
        self.write_objects(&mut f, bezier_error)?;
        Self::write_end_of_file(&mut f)?;
        Ok(())
    }
}

// private functions
impl Omap {
    fn write_header(&self, f: &mut BufWriter<File>) -> OmapResult<()> {
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
        f.write_all(include_str!("colors.txt").as_bytes())?;
        match self.scale {
            Scale::S10_000 => {
                f.write_all(include_str!("symbols_10.txt").as_bytes())?;
            }
            Scale::S15_000 => {
                f.write_all(include_str!("symbols_15.txt").as_bytes())?;
            }
        }
        Ok(())
    }

    fn write_objects(self, f: &mut BufWriter<File>, bezier_error: Option<f64>) -> OmapResult<()> {
        let num_objects = self.objects.values().fold(0, |acc, v| acc + v.len());

        f.write_all(
            format!(
                "<parts count=\"1\" current=\"0\">\n<part name=\"map\"><objects count=\"{num_objects}\">\n"
            )
            .as_bytes(),
        )?;

        for sym_vals in self.objects.into_values() {
            for obj in sym_vals {
                obj.write_to_map(
                    f,
                    bezier_error,
                    self.scale,
                    self.grivation,
                    self.combined_scale_factor,
                )?;
            }
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

    fn scale_factors_and_convergence(
        epsg: u16,
        ref_point: Coord,
        meters_above_sea_level: Option<f64>,
    ) -> OmapResult<(f64, f64, f64)> {
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

        //re-project the points down to the grid
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

        let elevation_scale_factor = if let Some(meters_above_sea_level) = meters_above_sea_level {
            // this is (ellipsoid_radius / (ellipsoid_radius + m_above_ellipsoid))
            //
            // ellipsoid_radius = R_equator * (1 - f * sin^2(lat))
            // f = 1 / 298.257223563
            // R_equator = 6378137.0m
            const F: f64 = 1. / 298.257223563;
            const R_EQUATOR: f64 = 6378137.;

            let ellipsoid_radius = R_EQUATOR * (1. - F * geo_ref_point.0.sin().powi(2));

            ellipsoid_radius / (ellipsoid_radius + meters_above_sea_level)
        } else {
            1.
        };

        Ok((grid_scale_factor, elevation_scale_factor, convergence))
    }

    fn declination(
        epsg: u16,
        ref_point: Coord,
        meters_above_sea_level: Option<f64>,
    ) -> OmapResult<f64> {
        let geographic_proj = Proj::from_epsg_code(4326)?;
        let local_proj = Proj::from_epsg_code(epsg)?;

        // transform ref_point to lat/lon
        let mut geo_ref_point = (ref_point.x, ref_point.y);
        transform(&local_proj, &geographic_proj, &mut geo_ref_point)?;

        let date = chrono::Local::now();
        let year = date.year();
        let day = date.ordinal() as u16;

        let field = GeomagneticField::new(
            Length::new::<meter>(meters_above_sea_level.unwrap_or(0.) as f32),
            Angle::new::<radian>(geo_ref_point.1 as f32),
            Angle::new::<radian>(geo_ref_point.0 as f32),
            Date::from_ordinal_date(year, day)
                .unwrap_or(Date::from_ordinal_date(2025, 180).unwrap()),
        )?;
        let dec = field.declination().get::<radian>();

        Ok(dec as f64)
    }
}

fn line_string_signed_area(line: &LineString) -> f64 {
    if line.0.len() < 3 {
        return 0.;
    }
    let mut area: f64 = 0.;
    for i in 0..line.0.len() - 1 {
        area += line.0[i].x * line.0[i + 1].y - line.0[i].y * line.0[i + 1].x;
    }
    0.5 * area
}

fn line_string_aspect_midpoint_rotation(line: &LineString) -> (f64, Coord, f64) {
    let mut midpoint = Coord::zero();
    for c in line.0.iter() {
        midpoint = midpoint + *c;
    }
    midpoint = midpoint / line.0.len() as f64;

    // Calculate second moments
    let mu20 = line
        .0
        .iter()
        .map(|p| (p.x - midpoint.x).powi(2))
        .sum::<f64>();
    let mu02 = line
        .0
        .iter()
        .map(|p| (p.y - midpoint.y).powi(2))
        .sum::<f64>();
    let mu11 = line
        .0
        .iter()
        .map(|p| (p.x - midpoint.x) * (p.y - midpoint.y))
        .sum::<f64>();

    // Calculate elongation using eigenvalues of the covariance matrix
    let temp = ((mu20 - mu02).powi(2) + 4.0 * mu11.powi(2)).sqrt();
    let lambda1 = (mu20 + mu02 + temp) / 2.0;
    let lambda2 = (mu20 + mu02 - temp) / 2.0;

    // Handle potential numerical issues
    if lambda2.abs() <= 2. * f64::EPSILON
        || ((mu20 - mu02).abs() <= 2. * f64::EPSILON && mu11.abs() <= 2. * f64::EPSILON)
    {
        return (1., midpoint, 0.);
    }

    let elongation = (lambda1 / lambda2).sqrt();

    // Calculate the angle of the major axis (in radians)
    // The eigenvector corresponding to the largest eigenvalue gives the direction
    let mut angle = 0.5 * f64::atan2(2.0 * mu11, mu20 - mu02);

    // Ensure the angle corresponds to the major (not minor) axis
    if !(mu20 < mu02 || mu11 >= 0.0) {
        angle += std::f64::consts::FRAC_PI_2;
    }

    angle %= std::f64::consts::PI;
    if angle < 0.0 {
        angle += std::f64::consts::PI;
    }

    (elongation, midpoint, angle)
}
