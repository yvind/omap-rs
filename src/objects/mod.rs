mod area_object;
mod line_object;
mod point_object;
mod text_object;

mod map_object;

use geo_types::{Coord, LineString};
pub use linestring2bezier::{BezierSegment, BezierString};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};
use std::collections::HashMap;

pub use area_object::{AreaObject, BezierPolygon, PatternRotation};
pub use line_object::LineObject;
pub use point_object::PointObject;
pub use text_object::{HorizontalAlign, TextGeometry, TextObject, VerticalAlign, WrapBox};

pub use map_object::MapObject;

use crate::{
    notes,
    utils::{from_file_coords, to_file_coords, try_get_attr},
};

use super::{Error, OmapSection, Result};

type FileCoord = (Coord<i32>, u8);

/// A coordinate starts a cubic Bézier segment.
pub const COORD_FLAG_CURVE_START: u8 = 1;
/// A coordinate closes the current path.
pub const COORD_FLAG_CLOSE_POINT: u8 = 2;
/// A coordinate is the endpoint of a line-symbol gap.
pub const COORD_FLAG_GAP_POINT: u8 = 4;
/// A coordinate closes an interior polygon ring.
pub const COORD_FLAG_HOLE_POINT: u8 = 16;
/// A coordinate is a forced dash point.
pub const COORD_FLAG_DASH_POINT: u8 = 32;

const COORD_FLAGS_RING_END: u8 = COORD_FLAG_CLOSE_POINT | COORD_FLAG_HOLE_POINT;

/// A mixed straight/cubic Bézier path with dash-point metadata on its
/// vertices.
///
/// `vertex_is_dash_point` contains one entry for the initial vertex followed
/// by one entry for every segment end, so its length is always
/// `geometry.num_segments() + 1`. For a closed path, its first and final
/// entries describe the same seam vertex and therefore have the same value.
///
/// Paths come from parsed objects, but [`Self::new`] builds one directly and
/// enforces both invariants, so a hand-built path is as trustworthy as a
/// parsed one.
#[derive(Debug, Clone)]
pub struct BezierPath {
    /// The straight and cubic segments forming the path.
    geometry: BezierString,
    /// Whether each path vertex carries [`COORD_FLAG_DASH_POINT`].
    vertex_is_dash_point: Vec<bool>,
}

impl BezierPath {
    /// Build a path from its geometry and per-vertex dash flags.
    ///
    /// Returns [`None`] unless the arguments uphold the type's invariants:
    /// `geometry` must hold at least one segment, `vertex_is_dash_point` must
    /// hold one entry per path vertex, and a closed path's first and final
    /// entries, which describe the same seam vertex, must agree.
    pub fn new(geometry: BezierString, vertex_is_dash_point: Vec<bool>) -> Option<Self> {
        let segments = geometry.num_segments();
        if segments == 0 {
            return None;
        }

        let expected = segments + 1;
        if vertex_is_dash_point.len() != expected {
            return None;
        }

        let first_segment = geometry.segments().next()?;
        let last_segment = geometry.segments().last()?;
        if first_segment.start() == last_segment.end()
            && vertex_is_dash_point.first() != vertex_is_dash_point.last()
        {
            return None;
        }

        Some(Self {
            geometry,
            vertex_is_dash_point,
        })
    }

    /// Get the mixed straight/cubic geometry.
    pub fn geometry(&self) -> &BezierString {
        &self.geometry
    }

    /// Get the dash-point state of the initial vertex and every segment end.
    pub fn vertex_is_dash_point(&self) -> &[bool] {
        &self.vertex_is_dash_point
    }

