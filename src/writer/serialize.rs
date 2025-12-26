use geo_types::{Coord, LineString, Point, Polygon};
use linestring2bezier::{BezierSegment, BezierString};

use crate::writer::{Error, Result, transform::Transform};

pub(crate) trait MapCoord {
    fn to_map_coordinates(self, transform: &Transform) -> Result<(i32, i32)>;
}

const MAX_MAP_UNIT: f64 = i32::MAX as f64;

impl MapCoord for Coord {
    fn to_map_coordinates(self, transform: &Transform) -> Result<(i32, i32)> {
        let coord = transform.world_to_map(self);

        if (coord.x.abs() > MAX_MAP_UNIT) || (coord.y.abs() > MAX_MAP_UNIT) {
            Err(Error::MapCoordinateOverflow)
        } else {
            Ok((coord.x as i32, coord.y as i32))
        }
    }
}

pub(crate) trait SerializePolyLine {
    fn serialize_polyline(self, transform: &Transform) -> Result<(Vec<u8>, usize)>;
}

pub(crate) trait SerializeBezier {
    fn serialize_bezier(self, bezier_error: f64, transform: &Transform)
    -> Result<(Vec<u8>, usize)>;
}

impl SerializePolyLine for LineString {
    fn serialize_polyline(self, transform: &Transform) -> Result<(Vec<u8>, usize)> {
        let num_coords = self.0.len();

        let mut byte_vec = Vec::with_capacity(num_coords * 10);

        let mut coord_iter = self.coords();
        let mut i = 0;
        while i < num_coords - 1 {
            let c = coord_iter.next().unwrap().to_map_coordinates(transform)?;
            byte_vec.extend(format!("{} {};", c.0, c.1).into_bytes());

            i += 1;
        }
        let c = coord_iter.next().unwrap().to_map_coordinates(transform)?;
        if self.is_closed() {
            byte_vec.extend(format!("{} {} 18;", c.0, c.1).into_bytes());
        } else {
            byte_vec.extend(format!("{} {};", c.0, c.1).into_bytes());
        }
        Ok((byte_vec, num_coords))
    }
}

impl SerializeBezier for LineString {
    fn serialize_bezier(
        self,
        bezier_error: f64,
        transform: &Transform,
    ) -> Result<(Vec<u8>, usize)> {
        let is_closed = self.is_closed();
        let bezier = BezierString::from_line_string(self, bezier_error)?;

        let num_coords = bezier.num_points();
        let mut byte_vec = Vec::with_capacity(num_coords * 12);

        let num_segments = bezier.num_segments();
        let mut bez_iterator = bezier.into_inner().into_iter();

        let mut i = 0;
        while i < num_segments - 1 {
            match bez_iterator.next().unwrap() {
                BezierSegment::Bezier(bc) => {
                    let c = bc.start.to_map_coordinates(transform)?;
                    let h1 = bc.handle1.to_map_coordinates(transform)?;
                    let h2 = bc.handle2.to_map_coordinates(transform)?;
                    byte_vec.extend(
                        format!("{} {} 1;{} {};{} {};", c.0, c.1, h1.0, h1.1, h2.0, h2.1)
                            .into_bytes(),
                    );
                }
                BezierSegment::Line(line) => {
                    let c = line.start.to_map_coordinates(transform)?;
                    byte_vec.extend(format!("{} {};", c.0, c.1).into_bytes());
                }
            }
            i += 1;
        }
        // finish with the last segment of the curve
        match bez_iterator.next().unwrap() {
            BezierSegment::Bezier(bc) => {
                let c1 = bc.start.to_map_coordinates(transform)?;
                let h1 = bc.handle1.to_map_coordinates(transform)?;
                let h2 = bc.handle2.to_map_coordinates(transform)?;
                let c2 = bc.end.to_map_coordinates(transform)?;
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
            }
            BezierSegment::Line(line) => {
                let c1 = line.start.to_map_coordinates(transform)?;
                let c2 = line.end.to_map_coordinates(transform)?;

                if is_closed {
                    byte_vec
                        .extend(format!("{} {};{} {} 18;", c1.0, c1.1, c2.0, c2.1).into_bytes());
                } else {
                    byte_vec.extend(format!("{} {};{} {};", c1.0, c1.1, c2.0, c2.1).into_bytes());
                }
            }
        }

        Ok((byte_vec, num_coords))
    }
}

impl SerializePolyLine for Polygon {
    fn serialize_polyline(self, transform: &Transform) -> Result<(Vec<u8>, usize)> {
        let (exterior, interiors) = self.into_inner();

        let (mut bytes_vec, mut num_coords) = exterior.serialize_polyline(transform)?;

        for hole in interiors {
            let (hv, hc) = hole.serialize_polyline(transform)?;
            bytes_vec.extend(hv);
            num_coords += hc;
        }
        Ok((bytes_vec, num_coords))
    }
}

impl SerializeBezier for Polygon {
    fn serialize_bezier(
        self,
        bezier_error: f64,
        transform: &Transform,
    ) -> Result<(Vec<u8>, usize)> {
        let (exterior, interiors) = self.into_inner();

        let (mut bytes_vec, mut num_coords) = exterior.serialize_bezier(bezier_error, transform)?;

        for hole in interiors {
            let (hv, hc) = hole.serialize_bezier(bezier_error, transform)?;
            bytes_vec.extend(hv);
            num_coords += hc;
        }
        Ok((bytes_vec, num_coords))
    }
}

impl SerializePolyLine for Point {
    fn serialize_polyline(self, transform: &Transform) -> Result<(Vec<u8>, usize)> {
        let c = self.0.to_map_coordinates(transform)?;
        let bytes = format!("{} {};", c.0, c.1).into_bytes();

        Ok((bytes, 1))
    }
}
