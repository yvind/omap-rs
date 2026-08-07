use geo_types::{Coord, LineString, Point, Polygon};
use linestring2bezier::{BezierSegment, BezierString};
#[cfg(feature = "geo_ref")]
use proj_core::Transform;

#[cfg(feature = "geo_ref")]
use crate::Error;
use crate::{
    Result,
    geo_referencing::CrsType,
    objects::{BezierPath, BezierPolygon},
};

use super::GeoRef;

/// Coordinate transform between map and projected (CRS) coordinates.
#[cfg_attr(
    feature = "geo_ref",
    doc = "",
    doc = "With the `geo_ref` feature the same transform also reaches WGS84,",
    doc = "`x` longitude and `y` latitude in degrees, through the",
    doc = "[`to_wgs84`](Self::to_wgs84) and [`from_wgs84`](Self::from_wgs84)",
    doc = "families, which chain the paper to projected step with a projection",
    doc = "between the map's CRS and WGS84."
)]
#[derive(Debug, Clone)]
pub struct MapTransform {
    map_center: Coord,
    proj_center: Coord,
    scale_factor: f64,
    sin: f64,
    cos: f64,
    /// The CRS definition used by this transform.
    crs_type: CrsType,
    /// The WGS84 transform pair
    #[cfg(feature = "geo_ref")]
    wgs84_transform: Option<Wgs84Transforms>,
}

/// The compiled projections between the map's CRS and WGS84.
#[cfg(feature = "geo_ref")]
#[derive(Debug, Clone)]
struct Wgs84Transforms {
    to_wgs84: Transform,
    from_wgs84: Transform,
}