    /// Transform every endpoint and Bézier handle while preserving path
    /// topology and dash-point metadata.
    ///
    /// `transform` should map equal coordinates to equal coordinates, as
    /// coordinate reprojection functions normally do.
    pub fn map_coords(mut self, transform: impl Fn(Coord) -> Coord) -> Self {
        for segment in self.geometry.segments_mut() {
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
        self
    }

    /// Consume the path and return its geometry and vertex dash flags.
    pub fn into_parts(self) -> (BezierString, Vec<bool>) {
        (self.geometry, self.vertex_is_dash_point)
    }

    /// Iterate over segments paired with the dash-point state of their end
    /// vertices.
    ///
    /// The initial vertex's state is available as
    /// `vertex_is_dash_point().first()`.
    pub fn segments(&self) -> impl ExactSizeIterator<Item = (&BezierSegment, bool)> {
        self.geometry
            .segments()
            .zip(self.vertex_is_dash_point.iter().copied().skip(1))
    }

    /// Approximate the path by a polyline whose vertices keep their
    /// dash-point state.
    ///
    /// The polyline is the one [`BezierString::to_line_string`] produces for
    /// the same tolerance, so it matches the flattened geometry the object
    /// accessors hand out. Every path vertex survives flattening exactly and
    /// keeps its dash-point state; the points introduced when subdividing a
    /// curve fall between vertices and are never dash points.
    ///
    /// # Errors
    ///
    /// Returns an error if the geometry cannot be flattened with the requested
    /// error tolerance.
    pub fn flatten(&self, allowed_error: f64) -> Result<FlattenedPath> {
        let (line_string, segment_end_indices) = self
            .geometry
            .to_line_string_with_segment_ends(allowed_error)?;

        // The initial vertex lands at index 0 and every later vertex ends a
        // segment, so this walks the two vectors in step. Segment ends are
        // ascending and none of them is 0, so the kept indices are too.
        let dash_point_indices = std::iter::once(0)
            .chain(segment_end_indices)
            .zip(self.vertex_is_dash_point.iter().copied())
            .filter_map(|(index, is_dash_point)| is_dash_point.then_some(index))
            .collect();

        Ok(FlattenedPath {
            line_string,
            dash_point_indices,
        })
    }
}

/// A flattened path and the vertices along it that are dash points.
///
/// Produced by [`BezierPath::flatten`], which is the only thing that can build
/// one. A path normally has a handful of dash points among a great many
/// flattened vertices, so they are held as ascending indices into
/// [`Self::line_string`] rather than as a flag per vertex.
#[derive(Debug, Clone)]
pub struct FlattenedPath {
    /// The polyline approximating the Bézier path.
    line_string: LineString,
    /// Where the dash points sit in `line_string`, ascending.
    dash_point_indices: Vec<usize>,
}

impl FlattenedPath {
    /// Get the flattened geometry.
    pub fn line_string(&self) -> &LineString {
        &self.line_string
    }

    /// Get the positions of the dash points in [`Self::line_string`], in
    /// ascending order.
    pub fn dash_point_indices(&self) -> &[usize] {
        &self.dash_point_indices
    }

    /// Iterate over every vertex paired with its dash-point state.
    ///
    /// Walks the sparse indices alongside the coordinates, so reading the
    /// whole path this way costs no more than the dense form would.
    pub fn vertices(&self) -> impl ExactSizeIterator<Item = (Coord, bool)> {
        let mut remaining = self.dash_point_indices.as_slice();

        self.line_string
            .0
            .iter()
            .copied()
            .enumerate()
            .map(move |(index, coord)| {
                if let Some((&next_dash_point, rest)) = remaining.split_first()
                    && next_dash_point == index
                {
                    remaining = rest;
                    (coord, true)
                } else {
                    (coord, false)
                }
            })
    }

