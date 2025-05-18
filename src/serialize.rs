use geo_types::{Coord, LineString, Polygon};
use linestring2bezier::{BezierSegment, BezierString};

use crate::{OmapError, OmapResult, Scale};

pub(crate) trait MapCoord {
    fn to_map_coordinates(
        self,
        scale: Scale,
        grivation: f64,
        inv_combined_scale_factor: f64,
    ) -> OmapResult<(i32, i32)>;
}

// 1 map unit is 0.001mm on paper => 1000 mu = 1mm on map = 15m on ground
const CONVERSION_15000: f64 = 1_000. / 15.;
const CONVERSION_10000: f64 = 1_000. / 10.;

const MAX_MU: f64 = i32::MAX as f64;

impl MapCoord for Coord {
    fn to_map_coordinates(
        self,
        scale: Scale,
        grivation: f64,
        inv_combined_scale_factor: f64,
    ) -> OmapResult<(i32, i32)> {
        let sin = grivation.sin();
        let cos = grivation.cos();

        let x = self.x * cos - self.y * sin;
        let y = self.x * sin + self.y * cos;

        let (x, y) = match scale {
            Scale::S10_000 => (
                (x * CONVERSION_10000 * inv_combined_scale_factor).round(),
                -(y * CONVERSION_10000 * inv_combined_scale_factor).round(),
            ),
            Scale::S15_000 => (
                (x * CONVERSION_15000 * inv_combined_scale_factor).round(),
                -(y * CONVERSION_15000 * inv_combined_scale_factor).round(),
            ),
        };

        if (x.abs() > MAX_MU) || (y.abs() > MAX_MU) {
            Err(OmapError::MapCoordinateOverflow)
        } else {
            Ok((x as i32, y as i32))
        }
    }
}

pub(crate) trait Serialize {
    fn serialize_polyline(
        self,
        scale: Scale,
        grivation: f64,
        inv_combined_scale_factor: f64,
    ) -> OmapResult<(Vec<u8>, usize)>;

    fn serialize_bezier(
        self,
        bezier_error: f64,
        scale: Scale,
        grivation: f64,
        inv_combined_scale_factor: f64,
    ) -> OmapResult<(Vec<u8>, usize)>;
}

impl Serialize for LineString {
    fn serialize_polyline(
        self,
        scale: Scale,
        grivation: f64,
        inv_combined_scale_factor: f64,
    ) -> OmapResult<(Vec<u8>, usize)> {
        let num_coords = self.0.len();

        let mut byte_vec = Vec::with_capacity(num_coords * 10);

        let mut coord_iter = self.coords();
        let mut i = 0;
        while i < num_coords - 1 {
            let c = coord_iter.next().unwrap().to_map_coordinates(
                scale,
                grivation,
                inv_combined_scale_factor,
            )?;
            byte_vec.extend(format!("{} {};", c.0, c.1).into_bytes());

            i += 1;
        }
        let c = coord_iter.next().unwrap().to_map_coordinates(
            scale,
            grivation,
            inv_combined_scale_factor,
        )?;
        if self.is_closed() {
            byte_vec.extend(format!("{} {} 18;", c.0, c.1).into_bytes());
        } else {
            byte_vec.extend(format!("{} {};", c.0, c.1).into_bytes());
        }
        Ok((byte_vec, num_coords))
    }

