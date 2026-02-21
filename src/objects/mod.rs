mod area_object;
mod line_object;
mod point_object;
mod text_object;

mod map_object;

use geo_types::Coord;
use quick_xml::{Reader, events::Event};
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
