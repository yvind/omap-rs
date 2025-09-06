mod area_object;
mod line_object;
mod point_object;
mod text_object;

mod map_object;

use std::collections::HashMap;

use area_object::AreaObject;
use geo_types::Coord;
use line_object::LineObject;
use point_object::PointObject;
use quick_xml::Reader;
use text_object::TextObject;

pub use map_object::MapObject;

use super::{Error, Result};

#[derive(Debug, Clone)]
pub enum ObjectGeometry {
    Area(AreaObject),
    Line(LineObject),
    Point(PointObject),
    Text(TextObject),
}

impl ObjectGeometry {
    fn type_value(&self) -> u8 {
        match self {
            ObjectGeometry::Point(_) => 0,
            ObjectGeometry::Area(_) => 1,
            ObjectGeometry::Line(_) => 1,
            ObjectGeometry::Text(_) => 4,
        }
    }

    fn get_special_keys(&self) -> Option<String> {
        match self {
            ObjectGeometry::Point(point_object) => point_object.get_special_keys(),
            ObjectGeometry::Text(text_object) => text_object.get_special_keys(),
            _ => None,
        }
    }

    fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        match self {
            ObjectGeometry::Area(area_object) => area_object.write(writer),
            ObjectGeometry::Line(line_object) => line_object.write(writer),
            ObjectGeometry::Point(point_object) => point_object.write(writer),
            ObjectGeometry::Text(text_object) => text_object.write(writer),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PatternRotation {
    rotation: f64,
    coord: Coord,
}

fn parse_tags<R: std::io::BufRead>(reader: &mut Reader<R>) -> Result<HashMap<String, String>> {
    let mut buf = Vec::new();

    let mut tags = HashMap::new();

    let mut key = None;
    let mut value = None;

    loop {
        match reader.read_event_into(&mut buf)? {
            quick_xml::events::Event::Start(bytes_start) => {
                if matches!(bytes_start.local_name().as_ref(), b"t") {
                    for attr in bytes_start.attributes() {
                        let attr = attr?;
                        key = Some(
                            attr.decode_and_unescape_value(bytes_start.decoder())?
                                .to_string(),
                        )
                    }
                }
            }
            quick_xml::events::Event::End(bytes_end) => match bytes_end.local_name().as_ref() {
                b"tags" => break,
                b"t" => {
                    if let Some(k) = key {
                        if let Some(v) = value {
                            let _ = tags.insert(k, v);
                        }
                    }
                    key = None;
                    value = None;
                }
                _ => (),
            },
            quick_xml::events::Event::Text(bytes_text) => {
                value = Some(bytes_text.decode()?.to_string());
            }
            quick_xml::events::Event::Eof => {
                return Err(Error::ParseOmapFileError(
                    "Unexpected EOF when parsing Tags".to_string(),
                ));
            }
            _ => (),
        }
    }
    Ok(tags)
}