    /// Consume the flattened path and return its geometry and dash-point
    /// positions.
    pub fn into_parts(self) -> (LineString, Vec<usize>) {
        (self.line_string, self.dash_point_indices)
    }
}

/// Build the exact mixed line/Bézier representation encoded by Mapper's raw
/// coordinate flags.
///
/// A coordinate with bit 0 set starts a cubic Bézier whose two handles and end
/// point are the following three coordinates. An end point may also start the
/// next curve, so it is deliberately visited again in that case.
fn bezier_from_raw_coords(coords: &[FileCoord]) -> Option<BezierPath> {
    let mut segments = Vec::new();
    let mut vertex_is_dash_point = coords
        .first()
        .map(|(_, flag)| flag & COORD_FLAG_DASH_POINT != 0)
        .into_iter()
        .collect::<Vec<_>>();
    let mut previous_anchor = None;
    let mut index = 0;

    while index < coords.len() {
        let (file_coord, flag) = coords[index];
        let coord = from_file_coords(file_coord);

        if let Some((previous_index, previous_coord)) = previous_anchor
            && previous_index != index
        {
            segments.push(BezierSegment::new(previous_coord, None, coord));
            vertex_is_dash_point.push(flag & COORD_FLAG_DASH_POINT != 0);
        }

        if flag & COORD_FLAG_CURVE_START != 0 && index + 3 < coords.len() {
            let handle1 = from_file_coords(coords[index + 1].0);
            let handle2 = from_file_coords(coords[index + 2].0);
            let end_index = index + 3;
            let end = from_file_coords(coords[end_index].0);
            segments.push(BezierSegment::new(coord, Some((handle1, handle2)), end));
            vertex_is_dash_point.push(coords[end_index].1 & COORD_FLAG_DASH_POINT != 0);
            previous_anchor = Some((end_index, end));

            if coords[end_index].1 & COORD_FLAG_CURVE_START != 0 {
                index = end_index;
            } else {
                index = end_index + 1;
            }
        } else {
            previous_anchor = Some((index, coord));
            index += 1;
        }
    }

    // A closed path repeats its first vertex as its final endpoint. Treat the
    // dash flag on either raw representation as metadata for that shared
    // vertex and expose the folded value at both ends of the vertex vector.
    if let (Some((first_coord, first_flag)), Some((last_coord, last_flag))) =
        (coords.first(), coords.last())
        && last_flag & COORD_FLAGS_RING_END != 0
        && first_coord == last_coord
        && (first_flag & COORD_FLAG_DASH_POINT != 0 || last_flag & COORD_FLAG_DASH_POINT != 0)
    {
        if let Some(first_is_dash_point) = vertex_is_dash_point.first_mut() {
            *first_is_dash_point = true;
        }
        if let Some(last_is_dash_point) = vertex_is_dash_point.last_mut() {
            *last_is_dash_point = true;
        }
    }

    BezierPath::new(BezierString::new(segments), vertex_is_dash_point)
}

fn file_coords_from_bezier(
    geometry: &BezierString,
    final_vertex_flags: u8,
) -> Result<Vec<FileCoord>> {
    let final_vertex = geometry
        .segments()
        .last()
        .map(BezierSegment::end)
        .ok_or(Error::ObjectError)?;

    let mut coords = Vec::with_capacity(geometry.num_points());
    for segment in geometry.segments() {
        match segment {
            BezierSegment::Bezier(curve) => {
                coords.push((to_file_coords(curve.start)?, COORD_FLAG_CURVE_START));
                coords.push((to_file_coords(curve.handle1)?, 0));
                coords.push((to_file_coords(curve.handle2)?, 0));
            }
            BezierSegment::Line(line) => {
                coords.push((to_file_coords(line.start)?, 0));
            }
        }
    }
    coords.push((to_file_coords(final_vertex)?, final_vertex_flags));
    Ok(coords)
}

fn parse_tags<R: std::io::BufRead>(reader: &mut Reader<R>) -> Result<HashMap<String, String>> {
    let mut buf = Vec::new();

    let mut tags = HashMap::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(bytes_start) => {
                if matches!(bytes_start.local_name().as_ref(), b"t") {
                    let key = try_get_attr(&bytes_start, "k")?.unwrap_or(String::new());
                    let value = notes::parse(reader)?;
                    if !key.is_empty() && !value.is_empty() {
                        let _ = tags.insert(key, value);
                    }
                }
            }
            Event::End(bytes_end) if bytes_end.local_name().as_ref() == b"tags" => {
                break;
            }
            Event::Eof => {
                return Err(Error::UnexpectedEof(OmapSection::Tags));
            }
            _ => (),
        }
    }
    Ok(tags)
}

