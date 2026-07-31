#[cfg(feature = "geo_ref")]
use std::cell::Cell;
use std::hash::{DefaultHasher, Hash as _, Hasher as _};

use geo_types::{Coord, LineString, Point, Polygon};
use linestring2bezier::{BezierSegment, BezierString};
#[cfg(feature = "geo_ref")]
use proj_core::Transform;

use crate::objects::{BezierPath, BezierPolygon};
#[cfg(feature = "geo_ref")]
use crate::{Error, Result, geo_referencing::CrsType};

use super::GeoRef;

/// A 2D affine coordinate transform (rotation + uniform scale + translation).
///
/// Represents the function `f(c) = scale * rotate(c) + translation`.
/// Obtained via [`MapTransform::affine_between`] to re-project all map
/// objects and non-georeferenced templates when the georeferencing changes
/// within a projected CRS.
#[derive(Debug, Clone)]
pub struct AffineMapTransform {
    /// Cosine component of the rotation.
    cos: f64,
    /// Sine component of the rotation.
    sin: f64,
    /// Uniform scale factor.
    scale: f64,
    /// Translation applied after rotation and scale.
    translation: Coord,
}

impl AffineMapTransform {
    /// Apply the affine transform to a single coordinate.
    pub fn apply(&self, coord: Coord) -> Coord {
        let x =
            coord.x * self.scale * self.cos - coord.y * self.scale * self.sin + self.translation.x;
        let y =
            coord.x * self.scale * self.sin + coord.y * self.scale * self.cos + self.translation.y;
        Coord { x, y }
    }

    pub(crate) fn rotation_radians(&self) -> f64 {
        self.sin.atan2(self.cos)
    }

    pub(crate) fn scale_factor(&self) -> f64 {
        self.scale
    }

    pub(crate) fn file_coord_matrix(&self) -> [f64; 9] {
        [
            self.scale * self.cos,
            self.scale * self.sin,
            self.translation.x,
            -self.scale * self.sin,
            self.scale * self.cos,
            -self.translation.y,
            0.,
            0.,
            1.,
        ]
    }
}

/// Coordinate transform between map and projected (CRS) coordinates.
#[cfg_attr(
    feature = "geo_ref",
    doc = "",
    doc = "With the `geo_ref` feature the same transform also reaches WGS84 —",
    doc = "`x` longitude and `y` latitude, in degrees — through the",
    doc = "[`to_wgs84`](Self::to_wgs84) and [`from_wgs84`](Self::from_wgs84)",
    doc = "families, which chain the paper ↔ projected step with a projection",
    doc = "between the map's CRS and WGS84."
)]
#[derive(Debug, Clone)]
pub struct MapTransform {
    map_center: Coord,
    proj_center: Coord,
    scale_factor: f64,
    sin: f64,
    cos: f64,
    /// Lightweight fingerprint of the CRS definition.
    ///
    /// This is a conservative guard for [`MapTransform::affine_between`]: affine
    /// transforms are only attempted when both transforms were built from the
    /// same CRS representation. This crate does not try to normalize equivalent
    /// CRS definitions.
    crs_hash: u64,
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
        BezierPolygon {
            exterior: self.to_map_bezierpath(proj_bezierpolygon.exterior),
            interiors: proj_bezierpolygon
                .interiors
                .into_iter()
                .map(|ring| self.to_map_bezierpath(ring))
                .collect(),
        }
    }

    /// Convert a [`BezierPath`] in projected (CRS) coordinates to map coordinates.
    pub fn to_map_bezierpath(&self, proj_bezierpath: BezierPath) -> BezierPath {
        proj_bezierpath.map_coords(|coord| self.to_map(coord))
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
        BezierPolygon {
            exterior: self.to_projected_bezierpath(map_bezierpolygon.exterior),
            interiors: map_bezierpolygon
                .interiors
                .into_iter()
                .map(|ring| self.to_projected_bezierpath(ring))
                .collect(),
        }
    }

    /// Convert a [`BezierPath`] in map coordinates to projected (CRS) coordinates.
    pub fn to_projected_bezierpath(&self, map_bezierpath: BezierPath) -> BezierPath {
        map_bezierpath.map_coords(|coord| self.to_projected(coord))
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

    pub(super) fn from_geo_ref(geo_ref: &GeoRef) -> Self {
        let mut hasher = DefaultHasher::new();
        geo_ref.crs_type.hash(&mut hasher);
        let crs_hash = hasher.finish();

        Self {
            map_center: geo_ref.map_ref_point,
            proj_center: geo_ref.projected_ref_point,
            sin: geo_ref.grivation_deg().to_radians().sin(),
            cos: geo_ref.grivation_deg().to_radians().cos(),
            scale_factor: geo_ref.combined_scale_factor() * geo_ref.scale_denominator as f64
                / 1000.,
            crs_hash,
            #[cfg(feature = "geo_ref")]
            wgs84_transform: Self::wgs84_transforms(&geo_ref.crs_type).ok(),
        }
    }

    /// Compute the affine transform that maps coordinates from the `old`
    /// coordinate frame to the `new` one, preserving projected (geographic)
    /// positions.
    ///
    /// Use this when changing the map's georeferencing and you need to
    /// transform all existing map objects and non-georeferenced templates so
    /// they remain at the same real-world locations.
    ///
    /// This is intentionally conservative: the two transforms must have matching
    /// CRS fingerprints. Equivalent CRS definitions written in different forms
    /// may be rejected, because `omap-rs` does not act as a projection library.
    ///
    /// The returned [`AffineMapTransform`] can be applied to every object via [`crate::Omap::apply_affine`].
    ///
    /// # Errors
    ///
    /// Returns
    /// [`crate::Error::CannotGetAffineTransformBetweenDifferentProjections`]
    /// if the transforms have different CRS fingerprints.
    pub fn affine_between(old: &Self, new: &Self) -> crate::Result<AffineMapTransform> {
        if old.crs_hash != new.crs_hash {
            return Err(crate::Error::CannotGetAffineTransformBetweenDifferentProjections);
        }

        // The composition is: new.to_map(old.to_projected(coord))
        // old.to_projected: rotate by (-old.cos, old.sin), scale by old.scale_factor, translate by old.proj_center
        // new.to_map: translate by -new.proj_center, scale by 1/new.scale_factor, rotate by (new.cos, new.sin), translate by new.map_center
        //
        // Combined rotation: angle = new_grivation - old_grivation (but we compose the sin/cos directly)
        // Combined scale: old.scale_factor / new.scale_factor

        let scale = old.scale_factor / new.scale_factor;

        let cos = new.cos * old.cos + new.sin * old.sin;
        let sin = new.sin * old.cos - new.cos * old.sin;

        let diff = old.proj_center - new.proj_center;
        let inv_scale = 1.0 / new.scale_factor;
        let rotated_diff = Coord {
            x: diff.x * new.cos - diff.y * new.sin,
            y: diff.x * new.sin + diff.y * new.cos,
        };
        let new_origin = Coord {
            x: rotated_diff.x * inv_scale + new.map_center.x,
            y: rotated_diff.y * inv_scale + new.map_center.y,
        };

        let rot_old_center = Coord {
            x: old.map_center.x * cos - old.map_center.y * sin,
            y: old.map_center.x * sin + old.map_center.y * cos,
        };
        let translation = Coord {
            x: new_origin.x - scale * rot_old_center.x,
            y: new_origin.y - scale * rot_old_center.y,
        };

        Ok(AffineMapTransform {
            cos,
            sin,
            scale,
            translation,
        })
    }
}

