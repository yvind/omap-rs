mod area_object;
mod line_object;
mod point_object;
mod text_object;

mod map_object;

use geo_types::Coord;
use linestring2bezier::{BezierCurve, BezierSegment, BezierString};
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
    utils::{from_file_coords, try_get_attr},
};

use super::{Error, OmapSection, Result};

const PARSE_BEZIER_ERROR: f64 = 0.1;

type FileCoord = (Coord<i32>, u8);

/// A mixed straight/cubic Bézier path with dash-point metadata on its
/// vertices.
///
/// `end_vertex_is_dash_point[i]` describes the end vertex of segment `i`.
/// The vectors therefore always have the same length. For a closed path, the
/// final entry describes the vertex shared by the final segment's end and the
/// first segment's start. For an open path, its unrepresented initial vertex
/// and its final end vertex cannot be dash points in the OMAP format.
#[derive(Debug, Clone)]
pub struct BezierPath {
    /// The straight and cubic segments forming the path.
    pub geometry: BezierString,
    /// Whether each segment's end vertex carries OMAP dash-point flag 32.
    pub end_vertex_is_dash_point: Vec<bool>,
}

/// Build the exact mixed line/Bézier representation encoded by Mapper's raw
/// coordinate flags.
///
/// A coordinate with bit 0 set starts a cubic Bézier whose two handles and end
/// point are the following three coordinates. An end point may also start the
/// next curve, so it is deliberately visited again in that case.
fn bezier_from_raw_coords(coords: &[FileCoord]) -> BezierPath {
    let mut segments = Vec::new();
    let mut end_vertex_is_dash_point = Vec::new();
    let mut previous_anchor = None;
    let mut index = 0;

    while index < coords.len() {
        let (file_coord, flag) = coords[index];
        let coord = from_file_coords(file_coord);

        if let Some((previous_index, previous_coord)) = previous_anchor
            && previous_index != index
        {
            segments.push(BezierSegment::Line(geo_types::Line::new(
                previous_coord,
                coord,
            )));
            end_vertex_is_dash_point.push(flag & 32 == 32);
        }

        if flag & 1 == 1 && index + 3 < coords.len() {
            let handle1 = from_file_coords(coords[index + 1].0);
            let handle2 = from_file_coords(coords[index + 2].0);
            let end_index = index + 3;
            let end = from_file_coords(coords[end_index].0);
            segments.push(BezierSegment::Bezier(BezierCurve::new(
                coord, handle1, handle2, end,
            )));
            end_vertex_is_dash_point.push(coords[end_index].1 & 32 == 32);
            previous_anchor = Some((end_index, end));

            if coords[end_index].1 & 1 == 1 {
                index = end_index;
            } else {
                index = end_index + 1;
            }
        } else {
            previous_anchor = Some((index, coord));
            index += 1;
        }
    }

    // A closed path repeats its first vertex as its final endpoint. Treat flag
    // 32 on either raw representation as metadata for that shared vertex.
    if let (Some((first_coord, first_flag)), Some((last_coord, last_flag))) =
        (coords.first(), coords.last())
        && last_flag & 2 == 2
        && first_coord == last_coord
        && (first_flag & 32 == 32 || last_flag & 32 == 32)
        && let Some(last_is_dash_point) = end_vertex_is_dash_point.last_mut()
    {
        *last_is_dash_point = true;
    }

    debug_assert_eq!(
        segments.len(),
        end_vertex_is_dash_point.len(),
        "every Bézier segment must have one end-vertex dash flag"
    );
    BezierPath {
        geometry: BezierString::new(segments),
        end_vertex_is_dash_point,
    }
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
    use linestring2bezier::BezierSegment;

    use super::bezier_from_raw_coords;

    #[test]
    fn dash_flags_align_with_segment_end_vertices() {
        let path = bezier_from_raw_coords(&[
            (Coord { x: 0, y: 0 }, 0),
            (Coord { x: 1_000, y: 0 }, 33),
            (Coord { x: 1_000, y: 1_000 }, 0),
            (Coord { x: 2_000, y: 1_000 }, 0),
            (Coord { x: 2_000, y: 0 }, 32),
            (Coord { x: 3_000, y: 0 }, 0),
        ]);

        assert_eq!(path.geometry.num_segments(), 3);
        assert_eq!(path.end_vertex_is_dash_point, [true, true, false]);
        assert!(matches!(path.geometry.0[0], BezierSegment::Line(_)));
        assert!(matches!(path.geometry.0[1], BezierSegment::Bezier(_)));
        assert!(matches!(path.geometry.0[2], BezierSegment::Line(_)));
    }

    #[test]
    fn closed_path_combines_first_and_closing_vertex_dash_flags() {
        let path = bezier_from_raw_coords(&[
            (Coord { x: 0, y: 0 }, 32),
            (Coord { x: 1_000, y: 0 }, 0),
            (Coord { x: 0, y: 0 }, 2),
        ]);

        assert_eq!(path.geometry.num_segments(), 2);
        assert_eq!(path.end_vertex_is_dash_point, [false, true]);
    }
}
