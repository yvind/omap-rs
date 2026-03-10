mod area_object;
mod line_object;
mod point_object;
mod text_object;

mod map_object;

use geo_types::Coord;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};
use std::collections::HashMap;

pub use area_object::AreaObject;
pub use line_object::LineObject;
pub use point_object::PointObject;
pub use text_object::{HorizontalAlign, TextGeometry, TextObject, VerticalAlign};

pub use map_object::MapObject;

use super::{Error, Result};

const PARSE_BEZIER_ERROR: f64 = 0.1;

type MapCoord = (Coord<i32>, u8);

fn parse_tags<R: std::io::BufRead>(reader: &mut Reader<R>) -> Result<HashMap<String, String>> {
    let mut buf = Vec::new();

    let mut tags = HashMap::new();

    let mut key = None;
    let mut value = None;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(bytes_start) => {
                if matches!(bytes_start.local_name().as_ref(), b"t") {
                    for attr in bytes_start.attributes().filter_map(std::result::Result::ok) {
                        key = attr
                            .decode_and_unescape_value(bytes_start.decoder())
                            .ok()
                            .map(|s| s.to_string())
                    }
                }
            }
            Event::End(bytes_end) => match bytes_end.local_name().as_ref() {
                b"tags" => break,
                b"t" => {
                    if let Some(k) = key
                        && let Some(v) = value
                    {
                        let _ = tags.insert(k, v);
                    }
                    key = None;
                    value = None;
                }
                _ => (),
            },
            Event::Text(bytes_text) => {
                value = bytes_text.xml11_content().ok().map(|s| s.to_string())
            }
            Event::Eof => {
                return Err(Error::ParseOmapFileError(
                    "Unexpected EOF when parsing Tags".to_string(),
                ));
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
            BytesStart::new("t").with_attributes([("key", key.as_str())]),
        ))?;
        writer.write_event(Event::Text(BytesText::new(value)))?;
        writer.write_event(Event::End(BytesEnd::new("t")))?;
    }
    writer.write_event(Event::End(BytesEnd::new("tags")))?;
    Ok(())
}

/// Write raw map coords as the content of a `<coords>` element
fn write_raw_coords<W: std::io::Write>(writer: &mut Writer<W>, coords: &[MapCoord]) -> Result<()> {
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