    fn serialize_bezier(
        self,
        bezier_error: f64,
        scale: Scale,
        grivation: f64,
        inv_combined_scale_factor: f64,
    ) -> OmapResult<(Vec<u8>, usize)> {
        let is_closed = self.is_closed();
        let bezier = BezierString::from_linestring(self, bezier_error);

        let num_coords = bezier.num_points();
        let num_segments = bezier.0.len();

        let mut byte_vec = Vec::with_capacity(num_coords * 12);

        let mut bez_iterator = bezier.0.into_iter();
        let mut i = 0;
        while i < num_segments - 1 {
            let segment = bez_iterator.next().unwrap();

            let BezierSegment {
                start,
                handles,
                end: _,
            } = segment;

            if let Some(handles) = handles {
                let c = start.to_map_coordinates(scale, grivation, inv_combined_scale_factor)?;
                let h1 =
                    handles
                        .0
                        .to_map_coordinates(scale, grivation, inv_combined_scale_factor)?;
                let h2 =
                    handles
                        .1
                        .to_map_coordinates(scale, grivation, inv_combined_scale_factor)?;
                byte_vec.extend(
                    format!("{} {} 1;{} {};{} {};", c.0, c.1, h1.0, h1.1, h2.0, h2.1).into_bytes(),
                );
            } else {
                let c = start.to_map_coordinates(scale, grivation, inv_combined_scale_factor)?;

                byte_vec.extend(format!("{} {};", c.0, c.1).into_bytes());
            }
            i += 1;
        }
        // finish with the last segment of the curve
        let final_segment = bez_iterator.next().unwrap();

        let BezierSegment {
            start,
            handles,
            end,
        } = final_segment;

        if let Some(handles) = handles {
            let c1 = start.to_map_coordinates(scale, grivation, inv_combined_scale_factor)?;
            let h1 = handles
                .0
                .to_map_coordinates(scale, grivation, inv_combined_scale_factor)?;
            let h2 = handles
                .1
                .to_map_coordinates(scale, grivation, inv_combined_scale_factor)?;
            let c2 = end.to_map_coordinates(scale, grivation, inv_combined_scale_factor)?;

            if is_closed {
                byte_vec.extend(
                    format!(
                        "{} {} 1;{} {};{} {};{} {} 18;",
                        c1.0, c1.1, h1.0, h1.1, h2.0, h2.1, c2.0, c2.1
                    )
                    .into_bytes(),
                );
            } else {
                byte_vec.extend(
                    format!(
                        "{} {} 1;{} {};{} {};{} {};",
                        c1.0, c1.1, h1.0, h1.1, h2.0, h2.1, c2.0, c2.1
                    )
                    .into_bytes(),
                );
            }
        } else {
            let c1 = start.to_map_coordinates(scale, grivation, inv_combined_scale_factor)?;
            let c2 = end.to_map_coordinates(scale, grivation, inv_combined_scale_factor)?;

            if is_closed {
                byte_vec.extend(format!("{} {};{} {} 18;", c1.0, c1.1, c2.0, c2.1).into_bytes());
            } else {
                byte_vec.extend(format!("{} {};{} {};", c1.0, c1.1, c2.0, c2.1).into_bytes());
            }
        }
        Ok((byte_vec, num_coords))
    }
}

impl Serialize for Polygon {
    fn serialize_polyline(
        self,
        scale: Scale,
        grivation: f64,
        inv_combined_scale_factor: f64,
    ) -> OmapResult<(Vec<u8>, usize)> {
        let (exterior, interiors) = self.into_inner();

        let (mut bytes_vec, mut num_coords) =
            exterior.serialize_polyline(scale, grivation, inv_combined_scale_factor)?;

        for hole in interiors {
            let (hv, hc) = hole.serialize_polyline(scale, grivation, inv_combined_scale_factor)?;
            bytes_vec.extend(hv);
            num_coords += hc;
        }
        Ok((bytes_vec, num_coords))
    }

    fn serialize_bezier(
        self,
        bezier_error: f64,
        scale: Scale,
        grivation: f64,
        inv_combined_scale_factor: f64,
    ) -> OmapResult<(Vec<u8>, usize)> {
        let (exterior, interiors) = self.into_inner();

        let (mut bytes_vec, mut num_coords) =
            exterior.serialize_bezier(bezier_error, scale, grivation, inv_combined_scale_factor)?;

        for hole in interiors {
            let (hv, hc) =
                hole.serialize_bezier(bezier_error, scale, grivation, inv_combined_scale_factor)?;
            bytes_vec.extend(hv);
            num_coords += hc;
        }
        Ok((bytes_vec, num_coords))
    }
}