fn write_tags<W: std::io::Write>(
    writer: &mut Writer<W>,
    tags: &HashMap<String, String>,
) -> Result<()> {
    writer.write_event(Event::Start(BytesStart::new("tags")))?;
    for (key, value) in tags {
        writer.write_event(Event::Start(
            BytesStart::new("t").with_attributes([("k", key.as_str())]),
        ))?;
        writer.write_event(Event::Text(BytesText::new(value)))?;
        writer.write_event(Event::End(BytesEnd::new("t")))?;
    }
    writer.write_event(Event::End(BytesEnd::new("tags")))?;
    Ok(())
}

/// Write raw map coords as the content of a `<coords>` element
fn write_raw_coords<W: std::io::Write>(writer: &mut Writer<W>, coords: &[FileCoord]) -> Result<()> {
    let bs =
        BytesStart::new("coords").with_attributes([("count", coords.len().to_string().as_str())]);
    writer.write_event(Event::Start(bs))?;
    let mut content = String::new();
    for (coord, flag) in coords {
        content.push_str(&coord.x.to_string());
        content.push(' ');
        content.push_str(&coord.y.to_string());
        if *flag != 0 {
            content.push(' ');
            content.push_str(&flag.to_string());
        }
        content.push(';');
    }
    writer.write_event(Event::Text(BytesText::new(&content)))?;
    writer.write_event(Event::End(BytesEnd::new("coords")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use geo_types::Coord;
    use linestring2bezier::{BezierCurve, BezierSegment, BezierString};

    use super::{
        BezierPath, COORD_FLAG_CLOSE_POINT, COORD_FLAG_CURVE_START, COORD_FLAG_DASH_POINT,
        COORD_FLAGS_RING_END, bezier_from_raw_coords, file_coords_from_bezier,
    };

    /// Three segments — line, curve, line — whose first three vertices are
    /// dash points. In map coordinates the vertices are (0, 0), (1, 0),
    /// (2, 0) and (3, 0), with the curve bulging away from the y = 0 axis.
    fn line_curve_line() -> Option<BezierPath> {
        bezier_from_raw_coords(&[
            (Coord { x: 0, y: 0 }, COORD_FLAG_DASH_POINT),
            (
                Coord { x: 1_000, y: 0 },
                COORD_FLAG_CURVE_START | COORD_FLAG_DASH_POINT,
            ),
            (Coord { x: 1_000, y: 1_000 }, 0),
            (Coord { x: 2_000, y: 1_000 }, 0),
            (Coord { x: 2_000, y: 0 }, COORD_FLAG_DASH_POINT),
            (Coord { x: 3_000, y: 0 }, 0),
        ])
    }

    #[test]
    fn dash_flags_align_with_segment_end_vertices() {
        let path = line_curve_line();
        assert!(path.is_some());
        let Some(path) = path else {
            return;
        };

        assert_eq!(path.geometry().num_segments(), 3);
        assert_eq!(path.vertex_is_dash_point(), [true, true, true, false]);
        assert!(matches!(path.geometry().0[0], BezierSegment::Line(_)));
        assert!(matches!(path.geometry().0[1], BezierSegment::Bezier(_)));
        assert!(matches!(path.geometry().0[2], BezierSegment::Line(_)));
        assert_eq!(
            path.segments()
                .map(|(_, end_is_dash_point)| end_is_dash_point)
                .collect::<Vec<_>>(),
            [true, true, false]
        );
    }

    #[test]
    fn closed_path_combines_first_and_closing_vertex_dash_flags() {
        let path = bezier_from_raw_coords(&[
            (Coord { x: 0, y: 0 }, 32),
            (Coord { x: 1_000, y: 0 }, 0),
            (Coord { x: 0, y: 0 }, 2),
        ]);
        assert!(path.is_some());
        let Some(path) = path else {
            return;
        };

        assert_eq!(path.geometry().num_segments(), 2);
        assert_eq!(path.vertex_is_dash_point(), [true, false, true]);
    }

    #[test]
    fn open_path_preserves_both_endpoint_dash_flags() {
        let path = bezier_from_raw_coords(&[
            (Coord { x: 0, y: 0 }, COORD_FLAG_DASH_POINT),
            (Coord { x: 1_000, y: 0 }, COORD_FLAG_DASH_POINT),
            (Coord { x: 2_000, y: 0 }, COORD_FLAG_DASH_POINT),
            (Coord { x: 3_000, y: 0 }, COORD_FLAG_DASH_POINT),
        ]);
        assert!(path.is_some());
        let Some(path) = path else {
            return;
        };

        assert_eq!(path.geometry().num_segments(), 3);
        assert_eq!(path.vertex_is_dash_point(), [true, true, true, true]);
    }

    #[test]
    fn bezier_path_construction_enforces_invariants() {
        assert!(BezierPath::new(BezierString::empty(), Vec::new()).is_none());

        let open_geometry = BezierString::new(vec![BezierSegment::new(
            Coord { x: 0.0, y: 0.0 },
            None,
            Coord { x: 1.0, y: 0.0 },
        )]);
        assert!(BezierPath::new(open_geometry.clone(), vec![false]).is_none());
        assert!(BezierPath::new(open_geometry, vec![false, true]).is_some());

        let closed_geometry = BezierString::new(vec![
            BezierSegment::new(Coord { x: 0.0, y: 0.0 }, None, Coord { x: 1.0, y: 0.0 }),
            BezierSegment::new(Coord { x: 1.0, y: 0.0 }, None, Coord { x: 0.0, y: 0.0 }),
        ]);
        assert!(BezierPath::new(closed_geometry.clone(), vec![false, false, true]).is_none());
        assert!(BezierPath::new(closed_geometry, vec![true, false, true]).is_some());
    }

    #[test]
    fn mapping_bezier_path_coords_preserves_structure_and_flags() {
        let geometry = BezierString::new(vec![BezierSegment::Bezier(BezierCurve::new(
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 0.0, y: 1.0 },
            Coord { x: 1.0, y: 1.0 },
            Coord { x: 1.0, y: 0.0 },
        ))]);
        let path = BezierPath::new(geometry, vec![true, false]);
        assert!(path.is_some());
        let Some(path) = path else {
            return;
        };

        let mapped = path.map_coords(|coord| Coord {
            x: coord.x + 2.0,
            y: coord.y - 3.0,
        });

        assert_eq!(mapped.vertex_is_dash_point(), [true, false]);
        let Some(BezierSegment::Bezier(curve)) = mapped.geometry().segments().next() else {
            return;
        };
        assert_eq!(curve.start, Coord { x: 2.0, y: -3.0 });
        assert_eq!(curve.handle1, Coord { x: 2.0, y: -2.0 });
        assert_eq!(curve.handle2, Coord { x: 3.0, y: -2.0 });
        assert_eq!(curve.end, Coord { x: 3.0, y: -3.0 });
    }

    #[test]
    fn bezier_serialization_uses_exported_coordinate_flags() {
        let geometry = BezierString::new(vec![
            BezierSegment::Line(geo_types::Line::new(
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 1.0, y: 0.0 },
            )),
            BezierSegment::Bezier(BezierCurve::new(
                Coord { x: 1.0, y: 0.0 },
                Coord { x: 1.0, y: 1.0 },
                Coord { x: 2.0, y: 1.0 },
                Coord { x: 2.0, y: 0.0 },
            )),
        ]);

        let coords = file_coords_from_bezier(&geometry, COORD_FLAGS_RING_END);
        assert!(coords.is_ok());
        let Ok(coords) = coords else {
            return;
        };
        assert_eq!(
            coords,
            [
                (Coord { x: 0, y: 0 }, 0),
                (Coord { x: 1_000, y: 0 }, COORD_FLAG_CURVE_START),
                (
                    Coord {
                        x: 1_000,
                        y: -1_000
                    },
                    0
                ),
                (
                    Coord {
                        x: 2_000,
                        y: -1_000
                    },
                    0
                ),
                (Coord { x: 2_000, y: 0 }, COORD_FLAGS_RING_END),
            ]
        );
    }

    #[test]
    fn flattening_keeps_dash_points_on_their_own_vertices() {
        let path = line_curve_line();
        assert!(path.is_some());
        let Some(path) = path else {
            return;
        };

        let flattened = path.flatten(0.01);
        assert!(flattened.is_ok());
        let Ok(flattened) = flattened else {
            return;
        };

        // The curve was subdivided, so the polyline has vertices the path did
        // not, and none of them may be a dash point.
        let vertex_count = flattened.line_string().0.len();
        assert!(vertex_count > path.geometry().num_segments() + 1);
        assert_eq!(flattened.dash_point_indices().len(), 3);
        assert!(flattened.dash_point_indices().is_sorted());
        assert!(
            flattened
                .dash_point_indices()
                .iter()
                .all(|index| *index < vertex_count)
        );

        // The dash points are exactly the three flagged path vertices, which
        // also shows subdivision reproduced them without drift.
        assert_eq!(
            flattened
                .dash_point_indices()
                .iter()
                .filter_map(|index| flattened.line_string().0.get(*index))
                .collect::<Vec<_>>(),
            [
                &Coord { x: 0., y: 0. },
                &Coord { x: 1., y: 0. },
                &Coord { x: 2., y: 0. },
            ]
        );
    }

    #[test]
    fn iterating_vertices_reproduces_the_sparse_dash_points() {
        let path = line_curve_line();
        assert!(path.is_some());
        let Some(path) = path else {
            return;
        };

        let flattened = path.flatten(0.01);
        assert!(flattened.is_ok());
        let Ok(flattened) = flattened else {
            return;
        };

        assert_eq!(flattened.vertices().len(), flattened.line_string().0.len());
        assert_eq!(
            flattened
                .vertices()
                .enumerate()
                .filter_map(|(index, (_, is_dash_point))| is_dash_point.then_some(index))
                .collect::<Vec<_>>(),
            flattened.dash_point_indices()
        );
        assert_eq!(
            flattened
                .vertices()
                .map(|(coord, _)| coord)
                .collect::<Vec<_>>(),
            flattened.line_string().0
        );
    }

    #[test]
    fn flattening_agrees_with_the_plain_line_string_conversion() {
        let path = line_curve_line();
        assert!(path.is_some());
        let Some(path) = path else {
            return;
        };

        let flattened = path.flatten(0.01);
        let plain = path.geometry().to_line_string(0.01);
        assert!(flattened.is_ok() && plain.is_ok());
        let (Ok(flattened), Ok(plain)) = (flattened, plain) else {
            return;
        };

        assert_eq!(flattened.line_string(), &plain);
    }

    #[test]
    fn flattening_a_closed_path_keeps_both_ends_of_the_seam() {
        let path = bezier_from_raw_coords(&[
            (Coord { x: 0, y: 0 }, COORD_FLAG_DASH_POINT),
            (Coord { x: 1_000, y: 0 }, 0),
            (Coord { x: 0, y: 0 }, COORD_FLAG_CLOSE_POINT),
        ]);
        assert!(path.is_some());
        let Some(path) = path else {
            return;
        };

        let flattened = path.flatten(0.01);
        assert!(flattened.is_ok());
        let Ok(flattened) = flattened else {
            return;
        };

        assert_eq!(flattened.line_string().0.len(), 3);
        assert_eq!(flattened.dash_point_indices(), [0, 2]);
        assert_eq!(
            flattened.line_string().0.first(),
            flattened.line_string().0.last()
        );
    }

    #[test]
    fn flattening_propagates_an_unusable_tolerance() {
        let path = line_curve_line();
        assert!(path.is_some());
        let Some(path) = path else {
            return;
        };

        assert!(path.flatten(0.).is_err());
    }
}