impl MapTransform {
    /// Convert a [Coord] in projected (CRS) coordinates to map coordinates.
    pub fn to_map(&self, proj_coord: Coord) -> Coord {
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

    /// Convert a [`BezierPolygon`] in projected (CRS) coordinates to map coordinates.
    pub fn to_map_bezierpolygon(&self, proj_bezierpolygon: BezierPolygon) -> BezierPolygon {
        proj_bezierpolygon.transform(|coord| self.to_map(coord))
    }

    /// Convert a [`BezierPath`] in projected (CRS) coordinates to map coordinates.
    pub fn to_map_bezierpath(&self, proj_bezierpath: BezierPath) -> BezierPath {
        proj_bezierpath.transform(|coord| self.to_map(coord))
    }

    /// Convert a [`LineString`] in projected (CRS) coordinates to map coordinates.
    pub fn to_map_linestring(&self, proj_linestring: LineString) -> LineString {
        proj_linestring
            .into_inner()
            .into_iter()
            .map(|c| self.to_map(c))
            .collect::<LineString>()
    }

    /// Convert a [`BezierString`] in projected (CRS) coordinates to map coordinates.
    pub fn to_map_bezierstring(&self, proj_bezierstring: BezierString) -> BezierString {
        transform_bezierstring(proj_bezierstring, |coord| self.to_map(coord))
    }

    /// Convert a [Point] in projected (CRS) coordinates to map coordinates.
    pub fn to_map_point(&self, proj_point: Point) -> Point {
        self.to_map(proj_point.0).into()
    }

    /// Convert a [Coord] in map coordinates to projected (CRS) coordinates.
    pub fn to_projected(&self, map_coord: Coord) -> Coord {
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

    /// Convert a [`BezierPolygon`] in map coordinates to projected (CRS) coordinates.
    pub fn to_projected_bezierpolygon(&self, map_bezierpolygon: BezierPolygon) -> BezierPolygon {
        map_bezierpolygon.transform(|coord| self.to_projected(coord))
    }

    /// Convert a [`BezierPath`] in map coordinates to projected (CRS) coordinates.
    pub fn to_projected_bezierpath(&self, map_bezierpath: BezierPath) -> BezierPath {
        map_bezierpath.transform(|coord| self.to_projected(coord))
    }

    /// Convert a [`LineString`] in map coordinates to projected (CRS) coordinates.
    pub fn to_projected_linestring(&self, map_linestring: LineString) -> LineString {
        map_linestring
            .into_inner()
            .into_iter()
            .map(|c| self.to_projected(c))
            .collect::<LineString>()
    }

    /// Convert a [`BezierString`] in map coordinates to projected (CRS) coordinates.
    pub fn to_projected_bezierstring(&self, map_bezierstring: BezierString) -> BezierString {
        transform_bezierstring(map_bezierstring, |coord| self.to_projected(coord))
    }

    /// Convert a [Point] in map coordinates to projected (CRS) coordinates.
    pub fn to_projected_point(&self, proj_point: Point) -> Point {
        self.to_projected(proj_point.0).into()
    }

    /// The ground distance, in the projection's units, spanned by one
    /// millimeter of map
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Like [`Self::from_geo_ref`], but reports why the CRS could not be
    /// related to WGS84 rather than storing the absence and failing later.
    #[cfg(feature = "geo_ref")]
    pub(super) fn try_from_geo_ref(geo_ref: &GeoRef) -> Result<Self> {
        Ok(Self {
            map_center: geo_ref.map_ref_point,
            proj_center: geo_ref.projected_ref_point,
            sin: geo_ref.grivation_deg().to_radians().sin(),
            cos: geo_ref.grivation_deg().to_radians().cos(),
            scale_factor: geo_ref.scale_factor(),
            crs_type: geo_ref.crs_type.clone(),
            wgs84_transform: Some(Self::wgs84_transforms(&geo_ref.crs_type)?),
        })
    }

    pub(super) fn from_geo_ref(geo_ref: &GeoRef) -> Self {
        Self {
            map_center: geo_ref.map_ref_point,
            proj_center: geo_ref.projected_ref_point,
            sin: geo_ref.grivation_deg().to_radians().sin(),
            cos: geo_ref.grivation_deg().to_radians().cos(),
            scale_factor: geo_ref.scale_factor(),
            crs_type: geo_ref.crs_type.clone(),
            #[cfg(feature = "geo_ref")]
            wgs84_transform: Self::wgs84_transforms(&geo_ref.crs_type).ok(),
        }
    }

    /// Compute a transform from the `old` map coordinate frame to the `new`
    /// one, preserving real-world positions.
    ///
    /// Use this when changing the map's georeferencing and you need to
    /// transform all existing map objects and non-georeferenced templates so
    /// they remain at the same real-world locations.
    ///
    /// Without the `geo_ref` feature, the coordinate reference systems must be
    /// identical. With `geo_ref`, differing coordinate reference systems are
    /// converted using `proj-core`.
    ///
    /// # Errors
    ///
    /// Returns an error if the coordinate reference systems cannot be related.
    pub fn transform_between(
        old: &Self,
        new: &Self,
    ) -> Result<Box<dyn Fn(Coord) -> Result<Coord>>> {
        if old.crs_type == new.crs_type {
            let old = old.clone();
            let new = new.clone();
            return Ok(Box::new(move |coord| {
                Ok(new.to_map(old.to_projected(coord)))
            }));
        }

        #[cfg(feature = "geo_ref")]
        {
            let projection = Transform::from_horizontal_components(
                &old.crs_type.to_crs_def()?,
                &new.crs_type.to_crs_def()?,
            )?;
            let old = old.clone();
            let new = new.clone();

            Ok(Box::new(move |coord| {
                let projected = projection.convert(old.to_projected(coord))?;
                Ok(new.to_map(projected))
            }))
        }

        #[cfg(not(feature = "geo_ref"))]
        Err(crate::Error::CannotGetTransformBetweenDifferentGeoRef)
    }
}

#[cfg(feature = "geo_ref")]
impl MapTransform {
    /// Build the projections between the map's CRS and WGS84
    fn wgs84_transforms(crs_type: &CrsType) -> Result<Wgs84Transforms> {
        let geographic_crs = proj_wkt::parse_crs("EPSG:4326")?;
        let to_wgs84 =
            Transform::from_horizontal_components(&crs_type.to_crs_def()?, &geographic_crs)?;
        let from_wgs84 = to_wgs84.inverse()?;

        Ok(Wgs84Transforms {
            to_wgs84,
            from_wgs84,
        })
    }

    /// Convert a [Coord] in map coordinates to WGS84 degrees, `x` longitude
    /// and `y` latitude.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn to_wgs84(&self, map_coord: Coord) -> Result<Coord> {
        Ok(self
            .wgs84_transform
            .as_ref()
            .ok_or(Error::NoWGS84TransformAvailable)?
            .to_wgs84
            .convert(self.to_projected(map_coord))?)
    }

    /// Convert a [Polygon] in map coordinates to WGS84 degrees.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn to_wgs84_polygon(&self, map_polygon: Polygon) -> Result<Polygon> {
        Ok(self
            .wgs84_transform
            .as_ref()
            .ok_or(Error::NoWGS84TransformAvailable)?
            .to_wgs84
            .convert_geometry(self.to_projected_polygon(map_polygon))?)
    }