#[cfg(feature = "geo_ref")]
impl MapTransform {
    /// Build the projections between the map's CRS and WGS84
    fn wgs84_transforms(crs_type: &CrsType) -> Result<Wgs84Transforms> {
        let geographic_crs = proj_wkt::parse_crs("EPSG:4326")?;
        let to_wgs84 = Transform::from_crs_defs(&crs_type.to_crs_def()?, &geographic_crs)?;
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
            .ok_or(Error::CombinedLineSymbolContainsNonLine)?
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
            .ok_or(Error::CombinedLineSymbolContainsNonLine)?
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
        let mut interiors = Vec::with_capacity(map_bezierpolygon.interiors.len());
        for ring in map_bezierpolygon.interiors {
            interiors.push(self.to_wgs84_bezierpath(ring)?);
        }

        Ok(BezierPolygon {
            exterior: self.to_wgs84_bezierpath(map_bezierpolygon.exterior)?,
            interiors,
        })
    }

    /// Convert a [`BezierPath`] in map coordinates to WGS84 degrees.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn to_wgs84_bezierpath(&self, map_bezierpath: BezierPath) -> Result<BezierPath> {
        try_transform_bezierpath(map_bezierpath, |coord| {
            Ok(self
                .wgs84_transform
                .as_ref()
                .ok_or(Error::CombinedLineSymbolContainsNonLine)?
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
            .ok_or(Error::CombinedLineSymbolContainsNonLine)?
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
                .ok_or(Error::CombinedLineSymbolContainsNonLine)?
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
                .ok_or(Error::CombinedLineSymbolContainsNonLine)?
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
            .ok_or(Error::CombinedLineSymbolContainsNonLine)?
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
        let mut interiors = Vec::with_capacity(wgs84_bezierpolygon.interiors.len());
        for ring in wgs84_bezierpolygon.interiors {
            interiors.push(self.from_wgs84_bezierpath(ring)?);
        }

        Ok(BezierPolygon {
            exterior: self.from_wgs84_bezierpath(wgs84_bezierpolygon.exterior)?,
            interiors,
        })
    }

    /// Convert a [`BezierPath`] in WGS84 degrees to map coordinates.
    ///
    /// # Errors
    ///
    /// Returns an error if the map's CRS cannot be related to WGS84, or if a
    /// coordinate falls outside the transform's domain.
    pub fn from_wgs84_bezierpath(&self, wgs84_bezierpath: BezierPath) -> Result<BezierPath> {
        try_transform_bezierpath(wgs84_bezierpath, |coord| {
            Ok(self.to_map(
                self.wgs84_transform
                    .as_ref()
                    .ok_or(Error::CombinedLineSymbolContainsNonLine)?
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
            .ok_or(Error::CombinedLineSymbolContainsNonLine)?
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
                    .ok_or(Error::CombinedLineSymbolContainsNonLine)?
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

/// Apply a fallible coordinate transform to a [`BezierPath`].
///
/// [`BezierPath::map_coords`] upholds the path's invariants but takes an
/// infallible transform, so a failure is carried out of the closure instead of
/// returned from it. Later coordinates are still visited; the reported failure
/// is the last one.
#[cfg(feature = "geo_ref")]
fn try_transform_bezierpath(
    bezierpath: BezierPath,
    transform: impl Fn(Coord) -> Result<Coord>,
) -> Result<BezierPath> {
    let failure = Cell::new(None);

    let transformed = bezierpath.map_coords(|coord| match transform(coord) {
        Ok(transformed) => transformed,
        Err(error) => {
            failure.set(Some(error));
            coord
        }
    });

    match failure.into_inner() {
        Some(error) => Err(error),
        None => Ok(transformed),
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
