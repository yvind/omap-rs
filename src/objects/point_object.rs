use std::{cell::RefCell, collections::HashMap, rc::Weak};

use geo_types::{Coord, Point};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use crate::{
    Error, Result,
    symbols::PointSymbol,
    utils::{from_file_coords, to_file_coords},
};

/// A point object placed at a single location on the map.
#[derive(Debug, Clone)]
pub struct PointObject {
    /// The tags associated with the object
    pub tags: HashMap<String, String>,
    /// Rotation of the symbol in radians.
    pub rotation: f64,
    /// Weak reference to the point symbol used to render this object.
    pub symbol: Weak<RefCell<PointSymbol>>,
    /// The point coordinates in mm on the map.
    pub geometry: Point,
}

impl PointObject {
    /// Write just the inner content (coords) — called from MapObject::write
    pub(super) fn write_content<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        let file_coord = to_file_coords(self.geometry.0)?;
        let bs = BytesStart::new("coords").with_attributes([("count", "1")]);
        writer.write_event(Event::Start(bs))?;
        writer.write_event(Event::Text(BytesText::new(&format!(
            "{} {};",
            file_coord.x, file_coord.y
        ))))?;
        writer.write_event(Event::End(BytesEnd::new("coords")))?;
        Ok(())
    }

    /// Write a full `<object>...</object>` element — used for point symbol elements
    pub fn write_as_element<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        let mut bs = BytesStart::new("object").with_attributes([("type", "0")]);
        if self.rotation.abs() > f64::EPSILON {
            bs.push_attribute(("rotation", self.rotation.to_string().as_str()));
        }
        writer.write_event(Event::Start(bs))?;
        self.write_content(writer)?;
        writer.write_event(Event::End(BytesEnd::new("object")))?;
        Ok(())
    }

    /// Parse a point object. The reader should be positioned right after
    /// the `<coords>` start event. Reads through `</object>`.
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        _coords_element: &BytesStart<'_>,
        rotation: f64,
    ) -> Result<PointObject> {
        let mut point = None;
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"object") {
                        break;
                    }
                }
                Event::Text(bytes_text) => {
                    let raw_xml = String::from_utf8(bytes_text.to_vec())?;

                    for vertex in raw_xml.split_terminator(';') {
                        let mut split = vertex.split_whitespace();

                        let x: i32 = split
                            .next()
                            .ok_or(Error::InvalidCoordinate("No x value".to_string()))?
                            .parse()?;
                        let y: i32 = split
                            .next()
                            .ok_or(Error::InvalidCoordinate("No y value".to_string()))?
                            .parse()?;

                        let coord = from_file_coords(Coord { x, y });
                        point = Some(Point::from(coord));
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in PointObject parsing".to_string(),
                    ));
                }
                _ => (),
            }
        }
        Ok(PointObject {
            tags: HashMap::new(),
            rotation,
            symbol: Weak::new(),
            geometry: point.ok_or(Error::ParseOmapFileError(
                "Could not parse point object".to_string(),
            ))?,
        })
    }
}