    /// Convert a [`BezierPolygon`] in map coordinates to WGS84 degrees.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn to_wgs84_bezierpolygon(
        &self,
        map_bezierpolygon: BezierPolygon,
    ) -> Result<BezierPolygon> {
        map_bezierpolygon.try_transform(|coord| self.to_wgs84(coord))
    }

    /// Convert a [`BezierPath`] in map coordinates to WGS84 degrees.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn to_wgs84_bezierpath(&self, map_bezierpath: BezierPath) -> Result<BezierPath> {
        map_bezierpath.try_transform(|coord| {
            Ok(self
                .wgs84_transform
                .as_ref()
                .ok_or(Error::NoWGS84TransformAvailable)?
                .to_wgs84
                .convert(self.to_projected(coord))?)
        })
    }

    /// Convert a [`LineString`] in map coordinates to WGS84 degrees.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn to_wgs84_linestring(&self, map_linestring: LineString) -> Result<LineString> {
        Ok(self
            .wgs84_transform
            .as_ref()
            .ok_or(Error::NoWGS84TransformAvailable)?
            .to_wgs84
            .convert_geometry(self.to_projected_linestring(map_linestring))?)
    }

    /// Convert a [`BezierString`] in map coordinates to WGS84 degrees.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn to_wgs84_bezierstring(&self, map_bezierstring: BezierString) -> Result<BezierString> {
        try_transform_bezierstring(map_bezierstring, |coord| {
            Ok(self
                .wgs84_transform
                .as_ref()
                .ok_or(Error::NoWGS84TransformAvailable)?
                .to_wgs84
                .convert(self.to_projected(coord))?)
        })
    }

    /// Convert a [Point] in map coordinates to WGS84 degrees.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn to_wgs84_point(&self, map_point: Point) -> Result<Point> {
        Ok(self.to_wgs84(map_point.0)?.into())
    }

    /// Convert a [Coord] in WGS84 degrees, `x` longitude and `y` latitude, to
    /// map coordinates.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn from_wgs84(&self, wgs84_coord: Coord) -> Result<Coord> {
        Ok(self.to_map(
            self.wgs84_transform
                .as_ref()
                .ok_or(Error::NoWGS84TransformAvailable)?
                .from_wgs84
                .convert(wgs84_coord)?,
        ))
    }

    /// Convert a [Polygon] in WGS84 degrees to map coordinates.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn from_wgs84_polygon(&self, wgs84_polygon: Polygon) -> Result<Polygon> {
        let projected = self
            .wgs84_transform
            .as_ref()
            .ok_or(Error::NoWGS84TransformAvailable)?
            .from_wgs84
            .convert_geometry(wgs84_polygon)?;

        Ok(self.to_map_polygon(projected))
    }

    /// Convert a [`BezierPolygon`] in WGS84 degrees to map coordinates.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn from_wgs84_bezierpolygon(
        &self,
        wgs84_bezierpolygon: BezierPolygon,
    ) -> Result<BezierPolygon> {
        wgs84_bezierpolygon.try_transform(|coord| self.from_wgs84(coord))
    }

    /// Convert a [`BezierPath`] in WGS84 degrees to map coordinates.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn from_wgs84_bezierpath(&self, wgs84_bezierpath: BezierPath) -> Result<BezierPath> {
        wgs84_bezierpath.try_transform(|coord| {
            Ok(self.to_map(
                self.wgs84_transform
                    .as_ref()
                    .ok_or(Error::NoWGS84TransformAvailable)?
                    .from_wgs84
                    .convert(coord)?,
            ))
        })
    }

    /// Convert a [`LineString`] in WGS84 degrees to map coordinates.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn from_wgs84_linestring(&self, wgs84_linestring: LineString) -> Result<LineString> {
        let projected = self
            .wgs84_transform
            .as_ref()
            .ok_or(Error::NoWGS84TransformAvailable)?
            .from_wgs84
            .convert_geometry(wgs84_linestring)?;

        Ok(self.to_map_linestring(projected))
    }

    /// Convert a [`BezierString`] in WGS84 degrees to map coordinates.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn from_wgs84_bezierstring(
        &self,
        wgs84_bezierstring: BezierString,
    ) -> Result<BezierString> {
        try_transform_bezierstring(wgs84_bezierstring, |coord| {
            Ok(self.to_map(
                self.wgs84_transform
                    .as_ref()
                    .ok_or(Error::NoWGS84TransformAvailable)?
                    .from_wgs84
                    .convert(coord)?,
            ))
        })
    }

    /// Convert a [Point] in WGS84 degrees to map coordinates.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn from_wgs84_point(&self, wgs84_point: Point) -> Result<Point> {
        Ok(self.from_wgs84(wgs84_point.0)?.into())
    }
}

