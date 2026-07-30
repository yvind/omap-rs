mod area_object;
mod line_object;
mod point_object;
mod text_object;

mod map_object;

use geo_types::Coord;
use linestring2bezier::{BezierSegment, BezierString};
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
/// `geometry.num_segments() + 1`, except for an empty path, which has
/// neither vertices nor segments. For a closed path, its first and final
/// entries describe the same seam vertex and therefore have the same value.
#[derive(Debug, Clone)]
pub struct BezierPath {
    /// The straight and cubic segments forming the path.
    pub geometry: BezierString,
    /// Whether each path vertex carries [`COORD_FLAG_DASH_POINT`].
    pub vertex_is_dash_point: Vec<bool>,
}

impl BezierPath {
    /// Iterate over segments paired with the dash-point state of their end
    /// vertices.
    ///
    /// The initial vertex's state is available as
    /// `vertex_is_dash_point.first()`.
    pub fn segments(&self) -> impl ExactSizeIterator<Item = (&BezierSegment, bool)> {
        debug_assert!(
            (self.geometry.num_segments() == 0 && self.vertex_is_dash_point.is_empty())
                || self.geometry.num_segments() + 1 == self.vertex_is_dash_point.len(),
            "a non-empty Bézier path must have one more vertex flag than segments"
        );
        self.geometry
            .segments()
            .zip(self.vertex_is_dash_point.iter().copied().skip(1))
    }
}

/// Build the exact mixed line/Bézier representation encoded by Mapper's raw
/// coordinate flags.
///
/// A coordinate with bit 0 set starts a cubic Bézier whose two handles and end
/// point are the following three coordinates. An end point may also start the
/// next curve, so it is deliberately visited again in that case.
fn bezier_from_raw_coords(coords: &[FileCoord]) -> BezierPath {
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

    debug_assert_eq!(
        segments.len() + usize::from(!coords.is_empty()),
        vertex_is_dash_point.len(),
        "every non-empty Bézier path must have one more vertex flag than segments"
    );
    BezierPath {
        geometry: BezierString::new(segments),
        vertex_is_dash_point,
    }
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
            Event::End(bytes_end) => {
                if bytes_end.local_name().as_ref() == b"tags" {
                    break;
                }
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
        COORD_FLAG_CURVE_START, COORD_FLAG_DASH_POINT, COORD_FLAGS_RING_END,
        bezier_from_raw_coords, file_coords_from_bezier,
    };

    #[test]
    fn dash_flags_align_with_segment_end_vertices() {
        let path = bezier_from_raw_coords(&[
            (Coord { x: 0, y: 0 }, COORD_FLAG_DASH_POINT),
            (Coord { x: 1_000, y: 0 }, 33),
            (Coord { x: 1_000, y: 1_000 }, 0),
            (Coord { x: 2_000, y: 1_000 }, 0),
            (Coord { x: 2_000, y: 0 }, 32),
            (Coord { x: 3_000, y: 0 }, 0),
        ]);

        assert_eq!(path.geometry.num_segments(), 3);
        assert_eq!(path.vertex_is_dash_point, [true, true, true, false]);
        assert!(matches!(path.geometry.0[0], BezierSegment::Line(_)));
        assert!(matches!(path.geometry.0[1], BezierSegment::Bezier(_)));
        assert!(matches!(path.geometry.0[2], BezierSegment::Line(_)));
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

        assert_eq!(path.geometry.num_segments(), 2);
        assert_eq!(path.vertex_is_dash_point, [true, false, true]);
    }

    #[test]
    fn open_path_preserves_both_endpoint_dash_flags() {
        let path = bezier_from_raw_coords(&[
            (Coord { x: 0, y: 0 }, COORD_FLAG_DASH_POINT),
            (Coord { x: 1_000, y: 0 }, COORD_FLAG_DASH_POINT),
            (Coord { x: 2_000, y: 0 }, COORD_FLAG_DASH_POINT),
            (Coord { x: 3_000, y: 0 }, COORD_FLAG_DASH_POINT),
        ]);

        assert_eq!(path.geometry.num_segments(), 3);
        assert_eq!(path.vertex_is_dash_point, [true, true, true, true]);
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
}
