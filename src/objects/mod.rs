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

/// Build the exact mixed line/Bézier representation encoded by Mapper's raw
/// coordinate flags.
///
/// A coordinate with bit 0 set starts a cubic Bézier whose two handles and end
/// point are the following three coordinates. An end point may also start the
/// next curve, so it is deliberately visited again in that case.
fn bezier_from_raw_coords(coords: &[FileCoord]) -> BezierString {
    let mut segments = Vec::new();
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
        }

        if flag & 1 == 1 && index + 3 < coords.len() {
            let handle1 = from_file_coords(coords[index + 1].0);
            let handle2 = from_file_coords(coords[index + 2].0);
            let end_index = index + 3;
            let end = from_file_coords(coords[end_index].0);
            segments.push(BezierSegment::Bezier(BezierCurve::new(
                coord, handle1, handle2, end,
            )));
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

    BezierString::new(segments)
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