/// Apply a fallible coordinate transform to a [`BezierString`], stopping at the
/// first failure.
#[cfg(feature = "geo_ref")]
fn try_transform_bezierstring(
    mut bezierstring: BezierString,
    transform: impl Fn(Coord) -> Result<Coord>,
) -> Result<BezierString> {
    for segment in bezierstring.segments_mut() {
        match segment {
            BezierSegment::Bezier(curve) => {
                curve.start = transform(curve.start)?;
                curve.handle1 = transform(curve.handle1)?;
                curve.handle2 = transform(curve.handle2)?;
                curve.end = transform(curve.end)?;
            }
            BezierSegment::Line(line) => {
                line.start = transform(line.start)?;
                line.end = transform(line.end)?;
            }
        }
    }
    Ok(bezierstring)
}

fn transform_bezierstring(
    mut bezierstring: BezierString,
    transform: impl Fn(Coord) -> Coord,
) -> BezierString {
    for segment in bezierstring.segments_mut() {
        match segment {
            BezierSegment::Bezier(curve) => {
                curve.start = transform(curve.start);
                curve.handle1 = transform(curve.handle1);
                curve.handle2 = transform(curve.handle2);
                curve.end = transform(curve.end);
            }
            BezierSegment::Line(line) => {
                line.start = transform(line.start);
                line.end = transform(line.end);
            }
        }
    }
    bezierstring
}

#[cfg(test)]
mod tests {
    use geo_types::{Coord, coord};

    use super::MapTransform;
    use crate::geo_referencing::{CrsType, GeoRef};

    fn map_transform(crs_type: CrsType, projected_ref_point: Coord) -> MapTransform {
        GeoRef {
            scale_denominator: 15_000,
            grid_scale_factor: 1.,
            auxiliary_scale_factor: 1.,
            declination_deg: 2.,
            convergence_deg: -1.,
            crs_type,
            map_ref_point: coord! { x: 25., y: -50. },
            projected_ref_point,
            geographic_ref_point_deg: Coord::zero(),
        }
        .get_transform()
    }

    #[test]
    fn transform_between_preserves_coordinates_in_the_same_projection() -> crate::Result<()> {
        let old = map_transform(CrsType::Utm(33), coord! { x: 500_000., y: 6_600_000. });
        let mut new_geo_ref = GeoRef {
            scale_denominator: 10_000,
            grid_scale_factor: 0.9998,
            auxiliary_scale_factor: 1.0001,
            declination_deg: -3.,
            convergence_deg: 1.,
            crs_type: CrsType::Utm(33),
            map_ref_point: coord! { x: -100., y: 75. },
            projected_ref_point: coord! { x: 500_250., y: 6_599_800. },
            geographic_ref_point_deg: Coord::zero(),
        };
        let new = new_geo_ref.get_transform();
        let input = coord! { x: 12.5, y: -8.25 };

        let transform = MapTransform::transform_between(&old, &new)?;
        let transformed = transform(input)?;
        let expected = old.to_projected(input);
        let actual = new.to_projected(transformed);

        assert!((expected.x - actual.x).abs() < 1e-8);
        assert!((expected.y - actual.y).abs() < 1e-8);

        // The returned transform owns everything it needs.
        new_geo_ref.projected_ref_point = Coord::zero();
        assert_eq!(transform(input)?, transformed);
        Ok(())
    }

    #[cfg(not(feature = "geo_ref"))]
    #[test]
    fn transform_between_rejects_different_projections_without_geo_ref() {
        let old = map_transform(CrsType::Utm(33), coord! { x: 500_000., y: 6_600_000. });
        let new = map_transform(CrsType::Utm(32), coord! { x: 500_000., y: 6_600_000. });

        assert!(matches!(
            MapTransform::transform_between(&old, &new),
            Err(crate::Error::CannotGetTransformBetweenDifferentGeoRef)
        ));
    }

    #[cfg(feature = "geo_ref")]
    #[test]
    fn transform_between_converts_different_projections_with_geo_ref() -> crate::Result<()> {
        let old = map_transform(CrsType::Utm(33), coord! { x: 500_000., y: 6_600_000. });
        let new = map_transform(CrsType::Utm(32), coord! { x: 500_000., y: 6_600_000. });
        let input = coord! { x: 12.5, y: -8.25 };

        let transform = MapTransform::transform_between(&old, &new)?;
        let transformed = transform(input)?;
        let expected = old.to_wgs84(input)?;
        let actual = new.to_wgs84(transformed)?;

        assert!((expected.x - actual.x).abs() < 1e-9);
        assert!((expected.y - actual.y).abs() < 1e-9);
        Ok(())
    }
}
