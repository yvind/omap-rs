use geo_types::{Coord, LineString, Point, Polygon};

use super::GeoRef;

/// Coordinate transform between map and projected (CRS) coordinates.
#[derive(Debug, Clone)]
pub struct MapTransform {
    map_center: Coord,
    proj_center: Coord,
    scale_factor: f64,
    sin: f64,
    cos: f64,
}

impl MapTransform {
    /// Convert a [Coord] in projected (CRS) coordinates to map coordinates.
    pub fn to_map_coordinates(&self, proj_coord: Coord) -> Coord {
        let (x, mut y) = ((proj_coord - self.proj_center) / self.scale_factor).x_y();

        let x_r = x * self.cos - y * self.sin;
        y = x * self.sin + y * self.cos;

        Coord { x: x_r, y } + self.map_center
    }

    /// Convert a [Polygon] in projected (CRS) coordinates to map coordinates.
    pub fn to_map_polygon(&self, proj_polygon: Polygon) -> Polygon {
        let (ext, ints) = proj_polygon.into_inner();

        let map_ext = self.to_map_linestring(ext);
        let map_ints = ints
            .into_iter()
            .map(|l| self.to_map_linestring(l))
            .collect::<Vec<_>>();

        Polygon::new(map_ext, map_ints)
    }

    /// Convert a [LineString] in projected (CRS) coordinates to map coordinates.
    pub fn to_map_linestring(&self, proj_linestring: LineString) -> LineString {
        proj_linestring
            .into_inner()
            .into_iter()
            .map(|c| self.to_map_coordinates(c))
            .collect::<LineString>()
    }

    /// Convert a [Point] in projected (CRS) coordinates to map coordinates.
    pub fn to_map_point(&self, proj_point: Point) -> Point {
        self.to_map_coordinates(proj_point.0).into()
    }

    /// Convert a [Coord] in map coordinates to projected (CRS) coordinates.
    pub fn to_projected_coordinates(&self, map_coord: Coord) -> Coord {
        let (x, mut y) = ((map_coord - self.map_center) * self.scale_factor).x_y();

        // we want to rotate other way so flip the signs of the sins
        let x_r = x * self.cos + y * self.sin;
        y = -x * self.sin + y * self.cos;

        Coord { x: x_r, y } + self.proj_center
    }

    /// Convert a [Polygon] in map coordinates to projected (CRS) coordinates.
    pub fn to_projected_polygon(&self, map_polygon: Polygon) -> Polygon {
        let (ext, ints) = map_polygon.into_inner();

        let map_ext = self.to_projected_linestring(ext);
        let map_ints = ints
            .into_iter()
            .map(|l| self.to_projected_linestring(l))
            .collect::<Vec<_>>();

        Polygon::new(map_ext, map_ints)
    }

    /// Convert a [LineString] in map coordinates to projected (CRS) coordinates.
    pub fn to_projected_linestring(&self, map_linestring: LineString) -> LineString {
        map_linestring
            .into_inner()
            .into_iter()
            .map(|c| self.to_projected_coordinates(c))
            .collect::<LineString>()
    }

    /// Convert a [Point] in map coordinates to projected (CRS) coordinates.
    pub fn to_projected_point(&self, proj_point: Point) -> Point {
        self.to_projected_coordinates(proj_point.0).into()
    }

    pub(super) fn from_geo_ref(geo_ref: &GeoRef) -> Self {
        Self {
            map_center: geo_ref.map_ref_point,
            proj_center: geo_ref.projected_ref_point,
            sin: geo_ref.grivation_deg().to_radians().sin(),
            cos: geo_ref.grivation_deg().to_radians().cos(),
            scale_factor: geo_ref.combined_scale_factor() * geo_ref.scale_denominator as f64
                / 1000.,
        }
    }
}
